use crate::tasks::usb::MAX_PACKET_SIZE as USB_MAX_PACKET_SIZE;
use crate::MutexKind;
use defmt::{error, info};
use embassy_executor::Spawner;
use embassy_rp::peripherals::USB;
use embassy_sync::channel::Channel;

use embassy_usb::class::cdc_acm::{CdcAcmClass, Receiver, Sender, State};
use embassy_usb::Builder;
use late_mate_shared::comms::{
    device_to_host, encode, host_to_device, CrcCobsAccumulator, FeedResult,
    MAX_BUFFER_SIZE as COMMS_MAX_BUFFER_SIZE,
};
use static_cell::StaticCell;

// max number of serial in/out messages that can be buffered before waiting for more space
const FROM_HOST_N_BUFFERED: usize = 4;
const TO_HOST_N_BUFFERED: usize = 4;

static FROM_HOST: Channel<MutexKind, host_to_device::Envelope, FROM_HOST_N_BUFFERED> =
    Channel::new();
static TO_HOST: Channel<MutexKind, device_to_host::Envelope, TO_HOST_N_BUFFERED> = Channel::new();

#[embassy_executor::task]
async fn serial_rx_task(mut serial_rx: Receiver<'static, embassy_rp::usb::Driver<'static, USB>>) {
    serial_rx.wait_connection().await;

    let mut cobs_acc = CrcCobsAccumulator::new();
    let mut usb_buf = [0u8; USB_MAX_PACKET_SIZE];

    info!("Starting USB serial RX loop");
    loop {
        // todo: error handling
        let usb_len = serial_rx.read_packet(&mut usb_buf).await.unwrap();

        let mut window = &usb_buf[..usb_len];

        'cobs: while !window.is_empty() {
            window = match cobs_acc.feed::<host_to_device::Envelope>(window) {
                FeedResult::Consumed => break 'cobs,
                FeedResult::OverFull { remaining } => {
                    error!("overfull");
                    remaining
                }
                FeedResult::Error {
                    error: e,
                    remaining,
                } => {
                    error!("COBS/CRC decoding error: {:?}", e);
                    remaining
                }
                FeedResult::Success { data, remaining } => {
                    FROM_HOST.send(data).await;
                    remaining
                }
            }
        }
    }
}

#[embassy_executor::task]
async fn serial_tx_task(mut serial_tx: Sender<'static, embassy_rp::usb::Driver<'static, USB>>) {
    serial_tx.wait_connection().await;

    info!("Starting USB serial TX loop");
    loop {
        let msg = TO_HOST.receive().await;

        let buffer = &mut [0u8; COMMS_MAX_BUFFER_SIZE];
        // todo: encode shouldn't use .unwrap
        let packet_len = encode(&msg, buffer);

        // todo: error handling
        match serial_tx.write_packet(&buffer[..packet_len]).await {
            Ok(()) => {}
            Err(e) => {
                error!("EndpointError sending to host: {:?}", e);
            }
        }
    }
}

pub async fn write_to_host(envelope: device_to_host::Envelope) {
    TO_HOST.send(envelope).await
}

pub async fn receive_from_host() -> host_to_device::Envelope {
    FROM_HOST.receive().await
}

pub struct PreparedUsb(CdcAcmClass<'static, embassy_rp::usb::Driver<'static, USB>>);

pub fn init_usb(
    builder: &mut Builder<'static, embassy_rp::usb::Driver<'static, USB>>,
) -> PreparedUsb {
    static CDC_STATE: StaticCell<State> = StaticCell::new();
    let cdc_state: &'static mut State = CDC_STATE.init(State::new());

    PreparedUsb(CdcAcmClass::new(
        builder,
        cdc_state,
        USB_MAX_PACKET_SIZE as u16,
    ))
}

pub fn run(spawner: &Spawner, PreparedUsb(cdc_class): PreparedUsb) {
    let (serial_tx, serial_rx) = cdc_class.split();

    spawner.must_spawn(serial_rx_task(serial_rx));
    spawner.must_spawn(serial_tx_task(serial_tx));
}
