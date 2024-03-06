use crate::tasks::light_sensor::MAX_LIGHT_LEVEL;
use crate::{
    CommsFromHost, CommsToHost, LightReadingsSubscriber, FIRMWARE_VERSION, HARDWARE_VERSION,
};
use embassy_executor::Spawner;
use embassy_time::Instant;
use late_mate_comms::{DeviceToHost, HostToDevice, Status, Version};

// this is needed to make sure we don't hang the device by mistake
const MAX_MEASUREMENT_DURATION_MS: u32 = 1000 * 5;

#[embassy_executor::task]
async fn reactor_task(
    comms_from_host: &'static CommsFromHost,
    comms_to_host: &'static CommsToHost,
    mut light_readings_sub: LightReadingsSubscriber,
) {
    loop {
        match comms_from_host.receive().await {
            HostToDevice::GetStatus => {
                let status = DeviceToHost::Status(Status {
                    version: Version {
                        hardware: HARDWARE_VERSION,
                        firmware: FIRMWARE_VERSION,
                    },
                    max_light_level: MAX_LIGHT_LEVEL,
                });
                comms_to_host.send(status).await;
            }
            HostToDevice::MeasureBackground { duration_ms } => {
                if duration_ms > MAX_MEASUREMENT_DURATION_MS {
                    defmt::error!(
                        "can't measure background for {}ms, max duration is {}ms",
                        duration_ms,
                        MAX_MEASUREMENT_DURATION_MS
                    );
                    continue;
                }

                // I can do smarter things here with a select! and make it impossible to
                // block forever on waiting for a new ADC reading, but I'd rather do go simple
                // todo: maybe do something with a deadline/future combinator?
                // todo: what to do when a new command arrives while I'm looping here?
                let finish = Instant::now().as_millis() + duration_ms as u64;
                while Instant::now().as_millis() < finish {
                    // todo: maybe log tick difference to control async transmission delay?
                    let measurement = light_readings_sub.next_message_pure().await;
                    comms_to_host.send(measurement.into()).await;
                }
            }
            HostToDevice::SendHidEvent { .. } => {}
            HostToDevice::UpdateFirmware { .. } => {
                defmt::error!("firmware update is not supported yet")
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn init(
    spawner: &Spawner,
    comms_from_host: &'static CommsFromHost,
    comms_to_host: &'static CommsToHost,
    light_readings_sub: LightReadingsSubscriber,
) {
    spawner.must_spawn(reactor_task(
        comms_from_host,
        comms_to_host,
        light_readings_sub,
    ));
}
