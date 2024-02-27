// use ads1220::command::{Command, Length, Offset};
// use ads1220::config::{
//     ConversionMode, DataRate, Gain, Mode, Mux, Pga, Register0, Register1, Register2, Register3,
//     Vref,
// };
// use defmt::info;
// use embassy_executor::Spawner;
// use embassy_rp::gpio::{Input, Output, Pull};
// use embassy_rp::peripherals::{DMA_CH0, DMA_CH1, PIN_16, PIN_18, PIN_19, PIN_2, PIN_22, PWM_CH1, SPI0};
// use embassy_rp::spi;
// use embassy_rp::spi::{Async, Phase, Polarity, Spi};
// use embassy_time::Timer;
// use embassy_rp::pwm::{Config as PwmConfig, Pwm};
//
// #[embassy_executor::task]
// async fn indicator_led_task(mut pwm_channel: PWM_CH1, mut led_pin: Output<'static, PIN_2>) {
//     let mut c: PwmConfig = Default::default();
//     c.top = 0x8000;
//     c.compare_a = 8;
//     let mut pwm = Pwm::new_output_a(pwm_channel, led_pin, c.clone());
//
//     loop {
//         info!("current LED duty cycle: {}/32768", c.compare_a);
//         Timer::after_secs(1).await;
//         c.compare_a = c.compare_a.rotate_left(4);
//         pwm.set_config(&c);
//     }
// }
//
// pub fn init(
//     spawner: &Spawner,
// ) {
//
//     spawner.must_spawn(indicator_led_task(spi, drdy));
// }
