use anyhow::anyhow;
use late_mate_device::hid::HidReport;
use late_mate_device::Device;

fn parse_hid_report(s: &str) -> Result<HidReport, anyhow::Error> {
    serde_json::from_str(s).map_err(|e| anyhow!("Invalid HID JSON: {}", e))
}

#[derive(Debug, clap::Args)]
pub struct Args {
    /// One or several JSON-encoded HID reports
    #[arg(value_parser(parse_hid_report))]
    report: Vec<HidReport>,
}

impl Args {
    pub async fn run(self, device: &Device) -> anyhow::Result<()> {
        for report in self.report {
            device.send_hid_report(&report).await?;
        }
        eprintln!("Done!");

        Ok(())
    }
}
