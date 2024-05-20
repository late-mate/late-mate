use embassy_executor::Spawner;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::Driver;
use embassy_usb::{Builder, UsbDevice};

#[embassy_executor::task]
pub async fn device_task(mut device: UsbDevice<'static, Driver<'static, USB>>) {
    info!("Starting USB device task");

    device.run().await
}

pub fn run(spawner: &Spawner, builder: Builder<'static, Driver<'static, USB>>) {
    let device = builder.build();

    spawner.must_spawn(device_task(device));
}
