use embassy_executor::Spawner;
use embassy_rp::peripherals::USB;
use embassy_usb::Builder;

#[embassy_executor::task]
pub async fn usb_task(builder: Builder<'static, embassy_rp::usb::Driver<'static, USB>>) {
    let mut device = builder.build();
    defmt::info!("Starting USB device");
    device.run().await;
}

pub fn init(spawner: &Spawner, builder: Builder<'static, embassy_rp::usb::Driver<'static, USB>>) {
    spawner.must_spawn(usb_task(builder));
}
