use crate::tasks::usb::MAX_PACKET_SIZE as USB_MAX_PACKET_SIZE;
use crate::{CommsToHost, HidSignal};
use embassy_executor::Spawner;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::Driver;
use embassy_time::Instant;

use embassy_usb::class::hid::{Config, HidWriter, State};
use embassy_usb::Builder;
use late_mate_comms::{DeviceToHost, HidReport};
use static_cell::StaticCell;
use usbd_hid::descriptor::{KeyboardReport, MouseReport, SerializedDescriptor};

#[embassy_executor::task]
async fn hid_sender_task(
    hid_signal: &'static HidSignal,
    to_host: &'static CommsToHost,
    mut mouse_writer: HidWriter<'static, Driver<'static, USB>, 64>,
    mut keyboard_writer: HidWriter<'static, Driver<'static, USB>, 64>,
) {
    loop {
        let report = hid_signal.wait().await;
        let writer = match &report {
            HidReport::Mouse(_) => &mut mouse_writer,
            HidReport::Keyboard(_) => &mut keyboard_writer,
        };
        // todo: error handling
        writer.write(report.descriptor()).await.unwrap();
        to_host
            .send(DeviceToHost::HidReport {
                microsecond: Instant::now().as_micros(),
                hid_report: report,
            })
            .await;
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
    ));
}
