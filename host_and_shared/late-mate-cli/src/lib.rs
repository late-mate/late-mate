mod device;

use crate::device::commands::get_status;
use crate::device::rxtx::{rx_loop, tx_loop, CrcCobsCodec};
use crate::device::serial::find_serial_port;
use anyhow::{anyhow, Context};
use clap::{Parser, Subcommand};
use futures::StreamExt;
use late_mate_comms::{
    CrcCobsAccumulator, DeviceToHost, FeedResult, HidReport, HostToDevice, KeyboardReport,
    MAX_BUFFER_SIZE, USB_PID, USB_VID,
};
use std::fs::File;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::{broadcast, mpsc};
use tokio::time::{interval, sleep, timeout};
use tokio_serial::{
    SerialPortBuilderExt, SerialPortInfo, SerialPortType, SerialStream, UsbPortInfo,
};
use tokio_util::codec::Framed;
use usbd_hid::descriptor::KeyboardUsage;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Stream current light level to console output (scaled to percents and throttled down to 120hz)
    MonitorBackground,
    /// Request status from the Late Mate device
    Status,
    /// Run a simulated HID event (pressing A on the keyboard) while measuring light levels
    HidDemo { csv_filename: String },
}

pub async fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // todo: handle more than one device being connected

    let serial_port_info = find_serial_port()?;
    let serial_stream = tokio_serial::new(serial_port_info.port_name, 115200)
        .open_native_async()
        .context("Serial port opening failure")?;

    let serial_framed = Framed::new(serial_stream, CrcCobsCodec::new());
    let (serial_framed_tx, serial_framed_rx) = serial_framed.split();

    let (device_tx, device_tx_receiver) = mpsc::channel(4);
    let (device_rx_sender, device_rx) = broadcast::channel(1);

    let tx_loop_handle = tokio::spawn(tx_loop(serial_framed_tx, device_tx_receiver));
    let rx_loop_handle = tokio::spawn(rx_loop(serial_framed_rx, device_rx_sender.clone()));

    let device_status = get_status(device_tx.clone(), device_rx_sender.subscribe()).await?;

    // this async block is important to bring commands to the same return type
    let command_future = async {
        match &cli.command {
            //Command::MonitorBackground => monitor_background(device_tx, device_rx).await,
            Command::Status => {
                dbg!(device_status);
            }
            // Command::HidDemo { csv_filename } => {
            //     hid_demo(device_tx, device_rx, csv_filename.clone()).await
            // }
            _ => (),
        }
    };

    command_future.await;

    Ok(())

    // tokio::select! {
    //     // device_loop can only return an error, but if it does we should stop trying to poll
    //     // the other future, it will fail anyway
    //     ret = device_loop_future => ret,
    //     ret = command_future => ret,
    // }
}

pub async fn monitor_background(
    device_tx: mpsc::Sender<HostToDevice>,
    mut device_rx: broadcast::Receiver<DeviceToHost>,
) -> anyhow::Result<()> {
    let request_loop_future = async move {
        let mut interval = interval(Duration::from_millis(1000));

        loop {
            device_tx
                .send(HostToDevice::MeasureBackground { duration_ms: 1300 })
                .await
                .context("Error requesting more background light level values")?;
            interval.tick().await;
        }
    };

    let print_future = async move {
        // 120hz, no point streaming faster
        let mut interval = interval(Duration::from_millis(1000 / 120));
        loop {
            let msg = match device_rx.recv().await {
                Ok(x) => x,
                Err(RecvError::Lagged(_)) => continue,
                Err(RecvError::Closed) => {
                    return Err(anyhow!("Device RX channel was unexpectedly closed"))
                }
            };
            if let DeviceToHost::CurrentLightLevel(light_level) = msg {
                println!(
                    "{:.4}",
                    // todo: pull max light level from the status command
                    (light_level as f64 / ((1 << 23) - 1) as f64) * 100f64
                )
            }
            interval.tick().await;
        }
    };

    tokio::select! {
        ret = request_loop_future => ret,
        ret = print_future => ret
    }
}

// pub async fn hid_demo(
//     device_tx: mpsc::Sender<HostToDevice>,
//     mut device_rx: broadcast::Receiver<DeviceToHost>,
//     csv_filename: String,
// ) -> anyhow::Result<()> {
//     sleep(Duration::from_secs(3)).await;
//
//     let req_future = async move {
//         device_tx
//             .send(HostToDevice::SendHidEvent {
//                 hid_event: HidReport::Keyboard(KeyboardReport {
//                     modifier: 0,
//                     reserved: 0,
//                     leds: 0,
//                     keycodes: [KeyboardUsage::KeyboardAa as u8, 0, 0, 0, 0, 0],
//                 }),
//                 duration_ms: 300,
//             })
//             .await
//             .context("Device TX channel was unexpectedly closed")
//     };
//     let resp_future = async move {
//         let mut data: Vec<(&'static str, u64, u32)> = vec![];
//         loop {
//             // todo: make this sane
//             match timeout(Duration::from_millis(100), device_rx.recv()).await {
//                 Ok(Ok(DeviceToHost::LightLevel {
//                     microsecond,
//                     light_level,
//                 })) => data.push(("light_level", microsecond, light_level)),
//                 Ok(Ok(DeviceToHost::HidReport { microsecond, .. })) => {
//                     data.push(("hid_event", microsecond, 0))
//                 }
//                 Ok(Ok(_)) => continue,
//                 Ok(Err(RecvError::Lagged(_))) => continue,
//                 Ok(Err(RecvError::Closed)) => {
//                     let result: anyhow::Result<Vec<(&'static str, u64, u32)>> =
//                         Err(anyhow!("Device RX channel was unexpectedly closed"));
//                     return result;
//                 }
//                 Err(_) => return Ok(data),
//             }
//         }
//     };
//
//     match tokio::try_join!(req_future, resp_future) {
//         Ok((_, data)) => {
//             dbg!(&data);
//             let mut file = File::create(csv_filename).context("CSV file creation error")?;
//             let mut start: Option<u64> = None;
//             for (kind, microsecond, light_level) in data {
//                 match start {
//                     None if kind == "hid_event" => start = Some(microsecond),
//                     None => continue,
//                     Some(start_microsecond) => {
//                         writeln!(
//                             file,
//                             "{:.4},{:.4}",
//                             (microsecond - start_microsecond) as f64 / 1000f64,
//                             light_level as f64 / ((1 << 23) - 1) as f64
//                         )
//                         .context("CSV file write error")?;
//                     }
//                 }
//             }
//             file.flush()?;
//             Ok(())
//         }
//         // have to rewrap to change the type ('Ok' branches are different)
//         Err(e) => Err(e),
//     }
// }
