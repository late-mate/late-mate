use anyhow::Context;
use futures::StreamExt;
use late_mate_device::scenario::Scenario;
use late_mate_device::Device;
use std::path::PathBuf;
use std::pin::Pin;
use tokio::io::{AsyncRead, AsyncReadExt};

#[derive(clap::Args, Debug)]
pub struct Args {
    #[command(flatten)]
    pub input: ScenarioInput,
    #[command(flatten)]
    pub output: ScenarioOutput,
    /// Overrides the provided scenario's "repeats" field
    pub repeats: Option<u16>,
}

#[derive(clap::Args, Debug)]
#[group(required = true, multiple = false)]
struct ScenarioInput {
    /// Path to a .json file with the scenario to run.
    /// Mutually exclusive with --stdin
    #[arg(long)]
    file: Option<PathBuf>,

    /// If set, the scenario is expected on STDIN.
    /// Mutually exclusive with --file
    #[arg(long)]
    stdin: bool,
}

#[derive(clap::Args, Debug)]
#[group(required = true, multiple = true)]
struct ScenarioOutput {
    /// Path to a directory where a directory with JSON files will be created.
    /// Can be set simultaneously with other output options
    #[arg(long)]
    output_json_dir: Option<PathBuf>,

    /// Path to a directory where a directory with CSV files will be created
    /// Can be set simultaneously with other output options
    #[arg(long)]
    output_csv_dir: Option<PathBuf>,

    /// If set, detected changepoints are sent line-by-line to STDOUT
    /// Can be set simultaneously with other output options
    #[arg(long)]
    output_stdout: bool,
}

impl Args {
    pub async fn run(self, device: &Device) -> anyhow::Result<()> {
        let scenario_str = {
            let mut reader: Pin<Box<dyn AsyncRead>> = match file {
                Some(path) => Box::pin(
                    tokio::fs::File::open(path)
                        .await
                        .context("Error while opening the scenario file")?,
                ),
                None => Box::pin(tokio::io::stdin()),
            };
            let mut result = String::new();
            reader
                .read_to_string(&mut result)
                .await
                .context("Error while reading the scenario")?;
            result
        };

        let scenario = serde_json::from_str::<Scenario>(&scenario_str)?;
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
