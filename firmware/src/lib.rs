#![no_std]
#![feature(type_alias_impl_trait)]

// TODO: conditional compilation
// https://github.com/simmsb/rusty-dilemma/blob/3b166839d33b9507bc81d1d2e9c6d6c2e3be8705/firmware/src/lib.rs#L34
#[allow(unused_imports)]
use {defmt_rtt as _, panic_probe as _};

mod temp_poller;
mod usb;

use embassy_executor::Spawner;
use embassy_rp::{adc, bind_interrupts};

bind_interrupts!(struct AdcIrqs {
    ADC_IRQ_FIFO => adc::InterruptHandler;
});

bind_interrupts!(struct UsbIrqs {
    USBCTRL_IRQ => embassy_rp::usb::InterruptHandler<embassy_rp::peripherals::USB>;
});

pub async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    // todo: clocks?

    let usb_driver = embassy_rp::usb::Driver::new(p.USB, UsbIrqs);
    usb::init(&spawner, usb_driver);

    let adc = adc::Adc::new(p.ADC, AdcIrqs, adc::Config::default());
    let temp_chan = adc::Channel::new_temp_sensor(p.ADC_TEMP_SENSOR);

    temp_poller::init(&spawner, adc, temp_chan);

    core::future::pending::<()>().await;
}
