use crate::tasks::usb;
use embassy_executor::Spawner;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::Driver;
use embassy_usb::class::cdc_acm::{CdcAcmClass, State};
use embassy_usb::Builder;
use static_cell::StaticCell;

#[embassy_executor::task]
async fn cdc_logger_task(cdc_class: CdcAcmClass<'static, Driver<'static, USB>>) {
    info!("Starting CDC logger");

    embassy_usb_logger::with_class!(1024, log::LevelFilter::Info, cdc_class).await;
}

pub struct PreparedUsb {
    pub class: CdcAcmClass<'static, Driver<'static, USB>>,
}

pub fn init_usb(builder: &mut Builder<'static, Driver<'static, USB>>) -> PreparedUsb {
    static STATE: StaticCell<State> = StaticCell::new();

    let state: &'static mut State = STATE.init(State::new());

    let class = CdcAcmClass::new(builder, state, usb::MAX_PACKET_SIZE as u16);

    PreparedUsb { class }
}

pub fn run(spawner: &Spawner, prepared_usb: PreparedUsb) {
    spawner.must_spawn(cdc_logger_task(prepared_usb.class));
}
