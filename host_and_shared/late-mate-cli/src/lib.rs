use clap::{Parser, Subcommand};
use late_mate_comms::{
    encode, CrcCobsAccumulator, DeviceToHost, FeedResult, HostToDevice, MAX_BUFFER_SIZE,
};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::timeout;
use tokio_serial::{SerialPortBuilderExt, SerialPortInfo, SerialPortType, UsbPortInfo};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// runs background monitoring
    MonitorBackground,
}

// note: use single-threaded Tokio due to https://github.com/berkowski/tokio-serial/issues/69
//       or maybe just handle it carefully on a separate thread/runtime?
// note: https://github.com/berkowski/tokio-serial/issues/55
// note: https://github.com/berkowski/tokio-serial/issues/37
pub async fn run() {
    let cli = Cli::parse();

    // todo: better device ID
    // todo: error handling
    // todo: handle more than one device being connected
    let all_ports = tokio_serial::available_ports().unwrap();
    let device_port_info = all_ports
        .iter()
        .find(|info| match &info.port_type {
            SerialPortType::UsbPort(UsbPortInfo { manufacturer, .. }) => {
                manufacturer
                    .as_ref()
                    .is_some_and(|s| s.as_str() == "Late Mate")
                    && info.port_name.starts_with("/dev/cu.")
            }
            _ => false,
        })
        .expect("the device must be connected");

    // You can check for the existence of subcommands, and if found use their
    // matches just as you would the top level cmd
    match &cli.command {
        Some(Commands::MonitorBackground) => {
            monitor_background(device_port_info).await;
        }
        None => {}
    }
}

pub async fn monitor_background(device_port_info: &SerialPortInfo) {
    let mut serial_stream = tokio_serial::new(device_port_info.port_name.clone(), 115200)
        .open_native_async()
        .unwrap();

    let mut command_buf = [0u8; MAX_BUFFER_SIZE];
    let command_len = encode(
        &HostToDevice::MeasureBackground { duration_ms: 3000 },
        &mut command_buf,
    );

    serial_stream
        .write_all(&command_buf[..command_len])
        .await
        .unwrap();

    let mut cobs_acc = CrcCobsAccumulator::new();
    let mut usb_buf = [0u8; 64];

    loop {
        // todo: error handling
        let usb_len =
            match timeout(Duration::from_millis(100), serial_stream.read(&mut usb_buf)).await {
                Err(_) => {
                    println!("restart");
                    serial_stream
                        .write_all(&command_buf[..command_len])
                        .await
                        .unwrap();
                    continue;
                }
                Ok(value) => value.unwrap(),
            };

        let mut window = &usb_buf[..usb_len];

        'cobs: while !window.is_empty() {
            window = match cobs_acc.feed::<DeviceToHost>(window) {
                FeedResult::Consumed => break 'cobs,
                FeedResult::OverFull { remaining } => {
                    println!("overfull");
                    remaining
                }
                FeedResult::Error {
                    error: e,
                    remaining,
                } => {
                    println!("error: {e:?}");
                    remaining
                }
                FeedResult::Success { data, remaining } => {
                    match data {
                        DeviceToHost::LightLevel { light_level, .. } => {
                            println!(
                                "light level: {:.4}%",
                                (light_level as f64 / ((1 << 23) - 1) as f64) * 100f64
                            )
                        }
                        DeviceToHost::HidReport { .. } => {}
                        DeviceToHost::Status(_) => {}
                    }
                    // println!("data: {data:?}");
                    remaining
                }
            }
        }
    }
}
