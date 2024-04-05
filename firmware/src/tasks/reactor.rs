use crate::measurement_buffer::MAX_MEASUREMENT_DURATION;
use crate::tasks::light_sensor::MAX_LIGHT_LEVEL;
use crate::{
    CommsFromHost, CommsToHost, HidAckKind, HidSignal, LightReadingsSubscriber, MeasurementBuffer,
    RawMutex, FIRMWARE_VERSION, HARDWARE_VERSION,
};
use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_futures::select::{select, Either};
use embassy_sync::signal::Signal;
use embassy_time::{with_timeout, Duration, Instant, TimeoutError, Timer};
use late_mate_comms::{DeviceToHost, HidRequest, HostToDevice, MeasureFollowup, Status, Version};

// how long to wait for a new value from the ADC
const LIGHT_READING_TIMEOUT: Duration = Duration::from_millis(10);

static BG_FINISH_TIME_SIGNAL: Signal<RawMutex, Instant> = Signal::new();
static BG_MEASUREMENT_ACTIVE: Signal<RawMutex, bool> = Signal::new();

#[embassy_executor::task]
async fn bg_measurement_loop_task(
    comms_to_host: &'static CommsToHost,
    mut light_readings_sub: LightReadingsSubscriber,
) {
    defmt::info!("starting bg measurement loop");
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
                    defmt::error!("timeout waiting for a light reading");
                    continue 'inner;
                }
                Either::Second(new_finish_time) => finish_time = new_finish_time,
            }
        }
        BG_MEASUREMENT_ACTIVE.signal(false);
    }
}

#[embassy_executor::task]
async fn reactor_task(
    comms_from_host: &'static CommsFromHost,
    comms_to_host: &'static CommsToHost,
    mut light_readings_sub: LightReadingsSubscriber,
    hid_signal: &'static HidSignal,
    measurement_buffer: &'static MeasurementBuffer,
) {
    defmt::info!("starting reactor loop");
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

            HostToDevice::SendHidReport(hid_request) => {
                hid_signal.signal((hid_request, HidAckKind::Immediate));
            }

            HostToDevice::Measure {
                duration_ms,
                start,
                followup,
            } => {
                measure(
                    comms_to_host,
                    &mut light_readings_sub,
                    hid_signal,
                    measurement_buffer,
                    duration_ms,
                    start,
                    followup,
                )
                .await;
            }
        }
    }
}

async fn measure(
    comms_to_host: &'static CommsToHost,
    light_readings_sub: &mut LightReadingsSubscriber,
    hid_signal: &'static HidSignal,
    measurement_buffer: &'static MeasurementBuffer,
    duration_ms: u16,
    start: HidRequest,
    followup: Option<MeasureFollowup>,
) {
    defmt::info!("a measurement requested");

    if duration_ms as u64 > MAX_MEASUREMENT_DURATION.as_millis() {
        defmt::error!(
            "duration_ms must be lower than {}",
            MAX_MEASUREMENT_DURATION.as_millis()
        );
        return;
    }

    defmt::info!("cancelling background measurements");

    // cancel background measurements
    BG_FINISH_TIME_SIGNAL.signal(Instant::now());
    while BG_MEASUREMENT_ACTIVE.wait().await {
        // wait for the background measurement loop to finish
    }

    defmt::info!("clearing the buffer");

    let started_at = Instant::now();
    {
        measurement_buffer.lock().await.clear(started_at);
    }

    defmt::info!("sending the start signal and measuring into the buffer");

    // todo: maybe change the signal to a channel?
    hid_signal.signal((start, HidAckKind::Buffered));

    let finish_at = started_at + Duration::from_millis(duration_ms as u64);

    let reader_future = async {
        loop {
            let light_reading = light_readings_sub.next_message_pure().await;
            if light_reading.instant < started_at {
                // it's the last value before the measurement started, ignore
                continue;
            }
            if Instant::now() >= finish_at {
                return;
            }
            measurement_buffer
                .lock()
                .await
                .store(light_reading.instant, light_reading.into());
        }
    };

    let timely_reader_future = async move {
        match with_timeout(Duration::from_millis(duration_ms as u64 * 2), reader_future).await {
            Ok(_) => (),
            Err(TimeoutError) => {
                defmt::error!("timeout while running a measurement");
                return;
            }
        }
    };

    if let Some(followup) = followup {
        let followup_future = async {
            Timer::at(started_at + Duration::from_millis(followup.after_ms as u64)).await;
            hid_signal.signal((followup.hid_request, HidAckKind::Buffered));
        };
        join(followup_future, timely_reader_future).await;
    } else {
        timely_reader_future.await;
    }

    defmt::info!("measurements finished, sending the buffer");

    let guard = measurement_buffer.lock().await;
    let total = guard.measurements.len() as u16;
    for (idx, measurement) in guard.measurements.iter().enumerate() {
        let idx = idx as u16;
        comms_to_host
            .send(DeviceToHost::BufferedMeasurement {
                measurement: *measurement,
                idx,
                total,
            })
            .await;
    }

    defmt::info!("measurement finished");
}

#[allow(clippy::too_many_arguments)]
pub fn init(
    spawner: &Spawner,
    comms_from_host: &'static CommsFromHost,
    comms_to_host: &'static CommsToHost,
    light_readings_sub_bg: LightReadingsSubscriber,
    light_readings_sub_measure: LightReadingsSubscriber,
    hid_signal: &'static HidSignal,
    measurement_buffer: &'static MeasurementBuffer,
) {
    spawner.must_spawn(bg_measurement_loop_task(
        comms_to_host,
        light_readings_sub_bg,
    ));
    spawner.must_spawn(reactor_task(
        comms_from_host,
        comms_to_host,
        light_readings_sub_measure,
        hid_signal,
        measurement_buffer,
    ));
}
