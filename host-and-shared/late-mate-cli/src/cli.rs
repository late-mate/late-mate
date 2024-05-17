mod reset_to_firmware_update;
mod run_scenario;
mod send_hid_report;
mod status;
use late_mate_device::Device;

#[derive(clap::Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(clap::Subcommand, Debug)]
enum Command {
    /// Request status from the Late Mate device
    Status(status::Args),
    // todo
    // /// Stream current light level to console output (throttled down to 120hz)
    // MonitorBackground,
    // /// Run an http/websocket server
    // RunServer {
    //     #[arg(long, default_value = "127.0.0.1")]
    //     interface: IpAddr,
    //     #[arg(long, default_value = "9118")]

    //     port: u16,
    // },
    /// Send HID reports to the device. Accepts JSON-encoded HID report structure(s)
    SendHidReport(send_hid_report::Args),
    /// Execute a latency testing scenario
    RunScenario(run_scenario::Args),
    /// Request device reset to firmware update mode
    ResetToFirmwareUpdate(reset_to_firmware_update::Args),
}

impl Command {
    pub async fn run(self, device: &Device) -> anyhow::Result<()> {
        match self {
            Command::Status(cmd) => cmd.run(device).await,
            Command::ResetToFirmwareUpdate(cmd) => cmd.run(device).await,
            Command::SendHidReport(cmd) => cmd.run(device).await,
            Command::RunScenario(cmd) => cmd.run(device).await,
        }
    }
}
