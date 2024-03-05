use crate::tasks::usb::MAX_PACKET_SIZE as USB_MAX_PACKET_SIZE;
use crate::{RawMutex, FROM_HOST_BUFFER, TO_HOST_BUFFER};
use embassy_executor::Spawner;
use embassy_rp::peripherals::USB;
use embassy_sync::channel::Channel;
use embassy_usb::class::cdc_acm::{CdcAcmClass, Receiver, Sender, State as CdcState};
use embassy_usb::Builder;
use late_mate_comms::{
    encode, CrcCobsAccumulator, DeviceToHost, FeedResult, HostToDevice,
    MAX_BUFFER_SIZE as COMMS_MAX_BUFFER_SIZE,
};
use static_cell::StaticCell;

#[embassy_executor::task]
async fn serial_rx_task(
    mut serial_rx: Receiver<'static, embassy_rp::usb::Driver<'static, USB>>,
    from_host: &'static Channel<RawMutex, HostToDevice, FROM_HOST_BUFFER>,
) {
    serial_rx.wait_connection().await;

    let mut cobs_acc = CrcCobsAccumulator::new();
    let mut usb_buf = [0u8; USB_MAX_PACKET_SIZE as usize];

    loop {
        // todo: error handling
        let usb_len = serial_rx.read_packet(&mut usb_buf).await.unwrap();

        let mut window = &usb_buf[..usb_len];

        'cobs: while !window.is_empty() {
            window = match cobs_acc.feed::<HostToDevice>(window) {
                FeedResult::Consumed => break 'cobs,
                FeedResult::OverFull { remaining } => {
                    defmt::error!("overfull");
                    remaining
                }
                FeedResult::Error {
                    error: _e,
                    remaining,
                } => {
                    // todo: can't format the error with defmt without a derive
                    defmt::error!("error");
                    remaining
                }
                FeedResult::Success { data, remaining } => {
                    from_host.send(data).await;
                    remaining
                }
            }
        }
    }
}

#[embassy_executor::task]
async fn serial_tx_task(
    mut serial_tx: Sender<'static, embassy_rp::usb::Driver<'static, USB>>,
    to_host: &'static Channel<RawMutex, DeviceToHost, TO_HOST_BUFFER>,
) {
    serial_tx.wait_connection().await;

    loop {
        let msg = to_host.receive().await;

        let buffer = &mut [0u8; COMMS_MAX_BUFFER_SIZE];
        // todo: encode shouldn't use .unwrap
        let packet_len = encode(&msg, buffer);

        // todo: error handling
        match serial_tx.write_packet(&buffer[..packet_len]).await {
            Ok(()) => {}
            Err(e) => {
                defmt::error!("Error sending to host: {:?}", e);
            }
        }
    }
}

pub fn init(
    spawner: &Spawner,
    builder: &mut Builder<'static, embassy_rp::usb::Driver<'static, USB>>,
    from_host: &'static Channel<RawMutex, HostToDevice, FROM_HOST_BUFFER>,
    to_host: &'static Channel<RawMutex, DeviceToHost, TO_HOST_BUFFER>,
) {
    static CDC_STATE: StaticCell<CdcState> = StaticCell::new();
    let cdc_state: &'static mut CdcState = CDC_STATE.init(CdcState::new());

    let class = CdcAcmClass::new(builder, cdc_state, USB_MAX_PACKET_SIZE);

    let (serial_tx, serial_rx) = class.split();

    spawner.must_spawn(serial_rx_task(serial_rx, from_host));
    spawner.must_spawn(serial_tx_task(serial_tx, to_host));
}
