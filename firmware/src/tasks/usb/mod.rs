use crate::RawMutex;
use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::Driver;
use embassy_sync::channel::Channel;
use embassy_usb::{Builder, Config};
use late_mate_comms::{DeviceToHost, HostToDevice};
use static_cell::StaticCell;

mod device;
pub mod serial_comms;

// maximum for full speed USB
pub const MAX_PACKET_SIZE: u16 = 64;

pub fn init_usb<'d, D: embassy_usb::driver::Driver<'d>>(driver: D) -> Builder<'d, D> {
    // Create embassy-usb Config
    let mut config = Config::new(0x2e8a, 0x000a);
    config.manufacturer = Some("Embassy");
    config.product = Some("USB-serial example");
    config.serial_number = Some("12345678");
    config.max_power = 100;
    // todo: docstring suggests leaving it the default value (8)?
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

    static DEVICE_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
    static CONFIG_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
    static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
    static MSOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
    static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();

    let device_descriptor: &'static mut [u8; 256] = DEVICE_DESCRIPTOR.init([0; 256]);
    let config_descriptor: &'static mut [u8; 256] = CONFIG_DESCRIPTOR.init([0; 256]);
    let bos_descriptor: &'static mut [u8; 256] = BOS_DESCRIPTOR.init([0; 256]);
    let msos_descriptor: &'static mut [u8; 256] = MSOS_DESCRIPTOR.init([0; 256]);
    let control_buf: &'static mut [u8; 64] = CONTROL_BUF.init([0; 64]);

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

pub fn init(
    spawner: &Spawner,
    driver: Driver<'static, USB>,
    from_host: &'static Channel<RawMutex, HostToDevice, { crate::FROM_HOST_BUFFER }>,
    to_host: &'static Channel<RawMutex, DeviceToHost, { crate::TO_HOST_BUFFER }>,
) {
    info!("Initializing usb");

    let mut builder = init_usb(driver);

    serial_comms::init(spawner, &mut builder, from_host, to_host);

    device::init(spawner, builder);
}
