#![no_std]

use defmt::info;
// TODO: conditional compilation
// https://github.com/simmsb/rusty-dilemma/blob/3b166839d33b9507bc81d1d2e9c6d6c2e3be8705/firmware/src/lib.rs#L34
#[allow(unused_imports)]
use {defmt_rtt as _, panic_probe as _};

mod ads1220;
mod temp_poller;
mod usb;

use embassy_executor::Spawner;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::pwm::{Config as PwmConfig, Pwm};
use embassy_rp::spi::{Config as SpiConfig, Phase as SpiPhase, Polarity as SpiPolarity, Spi};
use embassy_rp::{adc, bind_interrupts};
use embassy_time::Timer;

// bind_interrupts!(struct UsbIrqs {
//     USBCTRL_IRQ => embassy_rp::usb::InterruptHandler<embassy_rp::peripherals::USB>;
// });

pub async fn main(spawner: Spawner) {
    // let p = embassy_rp::init(Default::default());

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

    let p = embassy_rp::init(Default::default());
    info!("Hello World!");

    let miso = p.PIN_16;
    let mosi = p.PIN_19;
    let clk = p.PIN_18;

    let mut spi_config = SpiConfig::default();
    spi_config.frequency = 1_000_000;
    // per the datasheet:
    // "Only SPI mode 1 (CPOL = 0, CPHA = 1) is supported."
    spi_config.phase = embedded_hal::spi::MODE_1.phase;
    spi_config.polarity = embedded_hal::spi::MODE_1.polarity;

    let mut spi = Spi::new(p.SPI0, clk, mosi, miso, p.DMA_CH0, p.DMA_CH1, spi_config);

    let command: u8 = ads1220::command::Command::Wreg(
        ads1220::command::Offset::Register0,
        ads1220::command::Length::L1,
    )
    .into();
    let tx_buf = [
        command,
        ads1220::config::Register0::new(
            Default::default(),
            Default::default(),
            ads1220::config::Mux::Ain2Avss,
        )
        .to_value(),
    ];
    let mut rx_buf = [0_u8; 2];
    spi.transfer(&mut rx_buf, &tx_buf).await.unwrap();
    info!("wreg command return: {:?}", rx_buf);

    let command: u8 = ads1220::command::Command::Rreg(
        ads1220::command::Offset::Register0,
        ads1220::command::Length::L4,
    )
    .into();
    let tx_buf = [command, 0, 0, 0, 0];
    let mut rx_buf = [0_u8; 5];
    spi.transfer(&mut rx_buf, &tx_buf).await.unwrap();
    info!("rreg command return: {:?}, {=u8:b}", rx_buf, rx_buf[1]);

    loop {
        let command: u8 = ads1220::command::Command::StartOrSync.into();
        let tx_buf = [command];
        let mut rx_buf = [0_u8; 1];
        spi.transfer(&mut rx_buf, &tx_buf).await.unwrap();
        Timer::after_millis(500).await;

        // let command: u8 = ads1220::command::Command::Rdata.into();
        let tx_buf = [0, 0, 0, 0];
        let mut rx_buf = [0_u8; 4];
        spi.transfer(&mut rx_buf, &tx_buf).await.unwrap();

        info!("read result: {:?}", rx_buf);
        Timer::after_secs(1).await;
    }

    // -- LATER --
    // let usb_driver = embassy_rp::usb::Driver::new(p.USB, UsbIrqs);
    // usb::init(&spawner, usb_driver);
    //
    // let adc = adc::Adc::new(p.ADC, AdcIrqs, adc::Config::default());
    // let temp_chan = adc::Channel::new_temp_sensor(p.ADC_TEMP_SENSOR);
    //
    // temp_poller::init(&spawner, adc, temp_chan);
    //
    // core::future::pending::<()>().await;
}
