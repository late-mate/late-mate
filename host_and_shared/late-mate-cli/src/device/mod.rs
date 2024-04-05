use crate::device::rxtx::{rx_loop, tx_loop, CrcCobsCodec};
use crate::device::serial::find_serial_port;
use crate::nice_hid;
use anyhow::{anyhow, Context};
use futures::StreamExt;
use late_mate_comms::{
    DeviceToHost, HidRequest, HidRequestId, HostToDevice, MeasureFollowup, Measurement, Status,
};
use std::future::Future;
use std::time::Duration;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::{broadcast, mpsc, watch};
use tokio::time::sleep;
use tokio_serial::SerialPortBuilderExt;
use tokio_util::codec::Framed;

mod rxtx;
mod serial;

pub struct Device {
    tx_sender: mpsc::Sender<HostToDevice>,
    // this one is stored in the Device to obtain new broadcast subscriptions
    rx_sender: broadcast::Sender<DeviceToHost>,
    pub max_light_level: u32,
    bg_light: BackgroundLevelMonitor,
    hid_counter: HidRequestId,
}

struct BackgroundLevelMonitor {
    is_active_sender: watch::Sender<bool>,
    sender: broadcast::Sender<u32>,
}

pub async fn bg_request_loop(
    tx_sender: mpsc::Sender<HostToDevice>,
    mut is_active_receiver: watch::Receiver<bool>,
) {
    loop {
        if is_active_receiver.wait_for(|x| *x).await.is_err() {
            // is_active senders are dropped => Device is dropped => no point continuing
            return;
        }
        if tx_sender
            .send(HostToDevice::MeasureBackground { duration_ms: 1300 })
            .await
            .is_err()
        {
            // device tx channel is closed => device is disconnected => no point continuing
            return;
        }
        sleep(Duration::from_millis(1000)).await;
    }
}

pub async fn bg_channel_loop(
    mut rx_reciever: broadcast::Receiver<DeviceToHost>,
    mut is_active_receiver: watch::Receiver<bool>,
    bg_sender: broadcast::Sender<u32>,
) {
    loop {
        if is_active_receiver.wait_for(|x| *x).await.is_err() {
            // is_active senders are dropped => Device is dropped => no point continuing
            return;
        }

        match rx_reciever.recv().await {
            Ok(DeviceToHost::CurrentLightLevel(level)) => match bg_sender.send(level) {
                Ok(_) => (),
                Err(_) => {
                    // all receivers were dropped, but they might rejoin later
                    // todo: maybe I should stop here?
                    continue;
                }
            },
            Ok(_) => (),
            Err(RecvError::Lagged(_)) => (),
            // device rx channel is closed => device is disconnected => give up
            Err(RecvError::Closed) => return,
        }
    }
}

impl Device {
    pub async fn init() -> anyhow::Result<Self> {
        let serial_port_info = find_serial_port()?;
        let serial_stream = tokio_serial::new(serial_port_info.port_name, 115200)
            .open_native_async()
            .context("Serial port opening failure")?;

        let serial_framed = Framed::new(serial_stream, CrcCobsCodec::new());
        let (serial_framed_tx, serial_framed_rx) = serial_framed.split();

        let (tx_sender, tx_receiver) = mpsc::channel(4);
        // todo: that 4k is a pisstake, figure out what to do about Lagged
        let (rx_sender, rx_receiver) = broadcast::channel(4000);

        // todo: handle those handles
        let _tx_loop_handle = tokio::spawn(tx_loop(serial_framed_tx, tx_receiver));
        let _rx_loop_handle = tokio::spawn(rx_loop(serial_framed_rx, rx_sender.clone()));

        // todo: this is shite
        tokio::spawn(async move {
            tokio::select! {
                msg = _tx_loop_handle => dbg!(msg),
                msg = _rx_loop_handle => dbg!(msg)
            }
        });

        let (bg_is_active_sender, bg_is_active_receiver) = watch::channel(false);
        let (bg_sender, _bg_receiver) = broadcast::channel(1);

        // handle handles?
        tokio::spawn(bg_request_loop(
            tx_sender.clone(),
            bg_is_active_receiver.clone(),
        ));
        tokio::spawn(bg_channel_loop(
            rx_receiver,
            bg_is_active_receiver,
            bg_sender.clone(),
        ));

        let mut tmp_self = Self {
            tx_sender,
            rx_sender,
            max_light_level: 0,
            bg_light: BackgroundLevelMonitor {
                is_active_sender: bg_is_active_sender,
                sender: bg_sender,
            },
            // 1 is more special than 0
            hid_counter: 1,
        };
        let device_status = tmp_self.get_status().await?;
        tmp_self.max_light_level = device_status.max_light_level;

        Ok(tmp_self)
    }

    pub async fn get_status(&mut self) -> anyhow::Result<Status> {
        self.one_off(
            HostToDevice::GetStatus,
            Some(|msg| match msg {
                DeviceToHost::Status(s) => Some(s),
                _ => None,
            }),
        )
        .await
    }

