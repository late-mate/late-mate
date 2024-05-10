use crate::device::dispatcher::Dispatcher;
use crate::device::rxtx::{rx, tx};
use crate::nice_hid;
use anyhow::{anyhow, Context};
use futures::StreamExt;
use late_mate_shared::comms::device_to_host::{DeviceToHost, Measurement};
use late_mate_shared::comms::hid::{HidRequest, HidRequestId};
use late_mate_shared::comms::host_to_device::{HostToDevice, RequestId};
use late_mate_shared::comms::{device_to_host, host_to_device, usb_interface};
use late_mate_shared::{USB_PID, USB_VID};
use std::collections::HashMap;
use std::future::Future;
use std::mem;
use std::ops::DerefMut;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::{broadcast, mpsc, watch, Mutex as TokioMutex};
use tokio::task::JoinSet;
use tokio::time::{sleep, Instant};
use tokio_serial::SerialPortBuilderExt;
use tokio_util::codec::Framed;

mod dispatcher;
mod rxtx;

#[derive(Debug, thiserror::Error)]
pub enum DeviceError {
    #[error("Late Mate encountered an error. Check the device log to find more details")]
    OnDeviceError,
    #[error("Late Mate disconnected")]
    Disconnected,
}

#[derive(Debug, Clone)]
pub struct Status {
    hardware_version: String,
    firmware_version: String,
    serial_number: String,
    max_light_level: u32,
}

#[derive(Debug, thiserror::Error)]
pub enum StatusError {
    #[error("Unexpected DeviceToHost message")]
    UnexpectedDeviceToHost,
}

impl TryFrom<DeviceToHost> for Status {
    type Error = StatusError;

