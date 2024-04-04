use crate::device::rxtx::{rx_loop, tx_loop, CrcCobsCodec};
use crate::device::serial::find_serial_port;
use anyhow::{anyhow, Context};
use futures::StreamExt;
use late_mate_comms::{DeviceToHost, HostToDevice, Status};
use std::time::Duration;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::{broadcast, mpsc, watch};
use tokio::time::{sleep, timeout};
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
        let (rx_sender, rx_receiver) = broadcast::channel(1);

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

    // todo: detect drop or stop subscribing in some way?
    pub fn subscribe_to_background(&mut self) -> broadcast::Receiver<u32> {
        self.bg_light
            .is_active_sender
            .send(true)
            .expect("is_active channel must be opened here");
        self.bg_light.sender.subscribe()
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
        let resp_future = async {
            let mut rx_receiver = self.rx_sender.subscribe();
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
        let timely_resp_future = async move {
            match timeout(response_timeout, resp_future).await {
                Ok(Ok(x)) => Ok(x),
                Ok(Err(e)) => Err(e),
                Err(_) => Err(anyhow!("Timeout while waiting for response to {command:?}")),
            }
        };

        match tokio::try_join!(req_future, timely_resp_future) {
            Ok((_, status)) => Ok(status),
            // have to rewrap to change the type ('Ok' branches are different)
            Err(e) => Err(e),
        }
    }
}