    pub async fn send_hid_report(&mut self, report: &nice_hid::HidReport) -> anyhow::Result<()> {
        let hid_request = HidRequest {
            id: self.new_hid_request_id(),
            report: report.into(),
        };
        self.one_off(
            HostToDevice::SendHidReport(hid_request),
            Some(|msg| match msg {
                DeviceToHost::HidReportSent(id) if id == hid_request.id => Some(()),
                _ => None,
            }),
        )
        .await
    }

    // todo: detect drop or stop subscribing in some way?
    pub fn subscribe_to_background(&mut self) -> broadcast::Receiver<u32> {
        self.bg_light
            .is_active_sender
            .send(true)
            .expect("is_active channel must be opened here");
        self.bg_light.sender.subscribe()
    }

    pub async fn measure(
        &mut self,
        duration_ms: u16,
        start: &nice_hid::HidReport,
        followup: Option<(u16, &nice_hid::HidReport)>,
    ) -> anyhow::Result<Vec<Measurement>> {
        assert!(duration_ms < 1000);
        let command = HostToDevice::Measure {
            duration_ms,
            start: HidRequest {
                id: self.new_hid_request_id(),
                report: start.into(),
            },
            followup: followup.map(|(after_ms, report)| MeasureFollowup {
                after_ms,
                hid_request: HidRequest {
                    id: self.new_hid_request_id(),
                    report: report.into(),
                },
            }),
        };

        let req_future = async {
            self.tx_sender
                .send(command)
                .await
                .context("Device TX channel was unexpectedly closed")
        };

        let mut rx_receiver = self.rx_sender.subscribe();
        let resp_future = async {
            let mut maybe_total: Option<u16> = None;
            let mut next_idx = 0;
            let mut measurements: Vec<Measurement> = Vec::new();

            loop {
                match rx_receiver.recv().await {
                    Ok(DeviceToHost::BufferedMeasurement {
                        measurement,
                        idx,
                        total,
                    }) => {
                        assert_eq!(idx, next_idx, "Unexpected buffered measurement idx");
                        assert!(idx < total, "Unexpected buffered measurement idx larger than total, {idx} > {total}");
                        match maybe_total {
                            None => {
                                maybe_total = Some(total);
                                measurements.reserve(total as usize);
                            }
                            Some(known_total) => {
                                assert_eq!(
                                    known_total, total,
                                    "Unexpected change of total number of measurements"
                                );
                            }
                        }
                        measurements.push(measurement);
                        next_idx += 1;
                        if idx == total - 1 {
                            return Ok(measurements);
                        }
                    }
                    Ok(_) => continue,
                    Err(RecvError::Lagged(_)) => {
                        // todo: this is a problem, the results are missed here
                        println!("lagged");
                        continue;
                    }
                    Err(RecvError::Closed) => {
                        return Err(anyhow!("Device RX channel was unexpectedly closed"))
                    }
                }
            }
        };

        match tokio::try_join!(
            req_future,
            flat_timeout(
                Duration::from_millis(duration_ms as u64 * 2),
                anyhow!("Timeout while waiting for measurements"),
                resp_future
            )
        ) {
            Ok((_, measurements)) => Ok(measurements),
            // have to rewrap to change the type ('Ok' branches are different)
            Err(e) => Err(e),
        }
    }

    async fn one_off<T>(
        &mut self,
        command: HostToDevice,
        resp_mapper: Option<impl Fn(DeviceToHost) -> Option<T>>,
    ) -> anyhow::Result<T> {
        let response_timeout = Duration::from_secs(3);

        let req_future = async {
            self.tx_sender
                .send(command)
                .await
                .context("Device TX channel was unexpectedly closed")
        };
        // I believe this should ~guarantee that we won't miss the response
        let mut rx_receiver = self.rx_sender.subscribe();
        let resp_future = async {
            loop {
                match rx_receiver.recv().await {
                    Ok(msg) => {
                        if let Some(mapper) = &resp_mapper {
                            match mapper(msg) {
                                Some(result) => return Ok(result),
                                None => continue,
                            }
                        }
                    }
                    Err(RecvError::Lagged(_)) => continue,
                    Err(RecvError::Closed) => {
                        return Err(anyhow!("Device RX channel was unexpectedly closed"))
                    }
                }
            }
        };

        match tokio::try_join!(
            req_future,
            flat_timeout(
                response_timeout,
                anyhow!("Timeout while waiting for response to {command:?}"),
                resp_future
            )
        ) {
            Ok((_, status)) => Ok(status),
            // have to rewrap to change the type ('Ok' branches are different)
            Err(e) => Err(e),
        }
    }

    fn new_hid_request_id(&mut self) -> HidRequestId {
        let id = self.hid_counter;
        self.hid_counter += 1;
        id
    }
}

async fn flat_timeout<F: Future<Output = anyhow::Result<R>>, R>(
    timeout_duration: Duration,
    timeout_error: anyhow::Error,
    future: F,
) -> anyhow::Result<R> {
    match tokio::time::timeout(timeout_duration, future).await {
        Ok(Ok(x)) => Ok(x),
        Ok(Err(e)) => Err(e),
        Err(_) => Err(timeout_error),
    }
}