    fn try_from(value: DeviceToHost) -> Result<Self, Self::Error> {
        match value {
            DeviceToHost::Status {
                version,
                max_light_level,
                serial_number,
            } => Ok(Self {
                hardware_version: "hello".into(),
                firmware_version: "world".into(),
                serial_number: "foobar".into(),
                max_light_level,
            }),
            _ => Err(Self::Error::UnexpectedDeviceToHost),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Device {
    // bg_light: BackgroundLevelMonitor,
    // hid_counter: HidRequestId,
    tx_sender: mpsc::Sender<host_to_device::Envelope>,
    dispatcher: Arc<TokioMutex<Dispatcher>>,

    pub max_light_level: u32,
}

// struct BackgroundLevelMonitor {
//     is_active_sender: watch::Sender<bool>,
//     sender: broadcast::Sender<u32>,
// }

// pub async fn bg_request_loop(
//     tx_sender: mpsc::Sender<HostToDevice>,
//     mut is_active_receiver: watch::Receiver<bool>,
// ) -> anyhow::Result<()> {
//     loop {
//         is_active_receiver
//             .wait_for(|x| *x)
//             .await
//             .context("is_active_sender is dropped => device is dropped => exit")?;
//         tx_sender
//             .send(HostToDevice::StreamLightLevel { duration_ms: 1300 })
//             .await
//             .context("device tx channel is closed => device is disconnected => exit")?;
//         sleep(Duration::from_millis(1000)).await;
//     }
// }
//
// pub async fn bg_channel_loop(
//     mut rx_reciever: broadcast::Receiver<DeviceToHost>,
//     mut is_active_receiver: watch::Receiver<bool>,
//     bg_sender: broadcast::Sender<u32>,
// ) -> anyhow::Result<()> {
//     loop {
//         is_active_receiver
//             .wait_for(|x| *x)
//             .await
//             .context("is_active_sender is dropped => device is dropped => exit")?;
//
//         match rx_reciever.recv().await {
//             Ok(DeviceToHost::CurrentLightLevel(level)) => match bg_sender.send(level) {
//                 Ok(_) => (),
//                 Err(_) => {
//                     // all receivers were dropped, but they might rejoin later
//                     // todo: maybe I should stop here?
//                     continue;
//                 }
//             },
//             Ok(_) => (),
//             // todo: lagged
//             Err(RecvError::Lagged(_)) => (),
//             Err(RecvError::Closed) => {
//                 return Err(anyhow!(
//                     "device rx channel is closed => device is disconnected => exit"
//                 ))
//             }
//         }
//     }
// }

async fn acquire_device() -> anyhow::Result<nusb::Device> {
    let mut first_attempt = true;
    loop {
        let connected_devices = nusb::list_devices()
            .context("USB error while listing devices")?
            .filter(|di| di.vendor_id() == USB_VID && di.product_id() == USB_PID)
            .collect::<Vec<_>>();

        if connected_devices.is_empty() {
            if first_attempt {
                eprintln!("No Late Mate detected, waiting for the device to be connected");
                first_attempt = false;
            }
            sleep(Duration::from_secs(1)).await;
            continue;
        }

        let first = &connected_devices[0];

        let n = connected_devices.len();
        if n > 1 {
            eprintln!(
                "More than one Late Mate detected ({}), using {}",
                n,
                first
                    .serial_number()
                    .expect("Late Mate devices must have serial numbers")
            );
        }

        return first.open().context("USB error while opening the device");
    }
}

impl Device {
    pub async fn init() -> anyhow::Result<(Self, JoinSet<anyhow::Result<()>>)> {
        let device = acquire_device().await?;
        let interface = device
            .claim_interface(usb_interface::NUMBER)
            .context("USB error while claiming the interface")?;

        let mut subtasks = JoinSet::new();

        let in_queue = interface.bulk_in_queue(usb_interface::ENDPOINT_INDEX | 0x80);
        let out_queue = interface.bulk_out_queue(usb_interface::ENDPOINT_INDEX);

        let rx_receiver = rx::start(&mut subtasks, in_queue);
        let tx_sender = tx::start(&mut subtasks, out_queue);

        let dispatcher = dispatcher::start(&mut subtasks, rx_receiver);

        let mut tmp_self = Self {
            tx_sender,
            dispatcher,
            max_light_level: 0,
        };

        let device_status = tmp_self.get_status().await?;
        tmp_self.max_light_level = device_status.max_light_level;

        Ok((tmp_self, subtasks))
    }

    pub async fn get_status(&self) -> anyhow::Result<Status> {
        let response = self
            .one_off(HostToDevice::GetStatus)
            .await?
            .expect("The response should be present");

        Ok(Status::try_from(response).expect("The response should be of correct type"))
    }

    pub async fn reset_to_firmware_update(&self) -> anyhow::Result<()> {
        let response = self.one_off(HostToDevice::ResetToFirmwareUpdate).await?;
        assert!(response.is_none());

        Ok(())
    }

    async fn one_off(&self, request: HostToDevice) -> anyhow::Result<Option<DeviceToHost>> {
        let mut receiver = self
            .make_request(request)
            .await
            .context("USB device error while sending the command")?;
        let response = receiver
            .recv()
            .await
            .expect("Pending channels shouldn't be closed")
            .context("USB device error while receiving the response")?;
        Ok(response)
    }

    // pub async fn send_hid_report(&mut self, report: &nice_hid::HidReport) -> anyhow::Result<()> {
    //     let hid_request = HidRequest {
    //         id: self.new_hid_request_id(),
    //         report: report.into(),
    //     };
    //     self.one_off(
    //         HostToDevice::SendHidReport(hid_request),
    //         Some(Box::new(move |msg| match msg {
    //             DeviceToHost::HidReportSent(id) if id == hid_request.id => Some(()),
    //             _ => None,
    //         })),
    //     )
    //     .await
    // }
    //
    // // todo: detect drop or stop subscribing in some way?
    // pub fn subscribe_to_background(&mut self) -> broadcast::Receiver<u32> {
    //     self.bg_light.sender.subscribe()
    // }
    //
    // // todo: this doesn't actually help to handle multiple pages open at once,
    // //       rework this completely
    // pub fn background_enable(&mut self) {
    //     self.bg_light
    //         .is_active_sender
    //         .send(true)
    //         .expect("is_active channel must be opened here");
    // }
    //
    // pub fn background_disable(&mut self) {
    //     self.bg_light
    //         .is_active_sender
    //         .send(false)
    //         .expect("is_active channel must be opened here");
    // }
    //
    // pub async fn measure(
    //     &mut self,
    //     duration_ms: u16,
    //     start: &nice_hid::HidReport,
    //     followup: Option<(u16, nice_hid::HidReport)>,
    // ) -> anyhow::Result<Vec<Measurement>> {
    //     assert!(duration_ms < 1000);
    //     let command = HostToDevice::Measure {
    //         duration_ms,
    //         start: HidRequest {
    //             id: self.new_hid_request_id(),
    //             report: start.into(),
    //         },
    //         followup: followup.map(|(after_ms, report)| MeasureFollowup {
    //             after_ms,
    //             hid_request: HidRequest {
    //                 id: self.new_hid_request_id(),
    //                 report: (&report).into(),
    //             },
    //         }),
    //     };
    //
    //     let req_future = async {
    //         self.tx_sender
    //             .send(command)
    //             .await
    //             .context("Device TX channel was unexpectedly closed")
    //     };
    //
    //     let mut rx_receiver = self.rx_sender.subscribe();
    //     let resp_future = async {
    //         let mut maybe_total: Option<u16> = None;
    //         let mut next_idx = 0;
    //         let mut measurements: Vec<Measurement> = Vec::new();
    //
    //         loop {
    //             match rx_receiver.recv().await {
    //                 Ok(DeviceToHost::BufferedMeasurement {
    //                     measurement,
    //                     idx,
    //                     total,
    //                 }) => {
    //                     assert_eq!(idx, next_idx, "Unexpected buffered measurement idx");
    //                     assert!(idx < total, "Unexpected buffered measurement idx larger than total, {idx} > {total}");
    //                     match maybe_total {
    //                         None => {
    //                             maybe_total = Some(total);
    //                             measurements.reserve(total as usize);
    //                         }
    //                         Some(known_total) => {
    //                             assert_eq!(
    //                                 known_total, total,
    //                                 "Unexpected change of total number of measurements"
    //                             );
    //                         }
    //                     }
    //                     measurements.push(measurement);
    //                     next_idx += 1;
    //                     if idx == total - 1 {
    //                         return Ok(measurements);
    //                     }
    //                 }
    //                 Ok(_) => continue,
    //                 Err(RecvError::Lagged(_)) => {
    //                     // todo: this is a problem, the results are missed here
    //                     println!("lagged");
    //                     continue;
    //                 }
    //                 Err(RecvError::Closed) => {
    //                     return Err(anyhow!("Device RX channel was unexpectedly closed"))
    //                 }
    //             }
    //         }
    //     };
    //
    //     match tokio::try_join!(
    //         req_future,
    //         flat_timeout(
    //             Duration::from_millis(duration_ms as u64 * 2),
    //             anyhow!("Timeout while waiting for measurements"),
    //             resp_future
    //         )
    //     ) {
    //         Ok((_, measurements)) => Ok(measurements),
    //         // have to rewrap to change the type ('Ok' branches are different)
    //         Err(e) => Err(e),
    //     }
    // }
    //
    // pub async fn reset_to_firmware_update(&mut self) -> anyhow::Result<Status> {
    //     self.one_off(HostToDevice::ResetToFirmwareUpdate, None)
    //         .await
    // }
    //
    async fn make_request(
        &self,
        request: HostToDevice,
    ) -> anyhow::Result<mpsc::Receiver<Result<Option<DeviceToHost>, DeviceError>>> {
        let (receiver, envelope) = self.dispatcher.lock().await.register_request(request);

        self.tx_sender
            .send(envelope)
            .await
            .map_err(|_| DeviceError::Disconnected)?;

        Ok(receiver)
    }
    //
    // async fn one_off<T>(
    //     &mut self,
    //     command: HostToDevice,
    //     resp_mapper: Option<Box<dyn Fn(DeviceToHost) -> Option<T> + Send + Sync>>,
    // ) -> anyhow::Result<T> {
    //     let response_timeout = Duration::from_secs(3);
    //
    //     let req_future = async {
    //         self.tx_sender
    //             // todo: this clone?
    //             .send(command.clone())
    //             .await
    //             .context("Device TX channel was unexpectedly closed")
    //     };
    //     // I believe this should ~guarantee that we won't miss the response
    //     let mut rx_receiver = self.rx_sender.subscribe();
    //     let resp_future = async {
    //         // todo: what am I doing if I don't expect a reply (eg reset to firmware update)?
    //         loop {
    //             match rx_receiver.recv().await {
    //                 Ok(msg) => {
    //                     if let Some(mapper) = &resp_mapper {
    //                         match mapper(msg) {
    //                             Some(result) => return Ok(result),
    //                             None => continue,
    //                         }
    //                     }
    //                 }
    //                 Err(RecvError::Lagged(_)) => continue,
    //                 Err(RecvError::Closed) => {
    //                     return Err(anyhow!("Device RX channel was unexpectedly closed"))
    //                 }
    //             }
    //         }
    //     };
    //
    //     match tokio::try_join!(
    //         req_future,
    //         flat_timeout(
    //             response_timeout,
    //             anyhow!("Timeout while waiting for response to {command:?}"),
    //             resp_future
    //         )
    //     ) {
    //         Ok((_, status)) => Ok(status),
    //         // have to rewrap to change the type ('Ok' branches are different)
    //         Err(e) => Err(e),
    //     }
    // }
    //
    // fn new_hid_request_id(&mut self) -> HidRequestId {
    //     let id = self.hid_counter;
    //     self.hid_counter += 1;
    //     id
    // }
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
