use defmt_or_log::*;

use crate::tasks::light_sensor;
use crate::{scenario_buffer, MutexKind};
use embassy_executor::Spawner;
use embassy_sync::channel::Channel;
use embassy_sync::mutex::Mutex;
use embassy_time::{with_timeout, Instant, TimeoutError};

static SHOULD_RUN_SINCE: Channel<MutexKind, Option<Instant>, 1> = Channel::new();

#[embassy_executor::task]
async fn light_recorder_loop_task(
    mut light_recorder_sub: light_sensor::Subscriber,
    buffer: &'static Mutex<MutexKind, scenario_buffer::Buffer>,
) {
    info!("Starting the light scenario loop");

    let mut should_run_since = None;

    loop {
        while should_run_since.is_none() {
            should_run_since = SHOULD_RUN_SINCE.receive().await;
        }

        'inner: while should_run_since.is_some() {
            match with_timeout(
                light_sensor::TIMEOUT,
                light_recorder_sub.next_message_pure(),
            )
            .await
            {
                Ok(reading) => {
                    if reading.instant < should_run_since.unwrap() {
                        // There might be a value in the channel that was generated earlier
                        // than the recording has started. Just skip it
                        debug!("Got a light value from the past");
                        continue;
                    }
                    let push_result = buffer.lock().await.store(reading.instant, reading.into());
                    if push_result.is_err() {
                        error!("Buffer push failed, stopping the buffer recording");
                        should_run_since = None;
                        break 'inner;
                    }
                }
                Err(TimeoutError) => {
                    // if we got the timeout here, something is really wrong and there's no point
                    // continuing
                    error!("Timeout waiting for a light reading, stopping the buffer recording");
                    should_run_since = None;
                    break 'inner;
                }
            }

            if let Ok(new) = SHOULD_RUN_SINCE.try_receive() {
                should_run_since = new;
            }
        }
    }
}

pub async fn start(since: Instant) {
    SHOULD_RUN_SINCE.send(Some(since)).await;
}

pub async fn stop() {
    // if the value is taken, the loop has stopped
    SHOULD_RUN_SINCE.send(None).await;
}

pub fn init(
    spawner: &Spawner,
    light_recorder_sub: light_sensor::Subscriber,
    buffer: &'static Mutex<MutexKind, scenario_buffer::Buffer>,
) {
    spawner.must_spawn(light_recorder_loop_task(light_recorder_sub, buffer));
}
