mod send;
mod show_type;

#[derive(Debug, clap::Subcommand)]
pub enum Hid {
    /// Send one or more JSON-encoded HID reports
    Send(send::Args),
    /// Print a HID report TypeScript type
    ShowType(show_type::Args),
}
