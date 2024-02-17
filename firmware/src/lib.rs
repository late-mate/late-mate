#![no_std]

use defmt::info;
// TODO: conditional compilation
// https://github.com/simmsb/rusty-dilemma/blob/3b166839d33b9507bc81d1d2e9c6d6c2e3be8705/firmware/src/lib.rs#L34
#[allow(unused_imports)]
use {defmt_rtt as _, panic_probe as _};

mod temp_poller;
mod usb;

use embassy_executor::Spawner;
use embassy_rp::pwm::{Config as PwmConfig, Pwm};
use embassy_rp::{adc, bind_interrupts};
use embassy_time::Timer;

bind_interrupts!(struct AdcIrqs {
    ADC_IRQ_FIFO => adc::InterruptHandler;
});

bind_interrupts!(struct UsbIrqs {
    USBCTRL_IRQ => embassy_rp::usb::InterruptHandler<embassy_rp::peripherals::USB>;
});

pub async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    // let mut c: PwmConfig = Default::default();
    // c.top = 0x8000;
    // c.compare_a = 8;
    // let mut pwm = Pwm::new_output_a(p.PWM_CH1, p.PIN_2, c.clone());
    //
    // loop {
    //     info!("current LED duty cycle: {}/32768", c.compare_a);
    //     Timer::after_secs(1).await;
    //     c.compare_a = c.compare_a.rotate_left(4);
    //     pwm.set_config(&c);
    // }

    // todo: clocks?

    let usb_driver = embassy_rp::usb::Driver::new(p.USB, UsbIrqs);
    usb::init(&spawner, usb_driver);

    let adc = adc::Adc::new(p.ADC, AdcIrqs, adc::Config::default());
    let temp_chan = adc::Channel::new_temp_sensor(p.ADC_TEMP_SENSOR);

    temp_poller::init(&spawner, adc, temp_chan);

    core::future::pending::<()>().await;
}
