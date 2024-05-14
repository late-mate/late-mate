use clap::Parser;
use std::path::PathBuf;
use ts_rs::TS;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Directory to output typescript bindings to
    #[arg()]
    directory: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // late_mate_cli::server::api::ClientToServer::export_all_to(&cli.directory)?;
    // late_mate_cli::server::api::ServerToClient::export_all_to(&cli.directory)?;

    Ok(())
}
