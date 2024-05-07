use crate::tasks::light_sensor::LightReadingsSubscriber;
use crate::tasks::reactor::LIGHT_TIMEOUT;

use crate::{scenario_buffer, MutexKind};
use defmt::{error, info};
use embassy_executor::Spawner;

use embassy_sync::channel::Channel;
use embassy_sync::mutex::Mutex;

use embassy_time::{with_timeout, TimeoutError};




static SHOULD_RUN: Channel<MutexKind, bool, 1> = Channel::new();

#[embassy_executor::task]
async fn light_scenario_loop_task(
    mut light_scenario_sub: LightReadingsSubscriber,
    buffer: &'static Mutex<MutexKind, scenario_buffer::Buffer>,
) {
    info!("Starting the light scenario loop");

    let mut is_active = false;

    loop {
        while !is_active {
            is_active = SHOULD_RUN.receive().await;
        }

        'inner: while is_active {
            match with_timeout(LIGHT_TIMEOUT, light_scenario_sub.next_message_pure()).await {
                Ok(reading) => {
                    let push_result = buffer.lock().await.store(reading.instant, reading.into());
                    if push_result.is_err() {
                        error!("Buffer push failed, stopping the buffer recording");
                        is_active = false;
                        break 'inner;
                    }
                }
                Err(TimeoutError) => {
                    // if we got the timeout here, something is really wrong and there's no point
                    // continuing
                    error!("Timeout waiting for a light reading, stopping the buffer recording");
                    is_active = false;
                    break 'inner;
                }
            }

            if let Ok(new_is_active) = SHOULD_RUN.try_receive() {
                is_active = new_is_active;
            }
        }
    }
}

pub async fn start() {
    SHOULD_RUN.send(true).await;
}

pub async fn stop() {
    // if the value is taken, the loop has stopped
    SHOULD_RUN.send(false).await;
}

pub fn init(
    spawner: &Spawner,
    light_scenario_sub: LightReadingsSubscriber,
    buffer: &'static Mutex<MutexKind, scenario_buffer::Buffer>,
) {
    spawner.must_spawn(light_scenario_loop_task(light_scenario_sub, buffer));
}
