use crate::agents::dispatcher::DispatcherHandle;
use crate::agents::usb_tx::UsbTxHandle;
use crate::agents::{agent_watcher, dispatcher, usb_rx, usb_tx};
use crate::usb::UsbDevice;
use late_mate_shared::comms::device_to_host::DeviceToHost;
use late_mate_shared::comms::hid::HidRequest;
use late_mate_shared::comms::host_to_device::HostToDevice;
use late_mate_shared::MAX_SCENARIO_DURATION_MS;
use std::time::Duration;
use tokio::task::JoinSet;
use tokio::time::timeout;

mod agents;
pub mod hid;
mod usb;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Late Mate encountered an error. Check the device log to find more details")]
    OnDeviceError,
    #[error("Late Mate disconnected")]
    Disconnected,
    #[error("USB error while {0}")]
    UsbError(&'static str, #[source] nusb::Error),
    #[error("USB transfer error while {0}")]
    UsbTransferError(&'static str, #[source] nusb::transfer::TransferError),
    #[error("Timeout while trying to execute an operation")]
    Timeout,
}

type ResponseResult = Result<Option<DeviceToHost>, Error>;

#[derive(Debug, Clone)]
pub struct Status {
    pub hardware_version: String,
    pub firmware_version: String,
    pub serial_number: String,
    pub max_light_level: u32,
}

#[derive(Debug, thiserror::Error)]
pub enum StatusConversionError {
    #[error("Expected a Status message")]
    NotStatus,
}

impl TryFrom<DeviceToHost> for Status {
    type Error = StatusConversionError;

    fn try_from(value: DeviceToHost) -> Result<Self, Self::Error> {
        match value {
            DeviceToHost::Status {
                version,
                max_light_level,
                serial_number,
            } => Ok(Self {
                hardware_version: version.hardware.to_string(),
                firmware_version: version.firmware.to_string(),
                serial_number: serial_number
                    .into_iter()
                    .map(|b| format!("{:02X}", b).to_string())
                    .collect(),
                max_light_level,
            }),
            _ => Err(Self::Error::NotStatus),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Device {
    usb_tx: UsbTxHandle,
    dispatcher: DispatcherHandle,

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

impl Device {
    pub async fn init() -> Result<Self, Error> {
        let usb_device = UsbDevice::acquire().await?;
        let (in_queue, out_queue) = usb_device.into_queues()?;

        let mut agent_set: JoinSet<()> = JoinSet::new();

        let usb_rx = usb_rx::start(&mut agent_set, in_queue);
        let usb_tx = usb_tx::start(&mut agent_set, out_queue);
        let dispatcher = dispatcher::start(&mut agent_set, usb_rx);
        agent_watcher::start(agent_set);

        let mut self_ = Self {
            usb_tx,
            dispatcher,
            max_light_level: 0,
        };

        let device_status = self_.get_status().await?;
        self_.max_light_level = device_status.max_light_level;

        Ok(self_)
    }

    async fn one_off(&self, request: HostToDevice) -> ResponseResult {
        let (mut receiver, envelope) = self.dispatcher.register_request(request).await;

        self.usb_tx
            .send(envelope)
            .await
            .expect("Device should be ready");

        timeout(
            Duration::from_millis(MAX_SCENARIO_DURATION_MS + 1000),
            receiver.recv(),
        )
        .await
        .map_err(|_| Error::Timeout)?
        .expect("Pending responses should not be dropped")
    }

    pub async fn get_status(&self) -> Result<Status, Error> {
        let response = self
            .one_off(HostToDevice::GetStatus)
            .await?
            .expect("Status response should be present");

        Ok(Status::try_from(response).expect("Status response should be of correct type"))
    }

    pub async fn reset_to_firmware_update(&self) -> Result<(), Error> {
        let response = self.one_off(HostToDevice::ResetToFirmwareUpdate).await?;
        assert!(
            response.is_none(),
            "ResetToFirmwareUpdate shouldn't receive a response"
        );

        Ok(())
    }

    pub async fn send_hid_report(&mut self, report: &hid::HidReport) -> Result<(), Error> {
        let hid_request = HidRequest {
            id: 0,
            report: report.into(),
        };

        let response = self
            .one_off(HostToDevice::SendHidReport(hid_request))
            .await?;
        assert!(response.is_none(), "HidReport shouldn't receive a response");

        Ok(())
    }
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
}
