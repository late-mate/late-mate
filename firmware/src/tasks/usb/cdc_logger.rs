// todo: uncomment & use when https://github.com/embassy-rs/embassy/pull/2414
//       gets released
// use crate::tasks::usb;
// use defmt_or_log::{error, info};
// use embassy_executor::Spawner;
// use embassy_rp::peripherals::USB;
// use embassy_rp::usb::Driver;
// use embassy_time::Instant;
// use embassy_usb::class::cdc_acm::{CdcAcmClass, State};
// use embassy_usb::Builder;
// use late_mate_shared::comms;
// use static_cell::StaticCell;
//
// pub struct PreparedUsb {
//     pub class: CdcAcmClass<'static, Driver<'static, USB>>,
// }
//
// pub fn init_usb(builder: &mut Builder<'static, Driver<'static, USB>>) -> PreparedUsb {
//     static STATE: StaticCell<State> = StaticCell::new();
//
//     let state: &'static mut State = STATE.init(State::new());
//
//     let class = CdcAcmClass::new(builder, state, usb::MAX_PACKET_SIZE as u16);
//
//     PreparedUsb { class }
// }
//
// pub fn run(spawner: &Spawner, prepared_usb: PreparedUsb) {
//     let class = prepared_usb.class;
//     let future = embassy_usb_logger::with_class!(1024, log::LevelFilter::Info, class);
//     spawner.must_spawn(future);
// }
