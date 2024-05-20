use crate::statistics::process_recording;
use anyhow::{anyhow, Context};
use futures::StreamExt;
use indicatif::{ProgressBar, ProgressFinish, ProgressStyle};
use late_mate_device::scenario::Scenario;
use late_mate_device::Device;
use std::path::PathBuf;
use std::pin::Pin;
use std::time::Duration;
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
    /// Path to a directory where a subdirectory with JSON results will be created.
    /// Can be set simultaneously with other output options
    #[arg(long)]
    output_json_dir: Option<PathBuf>,

    /// Path to a directory where a subdirectory with CSV results will be created.
    /// Can be set simultaneously with other output options
    #[arg(long)]
    output_csv_dir: Option<PathBuf>,
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
        let mut scenario = self.read_scenario().await?;
        if let Some(repeats_override) = self.repeats {
            scenario.repeats = repeats_override;
        }

        let progress = ProgressBar::new(u64::from(scenario.repeats));
        // Note that I can't use the spinner if I want to minimise load during the test
        progress.set_style(
            ProgressStyle::with_template(
                "{prefix:.bold}  {bar:40.cyan} {pos:>3}/{len:3} [eta: {eta}]",
            )
            .expect("Progress bar template must be correct"),
        );
        progress.set_prefix("Running the scenario…");

        let mut stream = device
            .run_scenario(scenario)
            .await
            .context("Scenario validation error")?
            .enumerate();

        while let Some((idx, result)) = stream.next().await {
            // it's OK to just return the error because Device doesn't proceed after emitting
            // an error
            let recording = result?;

            let processed = process_recording(recording);
            if let Some(changepoint_us) = processed.changepoint_us {
                let changepoint = f64::from(changepoint_us) / 1000f64;
                progress.suspend(|| println!("{changepoint:.2}"));
            } else {
                progress.suspend(|| {
                    eprintln!("No change detected");
                });
            }

            progress.inc(1);
        }

        Ok(())
    }
}
