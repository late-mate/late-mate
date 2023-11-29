use crate::usb::serial::TO_HOST;
use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::adc;
use embassy_time::Timer;

fn convert_to_celsius(raw_temp: u16) -> f32 {
    // According to chapter 4.9.5. Temperature Sensor in RP2040 datasheet
    let temp = 27.0 - (raw_temp as f32 * 3.3 / 4096.0 - 0.706) / 0.001721;
    let sign = if temp < 0.0 { -1.0 } else { 1.0 };
    let rounded_temp_x10: i16 = ((temp * 10.0) + 0.5 * sign) as i16;
    (rounded_temp_x10 as f32) / 10.0
}

#[embassy_executor::task]
async fn temp_poller_task(
    mut adc: adc::Adc<'static, adc::Async>,
    mut adc_channel: adc::Channel<'static>,
) {
    loop {
        let raw_temp = adc.read(&mut adc_channel).await.unwrap();
        let centicelsius = (convert_to_celsius(raw_temp) * 100f32) as usize;
        TO_HOST.send(centicelsius).await;
        Timer::after_secs(1).await;
        info!("sent an update");
    }
}

pub fn init(
    spawner: &Spawner,
    adc: adc::Adc<'static, adc::Async>,
    adc_channel: adc::Channel<'static>,
) {
    info!("Initializing temp poller");

    spawner.must_spawn(temp_poller_task(adc, adc_channel));
}
