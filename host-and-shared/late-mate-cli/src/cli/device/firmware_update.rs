use late_mate_device::Device;

#[derive(Debug, clap::Args)]
pub struct Args {}

impl Args {
    pub async fn run(self, device: &Device) -> anyhow::Result<()> {
        device.reset_to_firmware_update().await?;
        println!("Firmware update started");
        println!("Late Mate should mount as a mass storage device");

        Ok(())
    }
}
