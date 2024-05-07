use crate::tasks::light_sensor;
use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_rp::peripherals::{PIN_2, PWM_CH1};
use embassy_rp::pwm::{Config as PwmConfig, Pwm};
use embassy_time::{Duration, Ticker};

#[embassy_executor::task]
async fn indicator_led_task(
    mut light_readings_sub: light_sensor::Subscriber,
    pwm_channel: PWM_CH1,
    w_led_pin: PIN_2,
) {
    let max_pwm = 0x8000;
    let correction = 0.45f32;

    let mut c: PwmConfig = Default::default();
    c.top = max_pwm;
    c.phase_correct = true;

    let mut pwm = Pwm::new_output_a(pwm_channel, w_led_pin, c.clone());

    let mut ticker = Ticker::every(Duration::from_millis(7));

    let fraction = 1. / (light_sensor::MAX_LIGHT_LEVEL as f32) * correction;

    let mut buffer = [0u32; 5];
    let mut idx = 0;

    loop {
        let (_, reading) = join(ticker.next(), light_readings_sub.next_message_pure()).await;

        buffer[idx] = reading.reading;
        idx += 1;
        if idx == (buffer.len()) {
            idx = 0;

            let avg = buffer.iter().sum::<u32>() as f32 / (buffer.len() as f32);
            c.compare_a = (avg * fraction * (max_pwm as f32)) as u16;

            pwm.set_config(&c);
        }
    }
}

pub fn init(
    spawner: &Spawner,
    light_readings_sub: light_sensor::Subscriber,
    pwm_channel: PWM_CH1,
    w_led_pin: PIN_2,
) {
    spawner.must_spawn(indicator_led_task(
        light_readings_sub,
        pwm_channel,
        w_led_pin,
    ));
}
