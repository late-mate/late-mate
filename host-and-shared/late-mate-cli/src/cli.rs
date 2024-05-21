mod device;
mod hid;
mod scenario;
mod send_hid_report;

use device::Device as CliDevice;
use hid::Hid as CliHid;
use scenario::Scenario as CliScenario;

use late_mate_device::Device;

#[derive(clap::Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, clap::Subcommand)]
pub enum Command {
    /// Everything related to the device itself.
    #[command(subcommand)]
    Device(CliDevice),
    /// Latency testing is done with scenarios. To start writing scenarios, look at the
    /// `examples` subcommand here.
    #[command(subcommand)]
    Scenario(CliScenario),
    /// If you want to just send a HID report without any timing, you can use this subcommand.
    #[command(subcommand)]
    Hid(CliHid),
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
}

impl Command {
    pub async fn run(self, device: &mut Device) -> anyhow::Result<()> {
        match self {
            Command::Device(CliDevice::Status(cmd)) => cmd.run(device).await,
            Command::Device(CliDevice::FirmwareUpdate(cmd)) => cmd.run(device).await,
            Command::Scenario(CliScenario::Run(cmd)) => cmd.run(device).await,
            Command::Scenario(CliScenario::Example(cmd)) => cmd.run(device).await,
            Command::Hid(CliHid::Send(cmd)) => cmd.run(device).await,
            Command::Hid(CliHid::ShowType(cmd)) => cmd.run(device).await,
        }
    }
}
