mod file_output;

use crate::cli::run_scenario::file_output::{FileOutput, FileOutputKind};
use crate::statistics::process_recording;
use anyhow::{anyhow, Context};
use futures::StreamExt;
use indicatif::{HumanDuration, ProgressBar, ProgressState, ProgressStyle};
use late_mate_device::scenario::{Recording, Scenario};
use late_mate_device::Device;
use std::path::PathBuf;
use std::pin::Pin;
use tokio::io::{AsyncRead, AsyncReadExt};

#[derive(clap::Args, Debug)]
pub struct Args {
    /// Path to a .json or .toml with a scenario to run. Set to "-" to read from STDIN
    pub input: String,

    /// Name of this test run. It is used for a subdirectory to store results in.
    /// If not provided, current date and time are used
    #[arg(long)]
    pub name: Option<String>,

    /// Path to a directory where a subdirectory with JSON results will be created.
    /// Can be used simultaneously with other output options
    #[arg(long)]
    output_json_dir: Option<PathBuf>,

    /// Path to a directory where a subdirectory with CSV results will be created.
    /// Can be used simultaneously with other output options
    #[arg(long)]
    output_csv_dir: Option<PathBuf>,

    /// Override scenario's "repeats" field
    #[arg(long)]
    pub repeats: Option<u16>,
}

async fn read_scenario(input: &str) -> anyhow::Result<Scenario> {
    let input_stdin = input == "-";

    let scenario_str = {
        let mut reader: Pin<Box<dyn AsyncRead>> = if input_stdin {
            Box::pin(tokio::io::stdin())
        } else {
            Box::pin(
                tokio::fs::File::open(input.to_owned())
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

    if input_stdin {
        if let Ok(s) = serde_json::from_str::<Scenario>(&scenario_str) {
            Ok(s)
        } else if let Ok(s) = toml::from_str::<Scenario>(&scenario_str) {
            Ok(s)
        } else {
            Err(anyhow!(
                "Can't parse STDIN as either JSON or TOML. \
                 You can pass the input as a file for a more specific error."
            ))
        }
    } else if input.to_lowercase().ends_with(".json") {
        serde_json::from_str(&scenario_str).context("Error parsing the scenario as JSON")
    } else if input.to_lowercase().ends_with(".toml") {
        toml::from_str(&scenario_str).context("Error parsing the scenario as TOML")
    } else {
        Err(anyhow!(
            "The scenario file name extension must be either .toml or .json"
        ))
    }
}

fn get_progressbar(scenario: &Scenario) -> ProgressBar {
    let (total_min, total_max) = scenario.total_duration();
    let total_avg = (total_max + total_min) / 2;
    let per_repeat_avg = total_avg / scenario.repeats.into();

    let repeats = u64::from(scenario.repeats);

    let progress = if repeats > 1 {
        ProgressBar::new(repeats)
    } else {
        ProgressBar::hidden()
    };
    // Note that I can't use the spinner if I want to minimise load during the test
    progress.set_style(
        ProgressStyle::with_template(
            "{prefix:.bold}  [ {bar:40.cyan} ] {pos:>3}/{len:3} [ETA: {eta}]",
        )
        .expect("Progress bar template must be correct")
        .with_key(
            "eta",
            move |state: &ProgressState, w: &mut dyn std::fmt::Write| {
                let eta = per_repeat_avg
                    * u32::try_from(repeats - state.pos())
                        .expect("Number of repeats left must fit into u32");
                write!(w, "{:#}", HumanDuration(eta)).expect("ETA write must succeed")
            },
        )
        .progress_chars("##-"),
    );
    progress.set_prefix("Running the scenario…");

    progress
}

impl Args {
    async fn prepare(&self) -> anyhow::Result<(Scenario, Vec<FileOutput>)> {
        let mut scenario = read_scenario(self.input.as_str()).await?;
        if let Some(repeats_override) = self.repeats {
            scenario.repeats = repeats_override;
        }

        let run_name = self
            .name
            .to_owned()
            .unwrap_or(chrono::Local::now().format("%Y_%m_%d-%H_%M_%S").to_string());

        let output_json = if let Some(ref dir) = self.output_json_dir {
            let output = FileOutput::prepare(FileOutputKind::Json, dir, &run_name)
                .await
                .context("Error while preparing JSON output directory")?;
            Some(output)
        } else {
            None
        };

        let output_csv = if let Some(ref dir) = self.output_csv_dir {
            let output = FileOutput::prepare(FileOutputKind::Csv, dir, &run_name)
                .await
                .context("Error while preparing CSV output directory")?;
            Some(output)
        } else {
            None
        };

        let file_outputs = vec![output_json, output_csv]
            .into_iter()
            .flatten()
            .collect();

        Ok((scenario, file_outputs))
    }

    fn output_init(&self, scenario: &Scenario, progress: &ProgressBar) {
        if scenario.repeats > 1 {
            progress.suspend(|| eprintln!("Input latencies (in milliseconds):"));
        } else {
            progress.suspend(|| eprintln!("Latency (in milliseconds):"));
        }
    }

    async fn output_step(
        &self,
        scenario: &Scenario,
        progress: &ProgressBar,
        file_outputs: &[FileOutput],
        idx: usize,
        recording: Recording,
    ) -> anyhow::Result<()> {
        let processed = process_recording(recording);

        if let Some(changepoint_us) = processed.changepoint_us {
            let changepoint = f64::from(changepoint_us) / 1000f64;
            progress.suspend(|| println!("{changepoint:.1}"));
        } else {
            progress.suspend(|| {
                eprintln!("No reaction to the input");
            });
        }

        for file_output in file_outputs {
            file_output
                .output_run(scenario, idx, &processed)
                .await
                .context("Error processing an output step")?;
        }

        progress.inc(1);

        Ok(())
    }

    pub async fn run(self, device: &Device) -> anyhow::Result<()> {
        let (scenario, file_outputs) = self.prepare().await?;

        let progress = get_progressbar(&scenario);

        let mut stream = device
            .run_scenario(scenario.clone())
            .await
            .context("Scenario validation error")?
            .enumerate();

        self.output_init(&scenario, &progress);

        while let Some((idx, result)) = stream.next().await {
            // it's OK to just return the error because Device doesn't proceed after emitting
            // an error
            let recording = result?;
            self.output_step(&scenario, &progress, &file_outputs, idx, recording)
                .await?;
        }

        Ok(())
    }
}
