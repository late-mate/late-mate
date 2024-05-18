use defmt_or_log::*;

use crate::serial_number::SerialNumber;
use embassy_executor::Spawner;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::Driver;
use embassy_usb::msos::windows_version;
use embassy_usb::{Builder, Config};
use late_mate_shared::{USB_PID, USB_VID};
use static_cell::ConstStaticCell;

pub mod bulk_comms;
mod cdc_logger;
mod device;
pub mod hid_sender;

// maximum for full speed USB
pub const MAX_PACKET_SIZE: usize = 64;

pub fn init_usb<'d, D: embassy_usb::driver::Driver<'d>>(
    driver: D,
    serial_number: &'static SerialNumber,
) -> Builder<'d, D> {
    // Create embassy-usb Config
    let mut config = Config::new(USB_VID, USB_PID);
    config.manufacturer = Some("YNO Engineering");
    config.product = Some("Late Mate test board rev1");
    // "Binary Coded Decimal", this corresponds to "1.0"
    config.device_release = 0x0100;
    config.serial_number = Some(serial_number.hex_str());
    config.max_power = 100;

    // todo: wtf
    // Apparently Windows 7 fails to enumerate a USB device unless it's set up as a composite
    // device with IAD (Microsoft's way to group interfaces), the bug is claimed to exist here:
    // https://developer.nordicsemi.com/nRF_Connect_SDK/doc/1.9.1/kconfig/CONFIG_CDC_ACM_IAD.html#help
    // IAD description:
    // https://learn.microsoft.com/en-us/windows-hardware/drivers/usbcon/usb-interface-association-descriptor
    // The following device class/subclass/protocol are special values signalling that IAD might
    // be present
    config.device_class = 0xEF;
    config.device_sub_class = 0x02;
    config.device_protocol = 0x01;
    config.composite_with_iads = true;

    // Embassy's USB needs a bunch of buffers. ConstStaticCell guarantees those arrays
    // are completely static and are never on the stack
    static DEVICE_DESCRIPTOR: ConstStaticCell<[u8; 256]> = ConstStaticCell::new([0; 256]);
    static CONFIG_DESCRIPTOR: ConstStaticCell<[u8; 256]> = ConstStaticCell::new([0; 256]);
    static BOS_DESCRIPTOR: ConstStaticCell<[u8; 256]> = ConstStaticCell::new([0; 256]);
    static MSOS_DESCRIPTOR: ConstStaticCell<[u8; 256]> = ConstStaticCell::new([0; 256]);
    static CONTROL_BUF: ConstStaticCell<[u8; 64]> = ConstStaticCell::new([0; 64]);

    let device_descriptor: &'static mut [u8; 256] = DEVICE_DESCRIPTOR.take();
    let config_descriptor: &'static mut [u8; 256] = CONFIG_DESCRIPTOR.take();
    let bos_descriptor: &'static mut [u8; 256] = BOS_DESCRIPTOR.take();
    let msos_descriptor: &'static mut [u8; 256] = MSOS_DESCRIPTOR.take();
    let control_buf: &'static mut [u8; 64] = CONTROL_BUF.take();

    let mut builder = Builder::new(
        driver,
        config,
        device_descriptor,
        config_descriptor,
        bos_descriptor,
        msos_descriptor,
        control_buf,
    );

    // following Embassy's example
    // https://github.com/embassy-rs/embassy/blob/9cbbedef793d619c659c6a81080675282690a8af/examples/rp/src/bin/usb_raw_bulk.rs#L94C5-L94C57
    // WinUSB support is declared on the interface level
    builder.msos_descriptor(windows_version::WIN8_1, 0);

    builder
}

pub fn run(spawner: &Spawner, driver: Driver<'static, USB>, serial_number: &'static SerialNumber) {
    info!("Initializing usb");

    let mut builder = init_usb(driver, serial_number);

    let serial_usb = bulk_comms::init_usb(&mut builder);
    let hid_usb = hid_sender::init_usb(&mut builder);

    device::run(spawner, builder);

    bulk_comms::run(spawner, serial_usb);
    hid_sender::run(spawner, hid_usb);
}
