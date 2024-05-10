//pub mod device;
mod device;
pub mod nice_hid;
// mod server;
//pub mod server;

use crate::device::Device;
use anyhow::{anyhow, Context};
use clap::{command, Parser, Subcommand};
use late_mate_shared::comms::MAX_BUFFER_SIZE;
use late_mate_shared::{MAX_SCENARIO_DURATION_MS, USB_PID, USB_VID};
use std::net::IpAddr;
use std::time::Duration;
use tokio::sync::broadcast::error::RecvError;
use tokio::task::{JoinError, JoinSet};
use tokio::time::interval;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Request status from the Late Mate device
    Status,
    /// Stream current light level to console output (throttled down to 120hz)
    MonitorBackground,
    /// Run an http/websocket server
    RunServer {
        #[arg(long, default_value = "127.0.0.1")]
        interface: IpAddr,
        #[arg(long, default_value = "9118")]
        port: u16,
    },
    /// Send HID reports to the device. Accepts a list of JSON-encoded HID report structures
    SendHidReports {
        #[arg(value_parser(parse_hid_report))]
        reports: Vec<nice_hid::HidReport>,
    },
    /// Run a single latency measurement
    Measure {
        #[arg(long, default_value = "300")]
        duration: u64,
        #[arg(long, value_parser(parse_hid_report))]
        start: nice_hid::HidReport,
        #[arg(long, requires = "followup")]
        followup_after: Option<u16>,
        #[arg(long, value_parser(parse_hid_report), requires = "followup_after")]
        followup: Option<nice_hid::HidReport>,
    },
    /// Request device reset to firmware update mode
    ResetToFirmwareUpdate,
}

fn parse_hid_report(s: &str) -> Result<nice_hid::HidReport, anyhow::Error> {
    serde_json::from_str(s).map_err(|e| anyhow!("Invalid JSON: {}", e))
}

async fn run_command(device: Device, command: Command) -> anyhow::Result<()> {
    match command {
        // Command::MonitorBackground => monitor_background(device).await?,
        Command::Status => {
            let status = device.get_status().await?;
            println!("Late Mate status: {status:?}");
        }
        Command::ResetToFirmwareUpdate => {
            device.reset_to_firmware_update().await?;
            println!("Late Mate should remount as a mass storage device");
        }
        _ => println!("todo"),
        // Command::SendHidReports { reports } => {
        //     for report in reports {
        //         device.send_hid_report(&report).await?;
        //     }
        // }
        // Command::Measure {
        //     duration,
        //     start,
        //     followup_after,
        //     followup,
        // } => {
        //     if duration > MAX_SCENARIO_DURATION_MS {
        //         return Err(anyhow!(
        //             "Maximum scenario duration is {}ms",
        //             MAX_SCENARIO_DURATION_MS
        //         ));
        //     }
        //     let measurements = device
        //         .measure(
        //             duration as u16,
        //             &start,
        //             followup.map(|f| (followup_after.unwrap(), f)),
        //         )
        //         .await?;
        //     for m in measurements {
        //         println!("{m:?}");
        //     }
        // }
        // Command::RunServer { interface, port } => {
        //     server::run(device, interface, port).await?;
        // }
        // Command::ResetToFirmwareUpdate => {
        //     device.reset_to_firmware_update().await?;
        // }
    };

    Ok(())
}

pub async fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let (device, mut device_subtasks) = Device::init().await?;

    let result = tokio::select! {
        command_result = run_command(device, cli.command) => command_result,
        // This catches the case when one of the subtasks errored out/panicked.
        // No point continuing, command_future will be cancelled, subtasks are torn down
        subtask_result = device_subtasks.join_next() => {
            print_join_error(subtask_result.expect("Subtasks shouldn't be empty"));
            Err(anyhow!("Some device tasks have failed"))
        }
    };

    device_subtasks.abort_all();
    while let Some(other_result) = device_subtasks.join_next().await {
        print_join_error(other_result);
    }

    result
}

fn print_join_error(join_result: Result<anyhow::Result<()>, JoinError>) {
    match join_result {
        Err(e) if e.is_panic() => eprintln!("A device task panic:\n{e}"),
        Err(e) if e.is_cancelled() => (),
        Err(_) => unreachable!("JoinError should be either a panic or a cancellation"),
        Ok(Err(e)) => eprintln!("A device task error:\n{e:?}"),
        Ok(Ok(())) => (),
    }
}

// pub async fn monitor_background(mut device: Device) -> anyhow::Result<()> {
//     let mut receiver = device.subscribe_to_background();
//     device.background_enable();
//
//     // 120hz, no point streaming faster
//     let mut interval = interval(Duration::from_millis(1000 / 120));
//     loop {
//         match receiver.recv().await {
//             Ok(light_level) => {
//                 println!(
//                     "{:.4}",
//                     (light_level as f64 / device.max_light_level as f64) * 100f64
//                 )
//             }
//             Err(RecvError::Lagged(_)) => continue,
//             Err(RecvError::Closed) => return Err(anyhow!("Background light level channel closed")),
//         };
//         interval.tick().await;
//     }
// }

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
