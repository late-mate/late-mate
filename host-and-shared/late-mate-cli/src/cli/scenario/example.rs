use late_mate_device::Device;

#[derive(Debug, clap::Args)]
pub struct Args {
    #[arg(value_enum)]
    format: Format,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum)]
enum Format {
    /// Recommended! Cleaner and supports comments. The example is annotated.
    Toml,
    /// If you have to…
    Json,
}

impl Args {
    pub async fn run(self, _device: &Device) -> anyhow::Result<()> {
        match self.format {
            Format::Toml => {
                println!(
                    "{}",
                    include_str!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/scenarios/type_a.toml"
                    ))
                );
            }
            Format::Json => {
                println!(
                    "{}",
                    include_str!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/scenarios/move_mouse_right_once.json"
                    ))
                );
            }
        }

        Ok(())
    }
}
