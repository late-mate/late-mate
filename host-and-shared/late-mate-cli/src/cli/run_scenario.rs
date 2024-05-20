use anyhow::{anyhow, Context};
use futures::StreamExt;
use late_mate_device::scenario::Scenario;
use late_mate_device::Device;
use std::path::PathBuf;
use std::pin::Pin;
use tokio::io::{AsyncRead, AsyncReadExt};

#[derive(clap::Args, Debug)]
pub struct Args {
    /// Path to a .json or .toml with a scenario to run. Set to "-" to read from STDIN
    pub input: String,
    /// Name of the subdirectory to store results in. If not provided, the current
    /// date and time will be used
    #[arg(long)]
    pub name: Option<String>,
    #[command(flatten)]
    pub output: ScenarioOutput,
    /// Override the scenario's "repeats" field
    #[arg(long)]
    pub repeats: Option<u16>,
}

#[derive(clap::Args, Debug)]
#[group(required = true, multiple = true)]
pub struct ScenarioOutput {
    /// Path to a directory where a subdirectory with JSON files will be created.
    /// Can be set simultaneously with other output options
    #[arg(long)]
    output_json_dir: Option<PathBuf>,

    /// Path to a directory where a subdirectory with CSV files will be created.
    /// Can be set simultaneously with other output options
    #[arg(long)]
    output_csv_dir: Option<PathBuf>,

    /// If set, detected changepoints are sent line-by-line to STDOUT.
    /// Can be set simultaneously with other output options
    #[arg(long)]
    output_stdout: bool,
}

impl Args {
    async fn read_scenario(&self) -> anyhow::Result<Scenario> {
        let scenario_str = {
            let mut reader: Pin<Box<dyn AsyncRead>> = if self.input == "-" {
                Box::pin(tokio::io::stdin())
            } else {
                Box::pin(
                    tokio::fs::File::open(self.input.to_owned())
                        .await
                        .context("Error while opening the scenario file")?,
                )
            };
            let mut result = String::new();
            reader
                .read_to_string(&mut result)
                .await
                .context("Error while reading the scenario")?;
            result
        };

        if self.input.to_lowercase().ends_with(".json") {
            serde_json::from_str(&scenario_str).context("Error parsing the scenario as JSON")
        } else if self.input.to_lowercase().ends_with(".toml") {
            toml::from_str(&scenario_str).context("Error parsing the scenario as TOML")
        } else {
            Err(anyhow!(
                "The scenario file name extension must be either .toml or .json"
            ))
        }
    }

    pub async fn run(self, device: &Device) -> anyhow::Result<()> {
        let scenario = self.read_scenario().await?;
        let mut stream = device
            .run_scenario(scenario)
            .await
            .context("Scenario validation error")?;
        while let Some(x) = stream.next().await {
            println!("New testing result:\n{x:?}");
        }

        Ok(())
    }
}
