#![no_std]

use defmt::info;
// TODO: conditional compilation
// https://github.com/simmsb/rusty-dilemma/blob/3b166839d33b9507bc81d1d2e9c6d6c2e3be8705/firmware/src/lib.rs#L34
#[allow(unused_imports)]
use {defmt_rtt as _, panic_probe as _};

mod measurement_buffer;
mod tasks;

use crate::measurement_buffer::Buffer;
use crate::tasks::light_sensor::LightReading;
use crate::tasks::{indicator_led, light_sensor, reactor, usb};
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::usb::Driver as UsbDriver;
use embassy_sync::channel::Channel;
use embassy_sync::mutex::Mutex;
use embassy_sync::pubsub;
use embassy_sync::pubsub::PubSubChannel;
use embassy_sync::signal::Signal;

use late_mate_comms::{DeviceToHost, HidRequest, HostToDevice};

pub const HARDWARE_VERSION: u8 = 1;
// todo: maybe just use a git hash?
pub const FIRMWARE_VERSION: u32 = 1;

bind_interrupts!(struct UsbIrqs {
    USBCTRL_IRQ => embassy_rp::usb::InterruptHandler<embassy_rp::peripherals::USB>;
});

// according to the docs:
// "Use ThreadModeRawMutex when data is shared between tasks running on the same executor,
// but you want a singleton."
// I don't think we will use those channel in interrupts (Embassy handles those), plus
// we don't use the second core (yet?), so this one should be fine
type RawMutex = embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;

// max number of serial in/out messages that can be buffered before waiting for more space
const FROM_HOST_N_BUFFERED: usize = 4;
const TO_HOST_N_BUFFERED: usize = 4;

type CommsFromHost = Channel<RawMutex, HostToDevice, FROM_HOST_N_BUFFERED>;
type CommsToHost = Channel<RawMutex, DeviceToHost, TO_HOST_N_BUFFERED>;

pub static COMMS_FROM_HOST: CommsFromHost = Channel::new();
pub static COMMS_TO_HOST: CommsToHost = Channel::new();

const LIGHT_READINGS_N_BUFFERED: usize = 1;
// reactor x2 (in measurements and in background monitoring) and LED x1
const LIGHT_READINGS_MAX_SUBS: usize = 3;
const LIGHT_READINGS_MAX_PUBS: usize = 1;
type LightReadings = PubSubChannel<
    RawMutex,
    LightReading,
    LIGHT_READINGS_N_BUFFERED,
    LIGHT_READINGS_MAX_SUBS,
    LIGHT_READINGS_MAX_PUBS,
>;
type LightReadingsSubscriber = pubsub::Subscriber<
    'static,
    RawMutex,
    LightReading,
    LIGHT_READINGS_N_BUFFERED,
    LIGHT_READINGS_MAX_SUBS,
    LIGHT_READINGS_MAX_PUBS,
>;
type LightReadingsPublisher = pubsub::Publisher<
    'static,
    RawMutex,
    LightReading,
    LIGHT_READINGS_N_BUFFERED,
    LIGHT_READINGS_MAX_SUBS,
    LIGHT_READINGS_MAX_PUBS,
>;
pub static LIGHT_READINGS: LightReadings = PubSubChannel::new();

pub enum HidAckKind {
    Immediate,
    Buffered,
}

pub type HidSignal = Signal<RawMutex, (HidRequest, HidAckKind)>;
pub static HID_SIGNAL: HidSignal = Signal::new();

pub type MeasurementBuffer = Mutex<RawMutex, Buffer>;
pub static MEASUREMENT_BUFFER: MeasurementBuffer = Mutex::new(Buffer::new());

pub async fn main(spawner: Spawner) {
    info!("Late Mate is booting up");

    // todo: clocks?

    let p = embassy_rp::init(Default::default());

    let clk = p.PIN_18;
    let mosi = p.PIN_19;
    let miso = p.PIN_16;
    let drdy = p.PIN_22;

    light_sensor::init(
        &spawner,
        p.SPI0,
        clk,
        mosi,
        miso,
        p.DMA_CH0,
        p.DMA_CH1,
        drdy,
        LIGHT_READINGS.publisher().unwrap(),
    );

    let usb_driver = UsbDriver::new(p.USB, UsbIrqs);

    usb::init(
        &spawner,
        usb_driver,
        &COMMS_FROM_HOST,
        &COMMS_TO_HOST,
        &HID_SIGNAL,
        &MEASUREMENT_BUFFER,
    );

    reactor::init(
        &spawner,
        &COMMS_FROM_HOST,
        &COMMS_TO_HOST,
        LIGHT_READINGS.subscriber().unwrap(),
        LIGHT_READINGS.subscriber().unwrap(),
        &HID_SIGNAL,
        &MEASUREMENT_BUFFER,
    );

    indicator_led::init(
        &spawner,
        LIGHT_READINGS.subscriber().unwrap(),
        p.PWM_CH1,
        p.PIN_2,
    );

    //
    // let adc = adc::Adc::new(p.ADC, AdcIrqs, adc::Config::default());
    // let temp_chan = adc::Channel::new_temp_sensor(p.ADC_TEMP_SENSOR);
    //
    // temp_poller::init(&spawner, adc, temp_chan);
    //
    core::future::pending::<()>().await;
}

// TODO:
// - LED reflecting the light level
// - temperature in the status report

// TODO: USB DFU allows firmware updates!!1 embassy-usb-dfu
