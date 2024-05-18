use defmt_or_log::*;

use crate::tasks::usb::MAX_PACKET_SIZE as USB_MAX_PACKET_SIZE;
use crate::MutexKind;
use embassy_executor::Spawner;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::Driver;
use embassy_sync::channel::Channel;
use embassy_time::Instant;
use embassy_usb::class::hid::{Config, HidWriter, State};
use embassy_usb::Builder;
use late_mate_shared::comms;
use static_cell::StaticCell;
use usbd_hid::descriptor::{KeyboardReport, MouseReport, SerializedDescriptor};

static CHANNEL_IN: Channel<MutexKind, comms::hid::HidRequest, 1> = Channel::new();
static CHANNEL_OUT: Channel<MutexKind, Result<Instant, ()>, 1> = Channel::new();

#[embassy_executor::task]
async fn hid_sender_task(
    mut mouse_writer: HidWriter<'static, Driver<'static, USB>, 64>,
    mut keyboard_writer: HidWriter<'static, Driver<'static, USB>, 64>,
) {
    info!("Starting USB HID sender loop");

    loop {
        let comms::hid::HidRequest { report, .. } = CHANNEL_IN.receive().await;

        // todo: remove to_usbd_hid, refactor
        let result = match report {
            comms::hid::HidReport::Mouse(r) => {
                match mouse_writer.write_serialize(&r.to_usbd_hid()).await {
                    Ok(_) => Ok(Instant::now()),
                    Err(e) => {
                        error!("Endpoint error while trying to send a HID report: {:?}", e);
                        Err(())
                    }
                }
            }
            comms::hid::HidReport::Keyboard(r) => {
                match keyboard_writer.write_serialize(&r.to_usbd_hid()).await {
                    Ok(_) => Ok(Instant::now()),
                    Err(e) => {
                        error!("Endpoint error while trying to send a HID report: {:?}", e);
                        Err(())
                    }
                }
            }
        };

        CHANNEL_OUT.send(result).await;
    }
}

pub async fn send(hid_request: comms::hid::HidRequest) -> Result<Instant, ()> {
    CHANNEL_IN.send(hid_request).await;
    CHANNEL_OUT.receive().await
}

type MaxPacketHidWriter = HidWriter<'static, Driver<'static, USB>, USB_MAX_PACKET_SIZE>;

fn prepare_hid_writer(
    builder: &mut Builder<'static, Driver<'static, USB>>,
    state_cell: &'static StaticCell<State>,
    report: &'static [u8],
) -> MaxPacketHidWriter {
    let state: &'static mut State = state_cell.init(State::new());
    let config = Config {
        report_descriptor: report,
        request_handler: None,
        poll_ms: 1,
        max_packet_size: USB_MAX_PACKET_SIZE as u16,
    };
    HidWriter::new(builder, state, config)
}

pub struct PreparedUsb {
    mouse_writer: MaxPacketHidWriter,
    keyboard_writer: MaxPacketHidWriter,
}

pub fn init_usb(builder: &mut Builder<'static, Driver<'static, USB>>) -> PreparedUsb {
    static MOUSE_STATE: StaticCell<State> = StaticCell::new();
    static KEYBOARD_STATE: StaticCell<State> = StaticCell::new();

    PreparedUsb {
        mouse_writer: prepare_hid_writer(builder, &MOUSE_STATE, MouseReport::desc()),
        keyboard_writer: prepare_hid_writer(builder, &KEYBOARD_STATE, KeyboardReport::desc()),
    }
}

pub fn run(
    spawner: &Spawner,
    PreparedUsb {
        mouse_writer,
        keyboard_writer,
    }: PreparedUsb,
) {
    spawner.must_spawn(hid_sender_task(mouse_writer, keyboard_writer));
}
