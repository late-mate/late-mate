use crate::agents::dispatcher::DispatcherHandle;
use crate::agents::usb_tx::UsbTxHandle;
use crate::agents::{agent_watcher, dispatcher, usb_rx, usb_tx};
use crate::scenario::{to_device_scenario, Moment, Recording, Scenario};
use crate::usb::UsbDevice;
use late_mate_shared::comms;
use late_mate_shared::comms::device_to_host;
use late_mate_shared::comms::host_to_device;
use late_mate_shared::MAX_SCENARIO_DURATION_MS;
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
mod scenario;
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

type ResponseResult = Result<Option<device_to_host::Message>, Error>;

#[derive(Debug, Clone)]
pub struct Status {
    pub hardware_version: String,
    pub firmware_version: String,
    pub serial_number: String,
    pub max_light_level: u32,
}

impl From<device_to_host::Status> for Status {
    fn from(
        device_to_host::Status {
            version,
            max_light_level,
            serial_number,
        }: device_to_host::Status,
    ) -> Self {
        Self {
            hardware_version: version.hardware.to_string(),
            firmware_version: version.firmware.to_string(),
            serial_number: serial_number
                .into_iter()
                .map(|b| format!("{:02X}", b).to_string())
                .collect(),
            max_light_level,
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

    async fn make_request(
        &self,
        request: host_to_device::Message,
    ) -> mpsc::Receiver<ResponseResult> {
        let (receiver, envelope) = self.dispatcher.register_request(request).await;

        self.usb_tx
            .send(envelope)
            .await
            .expect("Device should be ready");

        receiver
    }

    async fn one_off(&self, request: host_to_device::Message) -> ResponseResult {
        let mut response_stream = self.make_request(request).await;

        timeout(
            Duration::from_millis(MAX_SCENARIO_DURATION_MS + 1000),
            response_stream.recv(),
        )
        .await
        .map_err(|_| Error::Timeout)?
        .expect("Pending response channel should not be dropped")
    }

    pub async fn get_status(&self) -> Result<Status, Error> {
        let response = self
            .one_off(host_to_device::Message::GetStatus)
            .await?
            .expect("Status response should be present");

        match response {
            device_to_host::Message::Status(s) => Ok(s.into()),
            _ => unreachable!("Status response should be of correct type"),
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
                let request =
                    host_to_device::Message::ExecuteScenario(test_device_scenario.clone());
                let resp_receiver = device.make_request(request).await;
                let recording_result = Device::assemble_timeline(resp_receiver, &test_hid_index)
                    .await
                    .map(|timeline| Recording {
                        max_light_level: device.max_light_level,
                        timeline,
                    });
                // this is needed to only break AFTER sending the error
                let recording_err = recording_result.is_err();
                // the receiver went away
                let submit_err = sender.send(recording_result).await.is_err();
                if recording_err || submit_err {
                    break;
                }

                if let Some((ref device_scenario, _)) = revert {
                    let request = host_to_device::Message::ExecuteScenario(device_scenario.clone());
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
                sleep(Duration::from_millis(sleep_ms)).await;
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
