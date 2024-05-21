use crate::statistics::ProcessedRecording;
use anyhow::{anyhow, Context};
use late_mate_device::hid::HidReport;
use late_mate_device::scenario::{Event, Moment, Recording, Scenario};
use std::path::{Path, PathBuf};
use tokio::fs::{create_dir, File, OpenOptions};
use tokio::io::AsyncWriteExt;

#[derive(Debug, serde::Serialize)]
struct JsonRunFile<'a> {
    run_name: &'a str,
    idx: usize,
    changepoint_microsecond: Option<u32>,
    #[serde(flatten)]
    recording: &'a Recording,
}

#[derive(Debug, serde::Serialize)]
struct CsvTimelineFileRow<'a> {
    pub microsecond: u32,
    pub light_level: Option<u32>,
    pub usb_event: Option<&'a str>,
}

#[derive(Debug, serde::Serialize)]
struct CsvChangepointFileRow {
    pub run_idx: usize,
    pub changepoint_microsecond: Option<u32>,
}

pub enum FileOutputKind {
    Csv,
    Json,
}

pub struct FileOutput {
    kind: FileOutputKind,
    run_name: String,
    run_dir: PathBuf,
}

impl FileOutput {
    pub async fn prepare(
        kind: FileOutputKind,
        output_dir: &Path,
        run_name: &str,
    ) -> anyhow::Result<Self> {
        let output_dir_s = output_dir.to_string_lossy();
        if output_dir
            .try_exists()
            .with_context(|| format!("Error checking \"{output_dir_s}\""))?
        {
            if !output_dir.is_dir() {
                return Err(anyhow!("\"{output_dir_s}\" is not a directory"));
            }
        } else {
            create_dir(&output_dir)
                .await
                .with_context(|| format!("Error creating \"{output_dir_s}\""))?;
        }

        let run_dir = output_dir.join(run_name);
        let run_dir_s = run_dir.to_string_lossy();

        if run_dir
            .try_exists()
            .with_context(|| format!("Error checking \"{run_dir_s}\""))?
        {
            if !run_dir.is_dir() {
                return Err(anyhow!("\"{run_dir_s}\" exists and is not a directory"));
            }

            if run_dir
                .read_dir()
                .with_context(|| format!("Error reading \"{run_dir_s}\" as a directory"))?
                .next()
                .is_some()
            {
                return Err(anyhow!("\"{run_dir_s}\" exists and is not empty"));
            }
        } else {
            create_dir(&run_dir)
                .await
                .with_context(|| format!("Error creating \"{run_dir_s}\""))?;
        }

        Ok(Self {
            kind,
            run_dir,
            run_name: run_name.to_owned(),
        })
    }

    async fn output_run_json(
        &self,
        idx: usize,
        processed_recording: &ProcessedRecording,
        filename_base: &str,
    ) -> anyhow::Result<()> {
        let path = self.run_dir.join(format!("{filename_base}.json"));
        let path_s = path.to_string_lossy();

        let mut file = File::create(&path)
            .await
            .with_context(|| format!("Error creating an output file at {path_s}"))?;

        let record = JsonRunFile {
            run_name: &self.run_name,
            idx,
            changepoint_microsecond: processed_recording.changepoint_us,
            recording: &processed_recording.recording,
        };
        let serialised =
            serde_json::to_string_pretty(&record).context("Error serialising the test run")?;
        file.write_all(serialised.as_bytes())
            .await
            .with_context(|| format!("Error writing to {path_s}"))?;

        Ok(())
    }

    async fn output_run_csv_changepoint(
        &self,
        idx: usize,
        processed_recording: &ProcessedRecording,
    ) -> anyhow::Result<()> {
        let path = self.run_dir.join("_changepoints.csv");
        let new_file = path.exists();
        let mut file = if new_file {
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .await
        } else {
            File::create(&path).await
        }
        .with_context(|| format!("Error opening \"{}\"", path.to_string_lossy()))?;

        let row = CsvChangepointFileRow {
            run_idx: idx,
            changepoint_microsecond: processed_recording.changepoint_us,
        };
        let mut row_bytes = Vec::new();
        {
            let mut csv_writer = csv::WriterBuilder::new()
                .has_headers(!new_file)
                .from_writer(&mut row_bytes);

            csv_writer
                .serialize(row)
                .context("Error serialising a CSV row")?;
            csv_writer.flush().context("CSV error")?;
        };

        file.write_all(row_bytes.as_slice())
            .await
            .context("Error writing a CSV row")?;

        Ok(())
    }

    async fn output_run_csv_timeline(
        &self,
        processed_recording: &ProcessedRecording,
        filename_base: &str,
    ) -> anyhow::Result<()> {
        let path = self.run_dir.join(format!("timeline.{filename_base}.csv"));
        let mut file = File::create(&path)
            .await
            .with_context(|| format!("Error opening \"{}\"", path.to_string_lossy()))?;

        // Buffering the entire thing because it's not too big, and I don't want to do a sync
        // file operation here
        let mut bytes = Vec::<u8>::new();
        {
            let mut csv_writer = csv::Writer::from_writer(&mut bytes);
            for Moment { microsecond, event } in &processed_recording.recording.timeline {
                let microsecond = *microsecond;
                let row = match event {
                    Event::LightLevel(l) => CsvTimelineFileRow {
                        microsecond,
                        light_level: Some(*l),
                        usb_event: None,
                    },
                    Event::HidReport(report) => {
                        let usb_event = match report {
                            HidReport::Mouse(_) => "mouse_report",
                            HidReport::Keyboard(_) => "keyboard_report",
                        };
                        CsvTimelineFileRow {
                            microsecond,
                            light_level: None,
                            usb_event: Some(usb_event),
                        }
                    }
                };
                csv_writer
                    .serialize(row)
                    .context("Error serialising a CSV row")?;
            }
            csv_writer.flush().context("CSV error")?;
        };

        file.write_all(bytes.as_slice())
            .await
            .context("Error writing a CSV file")?;

        Ok(())
    }

    async fn output_run_csv(
        &self,
        idx: usize,
        processed_recording: &ProcessedRecording,
        filename_base: &str,
    ) -> anyhow::Result<()> {
        self.output_run_csv_changepoint(idx, processed_recording)
            .await?;
        self.output_run_csv_timeline(processed_recording, filename_base)
            .await?;

        Ok(())
    }

    pub async fn output_run(
        &self,
        scenario: &Scenario,
        idx: usize,
        processed_recording: &ProcessedRecording,
    ) -> anyhow::Result<()> {
        let width =
            usize::try_from(scenario.repeats.ilog10() + 1).expect("File width must fit into usize");
        let filename_base = format!("{idx:0width$}");

        match self.kind {
            FileOutputKind::Csv => {
                self.output_run_csv(idx, processed_recording, &filename_base)
                    .await
            }
            FileOutputKind::Json => {
                self.output_run_json(idx, processed_recording, &filename_base)
                    .await
            }
        }
    }
}
