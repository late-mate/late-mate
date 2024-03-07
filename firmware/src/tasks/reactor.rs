use crate::tasks::light_sensor::{LightReading, MAX_LIGHT_LEVEL};
use crate::{
    CommsFromHost, CommsToHost, LightReadingsSubscriber, RawMutex, FIRMWARE_VERSION,
    HARDWARE_VERSION,
};
use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_sync::signal::Signal;
use embassy_time::{with_timeout, Duration, Instant, TimeoutError};
use late_mate_comms::{DeviceToHost, HostToDevice, Status, Version};

// this is needed to make sure we don't hang the device by mistake
const MAX_BG_MEASUREMENT_DURATION: Duration = Duration::from_secs(5);
// how long to wait for a new value from the ADC
const BG_MEASUREMENT_TIMEOUT: Duration = Duration::from_millis(10);

static BG_FINISH_TIME_SIGNAL: Signal<RawMutex, Instant> = Signal::new();

#[embassy_executor::task]
async fn bg_measurement_loop_task(
    comms_to_host: &'static CommsToHost,
    mut light_readings_sub: LightReadingsSubscriber,
) {
    loop {
        let mut finish_time = BG_FINISH_TIME_SIGNAL.wait().await;
        'inner: while Instant::now() < finish_time {
            match select(
                with_timeout(
                    BG_MEASUREMENT_TIMEOUT,
                    light_readings_sub.next_message_pure(),
                ),
                BG_FINISH_TIME_SIGNAL.wait(),
            )
            .await
            {
                Either::First(Ok(measurement)) => comms_to_host.send(measurement.into()).await,
                Either::First(Err(TimeoutError)) => continue 'inner,
                Either::Second(new_finish_time) => finish_time = new_finish_time,
            }
        }
    }
}

#[embassy_executor::task]
async fn reactor_task(
    comms_from_host: &'static CommsFromHost,
    comms_to_host: &'static CommsToHost,
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
                if Duration::from_millis(duration_ms as u64) > MAX_BG_MEASUREMENT_DURATION {
                    defmt::error!(
                        "can't measure background for {}ms, max duration is {}ms",
                        duration_ms,
                        MAX_BG_MEASUREMENT_DURATION.as_millis()
                    );
                    continue;
                }

                let new_finish = Instant::now() + Duration::from_millis(duration_ms as u64);
                BG_FINISH_TIME_SIGNAL.signal(new_finish);
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
    spawner.must_spawn(bg_measurement_loop_task(comms_to_host, light_readings_sub));
    spawner.must_spawn(reactor_task(comms_from_host, comms_to_host));
}
