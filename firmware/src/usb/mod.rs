use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::Driver;

mod device;
pub mod serial;

pub fn init(spawner: &Spawner, driver: Driver<'static, USB>) {
    info!("Initializing usb");

    let mut builder = device::init_usb(driver);

    serial::init(spawner, &mut builder);

    spawner.must_spawn(device::run_usb(builder));
}