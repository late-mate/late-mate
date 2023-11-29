use crate::usb::device::MAX_PACKET_SIZE;
use embassy_executor::Spawner;
use embassy_rp::peripherals::USB;
use embassy_sync::channel::Channel;
use embassy_usb::class::cdc_acm::{CdcAcmClass, Sender, State};
use embassy_usb::Builder;
use numtoa::NumToA;
use static_cell::make_static;

const TO_HOST_BUFFER: usize = 16; // number of messages, I assume?
const FROM_HOST_BUFFER: usize = 16;
// according to the docs:
// "Use ThreadModeRawMutex when data is shared between tasks running on the same executor
// but you want a singleton."
// I don't think we will use those channel in interrupts (Embassy handles those), so
// this one should be fine
type RawMutex = embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;

pub static FROM_HOST: Channel<RawMutex, usize, FROM_HOST_BUFFER> = Channel::new();
pub static TO_HOST: Channel<RawMutex, usize, TO_HOST_BUFFER> = Channel::new();

#[embassy_executor::task]
async fn serial_out_task(mut serial_tx: Sender<'static, embassy_rp::usb::Driver<'static, USB>>) {
    loop {
        let mut rx_buf: [u8; 16];
        loop {
            let msg = TO_HOST.receive().await;
            rx_buf = [0; 16];
            msg.numtoa(10, &mut rx_buf);
            let mut end = rx_buf
                .iter()
                .enumerate()
                .filter(|(_i, x)| **x == 0)
                .next()
                .unwrap()
                .0;
            rx_buf[end] = b'\r';
            rx_buf[end + 1] = b'\n';

            if serial_tx.write_packet(&rx_buf[..]).await.is_err() {
                break;
            }
        }
    }
}

pub fn init(
    spawner: &Spawner,
    builder: &mut Builder<'static, embassy_rp::usb::Driver<'static, USB>>,
) {
    let cdc_state = make_static!(State::new());

    let class = CdcAcmClass::new(builder, cdc_state, MAX_PACKET_SIZE);

    let (serial_tx, _serial_rx) = class.split();

    spawner.must_spawn(serial_out_task(serial_tx));
}
