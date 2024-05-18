use defmt_or_log::*;

use crate::tasks::light_sensor;
use crate::tasks::usb::bulk_comms;
use crate::MutexKind;
use embassy_executor::Spawner;
use embassy_sync::channel::Channel;
use embassy_time::{with_timeout, Duration, Instant, TimeoutError};
use late_mate_shared::comms::device_to_host;
use late_mate_shared::comms::host_to_device::RequestId;

struct Request {
    request_id: RequestId,
    stream_until: Instant,
}

static STREAM_REQUEST: Channel<MutexKind, Option<Request>, 1> = Channel::new();

#[embassy_executor::task]
async fn light_stream_loop_task(mut light_stream_sub: light_sensor::Subscriber) {
    info!("Starting the light stream loop");

    let mut active_request = None;

    loop {
        while active_request.is_none() {
            active_request = STREAM_REQUEST.receive().await;
        }

        'inner: while active_request.is_some() {
            let request_id = match &active_request {
                None => break 'inner,
                Some(r) if r.stream_until < Instant::now() => {
                    active_request = None;
                    break 'inner;
                }
                Some(r) => r.request_id,
            };

            match with_timeout(light_sensor::TIMEOUT, light_stream_sub.next_message_pure()).await {
                Ok(reading) => {
                    bulk_comms::write_to_host(device_to_host::Envelope {
                        request_id,
                        response: Ok(Some(device_to_host::Message::CurrentLightLevel(
                            reading.reading,
                        ))),
                    })
                    .await;
                }
                Err(TimeoutError) => {
                    // if we got the timeout here, something is really wrong and there's no point
                    // continuing
                    error!("Timeout waiting for a light reading, stopping the stream");
                    active_request = None;
                    break 'inner;
                }
            }

            if let Ok(new_active_request) = STREAM_REQUEST.try_receive() {
                active_request = new_active_request;
            }
        }
    }
}

pub async fn stream_for(request_id: RequestId, d: Duration) {
    STREAM_REQUEST
        .send(Some(Request {
            request_id,
            stream_until: Instant::now() + d,
        }))
        .await;
}

pub async fn stop_streaming() {
    // if the value is taken, the loop has stopped
    STREAM_REQUEST.send(None).await;
}

pub fn init(spawner: &Spawner, light_stream_sub: light_sensor::Subscriber) {
    spawner.must_spawn(light_stream_loop_task(light_stream_sub));
}
