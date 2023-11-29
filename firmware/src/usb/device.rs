use embassy_rp::peripherals::USB;
use embassy_usb::driver::Driver;
use embassy_usb::{Builder, Config};
use static_cell::make_static;

pub const MAX_PACKET_SIZE: u16 = 64;

pub fn init_usb<'d, D: Driver<'d>>(driver: D) -> Builder<'d, D> {
    // Create embassy-usb Config
    let mut config = Config::new(0x2e8a, 0x000a);
    config.manufacturer = Some("Embassy");
    config.product = Some("USB-serial example");
    config.serial_number = Some("12345678");
    config.max_power = 100;
    config.max_packet_size_0 = 64;

    // Required for windows compatibility.
    // https://developer.nordicsemi.com/nRF_Connect_SDK/doc/1.9.1/kconfig/CONFIG_CDC_ACM_IAD.html#help
    config.device_class = 0xEF;
    config.device_sub_class = 0x02;
    config.device_protocol = 0x01;
    config.composite_with_iads = true;

    // Create embassy-usb DeviceBuilder using the driver and config.
    // It needs some buffers for building the descriptors.
    // (comments above are from the Embassy repo)

    let device_descriptor = make_static!([0; 256]);
    let config_descriptor = make_static!([0; 256]);
    let bos_descriptor = make_static!([0; 256]);
    let msos_descriptor = make_static!([]);
    let control_buf = make_static!([0; 64]);

    Builder::new(
        driver,
        config,
        device_descriptor,
        config_descriptor,
        bos_descriptor,
        msos_descriptor,
        control_buf,
    )
}

#[embassy_executor::task]
pub async fn run_usb(builder: Builder<'static, embassy_rp::usb::Driver<'static, USB>>) {
    let mut device = builder.build();
    device.run().await;
}
