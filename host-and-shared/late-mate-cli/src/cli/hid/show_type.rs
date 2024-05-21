use late_mate_device::Device;

#[derive(Debug, clap::Args)]
pub struct Args {}

impl Args {
    // todo: build process that helps to make sure that they stay in sync
    pub async fn run(self, _device: &Device) -> anyhow::Result<()> {
        println!(
            "{}",
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/HidReport.ts"))
        );

        Ok(())
    }
}
