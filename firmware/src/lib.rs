#![no_std]

use defmt::info;
// TODO: conditional compilation
// https://github.com/simmsb/rusty-dilemma/blob/3b166839d33b9507bc81d1d2e9c6d6c2e3be8705/firmware/src/lib.rs#L34
#[allow(unused_imports)]
use {defmt_rtt as _, panic_probe as _};

mod tasks;

use crate::tasks::{light_sensor, usb};
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::usb::Driver as UsbDriver;
use embassy_sync::channel::Channel;
use late_mate_comms::{DeviceToHost, HostToDevice};

bind_interrupts!(struct UsbIrqs {
    USBCTRL_IRQ => embassy_rp::usb::InterruptHandler<embassy_rp::peripherals::USB>;
});

// max number of serial in/out messages that can be buffered before waiting for more space
pub const FROM_HOST_BUFFER: usize = 4;
pub const TO_HOST_BUFFER: usize = 4;

// according to the docs:
// "Use ThreadModeRawMutex when data is shared between tasks running on the same executor,
// but you want a singleton."
// I don't think we will use those channel in interrupts (Embassy handles those), plus
// we don't use the second core (yet?), so this one should be fine
type RawMutex = embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;

pub static COMMS_FROM_HOST: Channel<RawMutex, HostToDevice, FROM_HOST_BUFFER> = Channel::new();
pub static COMMS_TO_HOST: Channel<RawMutex, DeviceToHost, TO_HOST_BUFFER> = Channel::new();

pub async fn main(spawner: Spawner) {
    info!("Late Mate is booting up");

    // todo: clocks?

    let p = embassy_rp::init(Default::default());

    let clk = p.PIN_18;
    let mosi = p.PIN_19;
    let miso = p.PIN_16;
    let drdy = p.PIN_22;

    light_sensor::init(
        &spawner, p.SPI0, clk, mosi, miso, p.DMA_CH0, p.DMA_CH1, drdy,
    );

    // TODO: https://docs.embassy.dev/embassy-sync/git/default/pubsub/struct.PubSubChannel.html
    //       for the current light level

    // TODO:
    // - LED reflecting the light level
    // - USB

    // TODO: USB DFU allows firmware updates!!1 embassy-usb-dfu

    let usb_driver = UsbDriver::new(p.USB, UsbIrqs);
    usb::init(&spawner, usb_driver, &COMMS_FROM_HOST, &COMMS_TO_HOST);
    //
    // let adc = adc::Adc::new(p.ADC, AdcIrqs, adc::Config::default());
    // let temp_chan = adc::Channel::new_temp_sensor(p.ADC_TEMP_SENSOR);
    //
    // temp_poller::init(&spawner, adc, temp_chan);
    //
    core::future::pending::<()>().await;
}
