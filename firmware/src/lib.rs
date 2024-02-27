#![no_std]

use defmt::info;
// TODO: conditional compilation
// https://github.com/simmsb/rusty-dilemma/blob/3b166839d33b9507bc81d1d2e9c6d6c2e3be8705/firmware/src/lib.rs#L34
#[allow(unused_imports)]
use {defmt_rtt as _, panic_probe as _};

mod tasks;
mod temp_poller;
mod usb;

use crate::tasks::light_sensor;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::usb::Driver as UsbDriver;

bind_interrupts!(struct UsbIrqs {
    USBCTRL_IRQ => embassy_rp::usb::InterruptHandler<embassy_rp::peripherals::USB>;
});

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
    // - USB serial output
    // - USB

    // USB CDC
    // USB mass storage, ethernet card (ECM), HID, CDC (serial port)

    // USB CDC -
    // USB HID - ???
    //
    // TODO: USB DFU allows firmware updates!!1 embassy-usb-dfu

    // -- LATER --
    let usb_driver = UsbDriver::new(p.USB, UsbIrqs);
    // usb::init(&spawner, usb_driver);
    //
    // let adc = adc::Adc::new(p.ADC, AdcIrqs, adc::Config::default());
    // let temp_chan = adc::Channel::new_temp_sensor(p.ADC_TEMP_SENSOR);
    //
    // temp_poller::init(&spawner, adc, temp_chan);
    //
    core::future::pending::<()>().await;
}
