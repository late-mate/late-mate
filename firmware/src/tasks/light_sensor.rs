use crate::LightReadingsPublisher;
use ads1220::command::{Command, Length, Offset};
use ads1220::config::{
    ConversionMode, DataRate, Gain, Mode, Mux, Pga, Register0, Register1, Register2, Register3,
    Vref,
};
use embassy_executor::Spawner;
use embassy_rp::gpio::{Input, Pull};
use embassy_rp::peripherals::{DMA_CH0, DMA_CH1, PIN_16, PIN_18, PIN_19, PIN_22, SPI0};
use embassy_rp::spi;
use embassy_rp::spi::{Async, Phase, Polarity, Spi};
use embassy_time::{Instant, Timer};
use late_mate_comms::MeasurementEvent;

// the measured max value
pub const MAX_LIGHT_LEVEL: u32 = (1 << 23) - 1;

#[derive(Debug, Clone, Copy)]
pub struct LightReading {
    pub instant: Instant,
    pub reading: u32,
}

impl From<LightReading> for MeasurementEvent {
    fn from(reading: LightReading) -> Self {
        Self::LightLevel(reading.reading)
    }
}

impl LightReading {
    fn current(reading: u32) -> Self {
        Self {
            instant: Instant::now(),
            reading,
        }
    }
}

#[embassy_executor::task]
async fn light_sensor_task(
    mut spi: Spi<'static, SPI0, Async>,
    mut drdy: Input<'static, PIN_22>,
    light_readings_pub: LightReadingsPublisher,
) {
    defmt::info!("configuring the ADC");
    configure_adc(&mut spi).await;

    defmt::info!("enabling the ADC");
    let mut cmd_buf = [0u8; 1];
    cmd_buf[0] = Command::StartOrSync.into();
    spi.write(&cmd_buf).await.unwrap();

    defmt::info!("Starting light sensor loop");
    loop {
        drdy.wait_for_low().await;
        let mut rx_buf = [0u8; 3];
        spi.read(&mut rx_buf).await.unwrap();

        let light_bytes = [0u8, rx_buf[0], rx_buf[1], rx_buf[2]];
        let light_level = u32::from_be_bytes(light_bytes);
        light_readings_pub.publish_immediate(LightReading::current(light_level));
    }
}

async fn configure_adc(spi: &mut Spi<'static, SPI0, Async>) {
    let full_config: [u8; 4] = [
        Register0::new()
            .with_mux(Mux::Ain2Avss)
            .with_gain(Gain::Gain1)
            .with_pga(Pga::Bypassed)
            .into(),
        Register1::new()
            .with_data_rate(DataRate::Normal1000)
            .with_mode(Mode::Turbo)
            .with_conversion_mode(ConversionMode::Continuous)
            .into(),
        Register2::new().with_vref(Vref::ExternalRefp0Refn0).into(),
        Register3::new().into(),
    ];

    let mut cmd_buf = [0u8; 1];

    cmd_buf[0] = Command::Reset.into();
    spi.write(&cmd_buf).await.unwrap();
    // per the datasheet, the ADC needs 50us + 32 ticks to reset; 1ms is DEFINITELY enough
    Timer::after_millis(1).await;

    cmd_buf[0] = Command::Wreg(Offset::Register0, Length::L4).into();
    spi.write(&cmd_buf).await.unwrap();
    spi.write(&full_config).await.unwrap();

    cmd_buf[0] = Command::Rreg(Offset::Register0, Length::L4).into();
    spi.write(&cmd_buf).await.unwrap();

    let mut readback_buf = [0u8; 4];
    spi.read(&mut readback_buf).await.unwrap();

    assert_eq!(full_config, readback_buf);
}

#[allow(clippy::too_many_arguments)]
pub fn init(
    spawner: &Spawner,
    spi_instance: SPI0,
    clk_pin: PIN_18,
    mosi_pin: PIN_19,
    miso_pin: PIN_16,
    tx_dma: DMA_CH0,
    rx_dma: DMA_CH1,
    drdy_pin: PIN_22,
    light_readings_pub: LightReadingsPublisher,
) {
    let mut spi_config = spi::Config::default();
    spi_config.frequency = 1_000_000;
    // per the datasheet:
    // "Only SPI mode 1 (CPOL = 0, CPHA = 1) is supported."
    // Mapping of mode to Phase/Polarity is taken from embedded-hal's spi::Mode::MODE_1
    spi_config.phase = Phase::CaptureOnSecondTransition;
    spi_config.polarity = Polarity::IdleLow;

    let spi = Spi::new(
        spi_instance,
        clk_pin,
        mosi_pin,
        miso_pin,
        tx_dma,
        rx_dma,
        spi_config,
    );
    let drdy = Input::new(drdy_pin, Pull::Up);

    spawner.must_spawn(light_sensor_task(spi, drdy, light_readings_pub));
}
