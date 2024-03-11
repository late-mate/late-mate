use crate::tasks::light_sensor::MAX_LIGHT_LEVEL;
use crate::{
    CommsFromHost, CommsToHost, HidSignal, LightReadingsSubscriber, RawMutex, FIRMWARE_VERSION,
    HARDWARE_VERSION,
};
use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_sync::signal::Signal;
use embassy_time::{with_timeout, Duration, Instant, TimeoutError};
use late_mate_comms::{DeviceToHost, HostToDevice, Status, Version};

// how long to wait for a new value from the ADC
const LIGHT_READING_TIMEOUT: Duration = Duration::from_millis(10);

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
                    LIGHT_READING_TIMEOUT,
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
    hid_signal: &'static HidSignal,
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
                BG_FINISH_TIME_SIGNAL
                    .signal(Instant::now() + Duration::from_millis(duration_ms as u64));
            }

            // todo: buffer the values to avoid affecting the measurements
            // todo: make microseconds u32 to save some RAM
            // todo: this is an awkward command because unlike MeasureBackground it can be blocking,
            //       and it has a hard limit, and should be synchronous so that the host could
            //       cleanly reset/repeat
            //       maybe it should JUST sent an event (eg for a reset), and testing should
            //       be separate
            HostToDevice::SendHidEvent {
                hid_event,
                duration_ms,
            } => {
                BG_FINISH_TIME_SIGNAL
                    .signal(Instant::now() + Duration::from_millis(duration_ms as u64));

                hid_signal.signal(hid_event);
            }

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
    hid_signal: &'static HidSignal,
) {
    spawner.must_spawn(bg_measurement_loop_task(comms_to_host, light_readings_sub));
    spawner.must_spawn(reactor_task(comms_from_host, comms_to_host, hid_signal));
}
