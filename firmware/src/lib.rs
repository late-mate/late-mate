#![no_std]

#[cfg(not(feature = "probe"))]
use panic_persist as _;
#[cfg(feature = "probe")]
use {defmt_rtt as _, panic_probe as _};

use defmt_or_log::*;

mod firmware_version;
mod scenario_buffer;
mod serial_number;
mod tasks;

use crate::firmware_version::get_git_firmware_version;
use crate::tasks::{indicator_led, light_sensor, reactor, usb};
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::usb::Driver as UsbDriver;
use embassy_time::Timer;
use late_mate_shared::comms::device_to_host;
use panic_persist::get_panic_message_bytes;

pub const HARDWARE_VERSION: u8 = 1;
pub const FIRMWARE_VERSION: device_to_host::FirmwareVersion = get_git_firmware_version();

bind_interrupts!(struct UsbIrqs {
    USBCTRL_IRQ => embassy_rp::usb::InterruptHandler<embassy_rp::peripherals::USB>;
});

// according to the docs:
// "Use ThreadModeRawMutex when data is shared between tasks running on the same executor,
// but you want a singleton."
// I don't think we will use those channel in interrupts (Embassy handles those), plus
// we don't use the second core (yet?), so this one should be fine
type MutexKind = embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;

// used by ResetToFirmwareUpdate to indicate disk activity
pub const RED_LED_GPIO_PIN: i32 = 14;

// Must be equal to the size of the flash chip. Pico uses a 2MB chip
pub const FLASH_SIZE: usize = 2 * 1024 * 1024;

pub async fn main(spawner: Spawner) {
    info!("Late Mate is booting up");

    let p = embassy_rp::init(Default::default());

    // per https://github.com/embassy-rs/embassy/blob/56a7b10064b830b1be1933085a5845d0d6be5f2e/examples/rp/src/bin/flash.rs#L21C1-L25C35:
    // apparently there is a race between flash access and the debug probe, wait a bit just in case
    Timer::after_millis(10).await;

    let serial_number = serial_number::read(p.FLASH);

    let clk = p.PIN_18;
    let mosi = p.PIN_19;
    let miso = p.PIN_16;
    let drdy = p.PIN_22;

    let (light_stream_sub, light_recorder_sub, light_led_sub) = light_sensor::init(
        &spawner, p.SPI0, clk, mosi, miso, p.DMA_CH0, p.DMA_CH1, drdy,
    );

    let usb_driver = UsbDriver::new(p.USB, UsbIrqs);

    usb::run(&spawner, usb_driver, serial_number);

    let panic_bytes = get_panic_message_bytes();

    reactor::init(
        &spawner,
        light_stream_sub,
        light_recorder_sub,
        serial_number,
        panic_bytes,
    );

    indicator_led::init(&spawner, light_led_sub, p.PWM_CH1, p.PIN_2);

    core::future::pending::<()>().await;
}
