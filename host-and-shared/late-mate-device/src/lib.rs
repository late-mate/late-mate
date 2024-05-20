use crate::agents::dispatcher::DispatcherHandle;
use crate::agents::usb_tx::UsbTxHandle;
use crate::agents::{agent_watcher, dispatcher, usb_rx, usb_tx};
use crate::scenario::{to_device_scenario, Moment, Recording, Scenario};
use crate::usb::UsbDevice;
use late_mate_shared::comms;
use late_mate_shared::comms::device_to_host;
use late_mate_shared::comms::host_to_device;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio::time::{sleep, timeout};
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::Stream;

mod agents;
pub mod hid;
pub mod scenario;
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
    #[error("Timeout while sending the request")]
    RequestTimeout,
    #[error("Timeout while waiting for the response")]
    ResponseTimeout,
}

type ResponseResult = Result<Option<device_to_host::Message>, Error>;

#[derive(Debug, Clone)]
pub struct Status {
    pub hardware_version: String,
    pub firmware_version: String,
    pub serial_number: String,
    pub max_light_level: u32,
    pub last_panic_message: Option<String>,
}

impl Status {
    pub fn from_device(
        device_to_host::Status {
            version,
            max_light_level,
            serial_number,
        }: device_to_host::Status,
        last_panic_message: Option<String>,
    ) -> Self {
        Self {
            hardware_version: version.hardware.to_string(),
            firmware_version: version.firmware.to_string(),
            serial_number: serial_number
                .into_iter()
                .map(|b| format!("{:02X}", b).to_string())
                .collect(),
            max_light_level,
            last_panic_message,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Device {
    usb_tx: UsbTxHandle,
    dispatcher: DispatcherHandle,

    pub max_light_level: u32,
    pub last_panic_message: Option<String>,
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
        tracing::debug!("Acquiring the device");
        let usb_device = UsbDevice::acquire().await?;

        let (in_queue, out_queue) = usb_device.into_queues()?;

        tracing::debug!("Starting the agents");
        let mut agent_set: JoinSet<()> = JoinSet::new();
        let usb_rx = usb_rx::start(&mut agent_set, in_queue);
        let usb_tx = usb_tx::start(&mut agent_set, out_queue);
        let dispatcher = dispatcher::start(&mut agent_set, usb_rx);
        agent_watcher::start(agent_set);

        let mut self_ = Self {
            usb_tx,
            dispatcher,
            max_light_level: 0,
            last_panic_message: None,
        };

        // todo: remove this match after Dan Luu updates his device; I only need this because
        //       I shortened the version string, so the driver hangs while trying to initialise
        tracing::debug!("Requesting the initial device status");
        let max_light_level = match self_.get_status().await {
            Err(Error::ResponseTimeout) => 0,
            Err(e) => return Err(e),
            Ok(status) => status.max_light_level,
        };
        self_.max_light_level = max_light_level;

        tracing::debug!("The device is now successfully initialised");
        Ok(self_)
    }

    async fn make_request(
        &self,
        request: host_to_device::Message,
    ) -> Result<mpsc::Receiver<ResponseResult>, Error> {
        let (receiver, envelope) = self.dispatcher.register_request(request).await;

        self.usb_tx.send(envelope).await?;

        Ok(receiver)
    }

    async fn one_off(&self, request: host_to_device::Message) -> ResponseResult {
        let mut response_receiver = self.make_request(request).await?;

        let result = timeout(usb::OPERATION_TIMEOUT, response_receiver.recv()).await;

        result
            .map_err(|_| Error::ResponseTimeout)?
            .expect("Pending response channel should not be dropped")
    }

    pub async fn get_status(&mut self) -> Result<Status, Error> {
        let mut response_receiver = self
            .make_request(host_to_device::Message::GetStatus)
            .await?;

        let mut panic_bytes = Vec::new();
        loop {
            let timely_response = timeout(usb::OPERATION_TIMEOUT, response_receiver.recv())
                .await
                .map_err(|_| Error::ResponseTimeout)?;
            let maybe_message =
                timely_response.expect("Pending response channel should not be dropped")?;
            let message = maybe_message.expect("Status response should be present");
            match message {
                device_to_host::Message::PanicChunk(chunk) => {
                    panic_bytes.extend(chunk);
                }
                device_to_host::Message::Status(device_status) => {
                    let last_panic_message = if panic_bytes.is_empty() {
                        None
                    } else {
                        Some(String::from_utf8_lossy(&panic_bytes).to_string())
                    };
                    if last_panic_message.is_some() {
                        self.last_panic_message.clone_from(&last_panic_message);
                    }
                    return Ok(Status::from_device(device_status, last_panic_message));
                }
                _ => unreachable!("Status response should be of correct type"),
            }
        }
    }

    pub async fn reset_to_firmware_update(&self) -> Result<(), Error> {
        let response = self
            .one_off(host_to_device::Message::ResetToFirmwareUpdate)
            .await?;
        assert!(
            response.is_none(),
            "ResetToFirmwareUpdate shouldn't receive a response"
        );

        Ok(())
    }

    pub async fn send_hid_report(&self, report: &hid::HidReport) -> Result<(), Error> {
        let hid_request = comms::hid::HidRequest {
            id: 0,
            report: report.into(),
        };

        let response = self
            .one_off(host_to_device::Message::SendHidReport(hid_request))
            .await?;
        assert!(response.is_none(), "HidReport shouldn't receive a response");

        Ok(())
    }

    async fn assemble_timeline(
        mut receiver: mpsc::Receiver<ResponseResult>,
        hid_index: &[hid::HidReport],
    ) -> Result<Vec<Moment>, Error> {
        let mut timeline = Vec::new();
        let mut reported_total = 0;

        loop {
            let message = receiver
                .recv()
                .await
                .expect("Pending response channel should not be dropped")?;
            match message {
                Some(device_to_host::Message::BufferedMoment(buffered_moment)) => {
                    assert_eq!(
                        buffered_moment.idx as usize,
                        timeline.len(),
                        "Scenario results must arrive in order"
                    );
                    timeline.push(Moment::from_device(buffered_moment, hid_index));
                    reported_total = buffered_moment.total as usize;
                }
                Some(_) => unreachable!("Scenario results must not interleave with other messages"),
                None => break,
            }
        }
        assert_eq!(
            timeline.len(),
            reported_total,
            "Must receive all scenario results"
        );

        timeline.sort_by(|m1, m2| m1.microsecond.cmp(&m2.microsecond));

        Ok(timeline)
    }

    pub async fn run_scenario(
        &self,
        scenario: Scenario,
    ) -> Result<impl Stream<Item = Result<Recording, Error>> + '_, scenario::ValidationError> {
        scenario.validate()?;

        let (test_device_scenario, test_hid_index) = to_device_scenario(scenario.test.as_slice());
        let revert = scenario.revert.as_deref().map(to_device_scenario);
        let delay_range = scenario.delay_between_ms.0..=scenario.delay_between_ms.1;

        let (sender, receiver) = mpsc::channel::<Result<Recording, Error>>(1);

        let device = self.clone();

        tokio::spawn(async move {
            let mut unsafe_rng = SmallRng::from_entropy();

            for _repeat in 0..scenario.repeats {
                let request = host_to_device::Message::RunScenario(test_device_scenario.clone());
                let recording_result = match device.make_request(request).await {
                    Ok(resp_receiver) => Device::assemble_timeline(resp_receiver, &test_hid_index)
                        .await
                        .map(|timeline| Recording {
                            max_light_level: device.max_light_level,
                            timeline,
                        }),
                    Err(e) => Err(e),
                };

                // this is needed to only break AFTER sending the error
                let recording_err = recording_result.is_err();
                // the receiver went away
                let submit_err = sender.send(recording_result).await.is_err();
                if recording_err || submit_err {
                    break;
                }

                if let Some((ref device_scenario, _)) = revert {
                    let request = host_to_device::Message::RunScenario(device_scenario.clone());
                    match device.one_off(request).await {
                        Ok(None) => (),
                        Ok(Some(_)) => unreachable!("Revert scenario should not return anything"),
                        Err(e) => {
                            // I break anyway, doesn't matter if the receiver went away
                            _ = sender.send(Err(e)).await;
                            break;
                        }
                    }
                }

                let sleep_ms = unsafe_rng.gen_range(delay_range.clone());
                sleep(Duration::from_millis(sleep_ms as u64)).await;
            }
        });

        Ok(ReceiverStream::new(receiver))
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
}
