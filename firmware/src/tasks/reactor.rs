mod light_scenario_loop;
mod light_stream_loop;

use crate::serial_number::SerialNumber;
use crate::tasks::light_sensor;
use crate::tasks::usb::{bulk_comms, hid_sender};
use crate::{scenario_buffer, MutexKind, FIRMWARE_VERSION, HARDWARE_VERSION, RED_LED_GPIO_PIN};
use defmt::{error, info};
use embassy_executor::Spawner;
use embassy_rp::rom_data::reset_to_usb_boot;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Instant, Timer};
use late_mate_shared::comms::device_to_host::{DeviceToHost, MeasurementEvent, Version};
use late_mate_shared::comms::host_to_device::{HostToDevice, RequestId, ScenarioStep};
use late_mate_shared::comms::{device_to_host, host_to_device};

#[embassy_executor::task]
async fn reactor_task(
    buffer: &'static Mutex<MutexKind, scenario_buffer::Buffer>,
    serial_number: &'static SerialNumber,
) {
    info!("Starting the reactor loop");
    loop {
        let host_to_device::Envelope {
            request_id,
            request,
        } = bulk_comms::receive_from_host().await;

        let mut should_reset_to_usb_boot = false;

        let response = match request {
            HostToDevice::ResetToFirmwareUpdate => {
                should_reset_to_usb_boot = true;
                Ok(None)
            }

            HostToDevice::GetStatus => {
                let status = DeviceToHost::Status {
                    version: Version {
                        hardware: HARDWARE_VERSION,
                        firmware: FIRMWARE_VERSION,
                    },
                    max_light_level: light_sensor::MAX_LIGHT_LEVEL,
                    serial_number: serial_number.bytes(),
                };
                Ok(Some(status))
            }

            HostToDevice::StreamLightLevel { duration_ms } => {
                light_stream_loop::stream_for(
                    request_id,
                    Duration::from_millis(duration_ms as u64),
                )
                .await;
                Ok(None)
            }

            HostToDevice::SendHidReport(hid_request) => hid_sender::send(hid_request)
                .await
                // ignore the instant the HID report was sent
                .map(|_| None),

            HostToDevice::ExecuteScenario {
                start_timing_at_idx,
                scenario,
            } => execute_scenario(request_id, buffer, start_timing_at_idx, scenario)
                .await
                .map(|_| None),
        };

        // todo: handle errors here?
        bulk_comms::write_to_host(device_to_host::Envelope {
            request_id,
            response,
        })
        .await;

        if should_reset_to_usb_boot {
            info!("resetting to USB firmware update mode");

            // sleep to allow the CLI to shut down cleanly
            Timer::after(Duration::from_secs(1)).await;

            // no point in grabbing a GPIO pin peripheral here, it'll be used in the bootloader,
            // not in the firmware itself, so protecting it does nothing
            //
            // first arg: bitmask for the LED that will indicate USB Mass Storage activity
            // second arg: allows disabling bootloader USB interfaces, 0 enables everything
            reset_to_usb_boot(1 << RED_LED_GPIO_PIN, 0);
        }
    }
}

async fn execute_scenario(
    request_id: RequestId,
    buffer: &'static Mutex<MutexKind, scenario_buffer::Buffer>,
    start_timing_at_idx: Option<u8>,
    scenario: heapless::Vec<ScenarioStep, 16>,
) -> Result<(), ()> {
    info!("Executing a scenario");

    let mut timing_started = false;

    light_stream_loop::stop_streaming().await;

    for (idx, step) in scenario.into_iter().enumerate() {
        if start_timing_at_idx.is_some_and(|start_idx| idx == start_idx as usize) {
            buffer.lock().await.clear(Instant::now());
            light_scenario_loop::start().await;
            timing_started = true;
        }

        match step {
            ScenarioStep::Wait { ms } => {
                Timer::after(Duration::from_millis(ms as u64)).await;
            }
            ScenarioStep::HidRequest(hid_request) => {
                let hid_request_id = hid_request.id;
                if let Ok(instant) = hid_sender::send(hid_request).await {
                    if timing_started {
                        let push_result = buffer
                            .lock()
                            .await
                            .store(instant, MeasurementEvent::HidReport(hid_request_id));
                        if push_result.is_err() {
                            error!("Buffer push failed, stopping the scenario early");
                            light_scenario_loop::stop().await;
                            return Err(());
                        }
                    }
                }
            }
        }
    }

    // stop() is idempotent, so I can just call it regardless
    light_scenario_loop::stop().await;

    let guard = buffer.lock().await;
    let total = guard.measurements.len() as u16;
    for (idx, measurement) in guard.measurements.iter().enumerate() {
        if let Ok(idx) = u16::try_from(idx) {
            let resp = DeviceToHost::BufferedMeasurement {
                measurement: *measurement,
                idx,
                total,
            };
            bulk_comms::write_to_host(device_to_host::Envelope {
                request_id,
                response: Ok(Some(resp)),
            })
            .await;
        } else {
            error!("The buffer should be smaller than 65_535");
            return Err(());
        }
    }
    drop(guard);

    Ok(())
}

pub fn init(
    spawner: &Spawner,
    light_stream_sub: light_sensor::Subscriber,
    light_scenario_sub: light_sensor::Subscriber,
    serial_number: &'static SerialNumber,
) {
    let buffer = scenario_buffer::init();

    light_stream_loop::init(spawner, light_stream_sub);
    light_scenario_loop::init(spawner, light_scenario_sub, buffer);

    spawner.must_spawn(reactor_task(buffer, serial_number));
}
