use crate::tasks::reactor::LIGHT_READING_TIMEOUT;
use crate::{CommsToHost, LightReadingsSubscriber, RawMutex};
use defmt::{error, info};
use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_sync::signal::Signal;
use embassy_time::{with_timeout, Duration, Instant, TimeoutError};
use late_mate_shared::comms::device_to_host::DeviceToHost;

static BG_FINISH_TIME_SIGNAL: Signal<RawMutex, Instant> = Signal::new();
static BG_MEASUREMENT_ACTIVE: Signal<RawMutex, bool> = Signal::new();

#[embassy_executor::task]
async fn bg_measurement_loop_task(
    comms_to_host: &'static CommsToHost,
    mut light_readings_sub: LightReadingsSubscriber,
) {
    info!("starting bg measurement loop");
    loop {
        let mut finish_time = BG_FINISH_TIME_SIGNAL.wait().await;
        'inner: while Instant::now() < finish_time {
            // note: this can potentially be expensive, but also it's the simplest way to
            //       do this, given the finish_time mutation below
            BG_MEASUREMENT_ACTIVE.signal(true);
            match select(
                with_timeout(
                    LIGHT_READING_TIMEOUT,
                    light_readings_sub.next_message_pure(),
                ),
                BG_FINISH_TIME_SIGNAL.wait(),
            )
            .await
            {
                Either::First(Ok(reading)) => {
                    comms_to_host
                        .send(DeviceToHost::CurrentLightLevel(reading.reading))
                        .await
                }
                Either::First(Err(TimeoutError)) => {
                    error!("timeout waiting for a light reading");
                    continue 'inner;
                }
                Either::Second(new_finish_time) => finish_time = new_finish_time,
            }
        }
        BG_MEASUREMENT_ACTIVE.signal(false);
    }
}

pub fn stream_for(d: Duration) {
    BG_FINISH_TIME_SIGNAL.signal(Instant::now() + d);
}

pub async fn stop_streaming() {
    BG_FINISH_TIME_SIGNAL.signal(Instant::now());
    while BG_MEASUREMENT_ACTIVE.wait().await {
        // wait for the background measurement loop to finish
    }
}

pub fn init(
    spawner: &Spawner,
    comms_to_host: &'static CommsToHost,
    light_readings_sub_bg: LightReadingsSubscriber,
) {
    spawner.must_spawn(bg_measurement_loop_task(
        comms_to_host,
        light_readings_sub_bg,
    ));
}
