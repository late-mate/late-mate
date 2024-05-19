use late_mate_device::Device;

#[derive(clap::Args, Debug)]
pub struct Args {}

impl Args {
    pub async fn run(self, device: &mut Device) -> anyhow::Result<()> {
        let status = device.get_status().await?;
        println!("Connection: success");
        println!("Serial number: {}", status.serial_number);
        println!("Version:");
        println!("  Hardware: {}", status.hardware_version);
        println!("  Firmware: {}", status.firmware_version);

        if let Some(panic_message) = status.last_panic_message {
            // todo: make it red
            println!();
            println!("Warning: the device experienced a panic, message below:");
            println!("{panic_message}");
        }

        Ok(())
    }
}
