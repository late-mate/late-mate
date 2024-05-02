use crate::tasks::usb::MAX_PACKET_SIZE as USB_MAX_PACKET_SIZE;
use crate::{CommsToHost, HidAckKind, HidSignal, RawMutex};
use embassy_executor::Spawner;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::Driver;
use embassy_sync::mutex::Mutex;
use embassy_time::Instant;

use embassy_usb::class::hid::{Config, HidWriter, State};
use embassy_usb::Builder;
use late_mate_shared::{DeviceToHost, HidReport, MeasurementEvent};
use static_cell::StaticCell;
use usbd_hid::descriptor::{KeyboardReport, MouseReport, SerializedDescriptor};

#[embassy_executor::task]
async fn hid_sender_task(
    hid_signal: &'static HidSignal,
    to_host: &'static CommsToHost,
    mut mouse_writer: HidWriter<'static, Driver<'static, USB>, 64>,
    mut keyboard_writer: HidWriter<'static, Driver<'static, USB>, 64>,
    measurement_buffer: &'static Mutex<RawMutex, crate::scenario_buffer::Buffer>,
) {
    defmt::info!("Starting USB HID sender loop");

    loop {
        let (request, ack) = hid_signal.wait().await;
        // todo: error handling
        let hid_write_result = match &request.report {
            HidReport::Mouse(r) => mouse_writer.write_serialize(&r.to_usbd_hid()).await,
            HidReport::Keyboard(r) => keyboard_writer.write_serialize(&r.to_usbd_hid()).await,
        };
        if let Err(e) = hid_write_result {
            defmt::error!("HID write error: {:?}", e);
            continue;
        }
        match ack {
            HidAckKind::Immediate => {
                to_host.send(DeviceToHost::HidReportSent(request.id)).await;
            }
            HidAckKind::Buffered => {
                let reported_at = Instant::now();
                let mut guard = measurement_buffer.lock().await;
                guard.store(reported_at, MeasurementEvent::HidReport(request.id));
            }
        }
    }
}

// 64 there is a buffer size, not sure why 64 in particular
fn prepare_hid_writer(
    builder: &mut Builder<'static, Driver<'static, USB>>,
    state_cell: &'static StaticCell<State>,
    report: &'static [u8],
) -> HidWriter<'static, Driver<'static, USB>, 64> {
    let state: &'static mut State = state_cell.init(State::new());
    let config = Config {
        report_descriptor: report,
        request_handler: None,
        poll_ms: 1,
        max_packet_size: USB_MAX_PACKET_SIZE,
    };
    HidWriter::new(builder, state, config)
}

pub fn init(
    spawner: &Spawner,
    builder: &mut Builder<'static, Driver<'static, USB>>,
    to_host: &'static CommsToHost,
    hid_signal: &'static HidSignal,
    measurement_buffer: &'static Mutex<RawMutex, crate::scenario_buffer::Buffer>,
) {
    static MOUSE_STATE: StaticCell<State> = StaticCell::new();
    static KEYBOARD_STATE: StaticCell<State> = StaticCell::new();

    let mouse_writer = prepare_hid_writer(builder, &MOUSE_STATE, MouseReport::desc());
    let keyboard_writer = prepare_hid_writer(builder, &KEYBOARD_STATE, KeyboardReport::desc());

    spawner.must_spawn(hid_sender_task(
        hid_signal,
        to_host,
        mouse_writer,
        keyboard_writer,
        measurement_buffer,
    ));
}
