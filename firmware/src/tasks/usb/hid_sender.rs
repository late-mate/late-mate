use crate::tasks::usb::MAX_PACKET_SIZE as USB_MAX_PACKET_SIZE;
use crate::RawMutex;
use defmt::info;
use embassy_executor::Spawner;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::Driver;
use embassy_sync::channel::Channel;
use embassy_time::Instant;

use embassy_usb::class::hid::{Config, HidWriter, State};
use embassy_usb::driver::EndpointError;
use embassy_usb::Builder;
use late_mate_shared::comms::hid::{HidReport, HidRequest, HidRequestId};
use static_cell::StaticCell;
use usbd_hid::descriptor::{KeyboardReport, MouseReport, SerializedDescriptor};

static CHANNEL_IN: Channel<RawMutex, HidRequest, 1> = Channel::new();
static CHANNEL_OUT: Channel<RawMutex, (HidRequestId, Result<Instant, HidSenderError>), 1> =
    Channel::new();

#[derive(Copy, Clone, Eq, PartialEq, Debug, defmt::Format)]
pub enum HidSenderError {
    Endpoint(EndpointError),
    UnexpectedHidReport(HidReport),
}

#[embassy_executor::task]
async fn hid_sender_task(
    mut mouse_writer: HidWriter<'static, Driver<'static, USB>, 64>,
    mut keyboard_writer: HidWriter<'static, Driver<'static, USB>, 64>,
) {
    info!("Starting USB HID sender loop");

    loop {
        let HidRequest {
            id: hid_request_id,
            report: hid_report,
        } = CHANNEL_IN.receive().await;

        // todo: remove to_usbd_hid
        let result = match hid_report {
            HidReport::Mouse(r) => mouse_writer
                .write_serialize(&r.to_usbd_hid())
                .await
                .map_err(HidSenderError::Endpoint)
                .map(|| Instant::now()),
            HidReport::Keyboard(r) => keyboard_writer
                .write_serialize(&r.to_usbd_hid())
                .await
                .map_err(HidSenderError::Endpoint)
                .map(|| Instant::now()),
            other => Err(HidSenderError::UnexpectedHidReport(other)),
        };

        CHANNEL_OUT.send((hid_request_id, result)).await;
        // if let Err(e) = hid_write_result {
        //     error!("HID write error: {:?}", e);
        //     continue;
        // }
        // match ack {
        //     HidAckKind::Immediate => {
        //         to_host.send(DeviceToHost::HidReportSent(request.id)).await;
        //     }
        //     HidAckKind::Buffered => {
        //         let reported_at = Instant::now();
        //         let mut guard = measurement_buffer.lock().await;
        //         guard.store(reported_at, MeasurementEvent::HidReport(request.id));
        //     }
        // }
    }
}

pub async fn send(hid_request: HidRequest) -> (HidRequestId, Result<Instant, HidSenderError>) {
    CHANNEL_IN.send(hid_request).await;
    CHANNEL_OUT.receive().await
}

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

pub fn init(spawner: &Spawner, builder: &mut Builder<'static, Driver<'static, USB>>) {
    static MOUSE_STATE: StaticCell<State> = StaticCell::new();
    static KEYBOARD_STATE: StaticCell<State> = StaticCell::new();

    let mouse_writer = prepare_hid_writer(builder, &MOUSE_STATE, MouseReport::desc());
    let keyboard_writer = prepare_hid_writer(builder, &KEYBOARD_STATE, KeyboardReport::desc());

    spawner.must_spawn(hid_sender_task(mouse_writer, keyboard_writer));
}
