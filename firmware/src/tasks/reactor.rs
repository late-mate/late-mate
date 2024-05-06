mod bg_measurement_loop;

use crate::serial_number::SerialNumber;
use crate::tasks::light_sensor::MAX_LIGHT_LEVEL;
use crate::tasks::usb::{hid_sender, serial_comms};
use crate::{
    CommsFromHost, CommsToHost, HidAckKind, HidSignal, LightReadingsSubscriber, RawMutex,
    FIRMWARE_VERSION, HARDWARE_VERSION,
};
use defmt::{error, info};
use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_futures::select::{select, Either};
use embassy_rp::rom_data::reset_to_usb_boot;
use embassy_sync::mutex::Mutex;
use embassy_sync::signal::Signal;
use embassy_time::{with_timeout, Duration, Instant, TimeoutError, Timer};
use late_mate_shared::comms::device_to_host::{DeviceToHost, Version};
use late_mate_shared::comms::host_to_device::{HostToDevice, RequestId};
use late_mate_shared::comms::{device_to_host, host_to_device};

// how long to wait for a new value from the ADC
const LIGHT_READING_TIMEOUT: Duration = Duration::from_millis(10);

async fn reply_to_host(request_id: RequestId, response: Result<Option<DeviceToHost>, ()>) {
    // todo: handle errors here?
    serial_comms::write_to_host(device_to_host::Envelope {
        request_id,
        response,
    })
    .await
}

#[embassy_executor::task]
async fn reactor_task(
    mut light_readings_sub: LightReadingsSubscriber,
    hid_signal: &'static HidSignal,
    measurement_buffer: &'static Mutex<RawMutex, crate::scenario_buffer::Buffer>,
    serial_number: &'static SerialNumber,
) {
    info!("starting the reactor loop");
    loop {
        let host_to_device::Envelope {
            request_id,
            request,
        } = serial_comms::receive_from_host().await;

        match request {
            HostToDevice::ResetToFirmwareUpdate => {
                info!("resetting to the USB firmware update mode");
                // no point in a synchronous response
                reply_to_host(request_id, Ok(None)).await;

                // TODO: push that to lib.rs
                // no point in grabbing a peripheral here, it'll be used in the bootloader, not
                // in this code
                let red_led_gpio = 14;
                // first arg: bitmask for the LED that will indicate USB Mass Storage activity
                // second arg: allows disabling bootloader USB interfaces, 0 enables everything
                reset_to_usb_boot(1 << red_led_gpio, 0);
            }
            HostToDevice::GetStatus => {
                let status = DeviceToHost::Status {
                    version: Version {
                        hardware: HARDWARE_VERSION,
                        firmware: FIRMWARE_VERSION,
                    },
                    max_light_level: MAX_LIGHT_LEVEL,
                    serial_number: serial_number.bytes(),
                };
                reply_to_host(request_id, Ok(Some(status))).await;
            }

            HostToDevice::StreamLightLevel { duration_ms } => {
                bg_measurement_loop::stream_for(Duration::from_millis(duration_ms as u64));
                reply_to_host(request_id, Ok(None)).await;
            }

            HostToDevice::SendHidReport(hid_request) => {
                let (hid_request_id, result) = hid_sender::send(hid_request).await;
                
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
    measurement_buffer: &'static Mutex<RawMutex, crate::scenario_buffer::Buffer>,
    duration_ms: u16,
    start: HidRequest,
    followup: Option<MeasureFollowup>,
) {
    info!("a measurement requested");

    if duration_ms as u64 > MAX_SCENARIO_DURATION_MS {
        error!(
            "duration_ms must be lower than {}",
            MAX_SCENARIO_DURATION_MS
        );
        return;
    }

    info!("stopping background measurements");

    bg_measurement_loop::stop_streaming().await;

    info!("clearing the buffer");

    let started_at = Instant::now();
    {
        measurement_buffer.lock().await.clear(started_at);
    }

    info!("sending the start signal and measuring into the buffer");

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
                error!("timeout while running a measurement");
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

    info!("measurements finished, sending the buffer");

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

    info!("measurement finished");
}

#[allow(clippy::too_many_arguments)]
pub fn init(
    spawner: &Spawner,
    comms_from_host: &'static CommsFromHost,
    comms_to_host: &'static CommsToHost,
    light_readings_sub_bg: LightReadingsSubscriber,
    light_readings_sub_measure: LightReadingsSubscriber,
    hid_signal: &'static HidSignal,
    measurement_buffer: &'static Mutex<RawMutex, crate::scenario_buffer::Buffer>,
    serial_number: &'static SerialNumber,
) {
    bg_measurement_loop::init(spawner, comms_to_host, light_readings_sub_bg);
    spawner.must_spawn(reactor_task(
        comms_from_host,
        comms_to_host,
        light_readings_sub_measure,
        hid_signal,
        measurement_buffer,
        serial_number,
    ));
}
