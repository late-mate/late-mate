pub mod firmware_update;
pub mod status;

#[derive(Debug, clap::Subcommand)]
pub enum Device {
    /// Device status and versions
    Status(status::Args),
    /// Request device reset to firmware update mode
    FirmwareUpdate(firmware_update::Args),
}
