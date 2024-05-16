use crate::hid;
use late_mate_shared::comms;
use late_mate_shared::comms::device_to_host;
use late_mate_shared::comms::host_to_device;
use late_mate_shared::{heapless, MAX_SCENARIO_DURATION_MS, MAX_SCENARIO_LENGTH};
use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Total length of the test section must be less than or equal to {MAX_SCENARIO_LENGTH}, got {0} steps")]
    TestTooLarge(usize),
    #[error("Total length of the revert section must be less than or equal to {MAX_SCENARIO_LENGTH}, got {0} steps")]
    ReverseTooLarge(usize),
    #[error("Total duration of the test section must be less than {MAX_SCENARIO_DURATION_MS}ms, got {ms}ms")]
    TestTooLong { ms: u64 },
    #[error("Test section must include start_timing")]
    NoStartTiming,
    #[error("Test section must include only one start_timing")]
    MultipleStartTiming,
    #[error("Revert section must not include start_timing")]
    StartTimingInRevert,
    #[error("Random delay range start must be less than or equal than its end")]
    InvalidDelayRange,
}

#[derive(Debug, Eq, PartialEq, Clone, serde::Deserialize, serde::Serialize, ts_rs::TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ScenarioStep {
    Wait { ms: u16 },
    HidReport(hid::HidReport),
    StartTiming,
}

impl From<&ScenarioStep> for Duration {
    fn from(value: &ScenarioStep) -> Self {
        let total_ms = match value {
            ScenarioStep::Wait { ms } => *ms,
            // it takes 1ms to send a HID report + margin of error
            ScenarioStep::HidReport(_) => 2,
            ScenarioStep::StartTiming => 0,
        };
        Duration::from_millis(total_ms as u64)
    }
}

#[derive(Debug, Eq, PartialEq, Clone, serde::Deserialize, serde::Serialize, ts_rs::TS)]
#[serde(default, deny_unknown_fields)]
pub struct Scenario {
    pub test: Vec<ScenarioStep>,
    pub revert: Option<Vec<ScenarioStep>>,
    pub repeats: u64,
    pub delay_between_ms: (u64, u64),
}

impl Scenario {
    pub fn test_duration(&self) -> Duration {
        self.test.iter().map(Duration::from).sum()
    }

    pub fn total_duration(&self) -> (Duration, Duration) {
        let revert_duration = self
            .revert
            .as_ref()
            .map_or(Duration::default(), |r| r.iter().map(Duration::from).sum());

        let repeats = u32::try_from(self.repeats).expect("Repeats should fit into u32");
        let base = (revert_duration + self.test_duration()) * repeats;

        (
            base + Duration::from_millis(self.delay_between_ms.0) * repeats,
            base + Duration::from_millis(self.delay_between_ms.1) * repeats,
        )
    }

    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.test.len() > MAX_SCENARIO_LENGTH {
            return Err(ValidationError::TestTooLarge(self.test.len()));
        }

        if let Some(revert) = &self.revert {
            if revert.len() > MAX_SCENARIO_LENGTH {
                return Err(ValidationError::ReverseTooLarge(revert.len()));
            }
        }

        let test_duration_ms = u64::try_from(self.test_duration().as_millis())
            .expect("Test duration must fit into u64");
        if test_duration_ms > MAX_SCENARIO_DURATION_MS {
            return Err(ValidationError::TestTooLong {
                ms: test_duration_ms,
            });
        }

        let test_num_starts = self
            .test
            .iter()
            .filter(|s| matches!(s, ScenarioStep::StartTiming))
            .count();
        if test_num_starts == 0 {
            return Err(ValidationError::NoStartTiming);
        } else if test_num_starts > 1 {
            return Err(ValidationError::MultipleStartTiming);
        }

        if let Some(revert) = &self.revert {
            if revert
                .iter()
                .any(|s| matches!(s, ScenarioStep::StartTiming))
            {
                return Err(ValidationError::StartTimingInRevert);
            }
        }

        if self.delay_between_ms.0 > self.delay_between_ms.1 {
            return Err(ValidationError::InvalidDelayRange);
        }

        Ok(())
    }
}

impl Default for Scenario {
    fn default() -> Self {
        Self {
            test: vec![],
            revert: None,
            repeats: 50,
            delay_between_ms: (100, 1000),
        }
    }
}

pub fn to_device_scenario(
    steps: &[ScenarioStep],
) -> (host_to_device::Scenario, Vec<hid::HidReport>) {
    let mut start_recording_at_idx = None;
    let mut device_steps =
        heapless::Vec::<host_to_device::ScenarioStep, MAX_SCENARIO_LENGTH>::new();
    let mut hid_report_index = Vec::new();

    // this justifies .unwrap()s below
    assert!(
        steps.len() <= MAX_SCENARIO_LENGTH,
        "Maximum scenario length exceeded (non-validated scenario?)"
    );

    for (idx, s) in steps.iter().enumerate() {
        match s {
            ScenarioStep::Wait { ms } => {
                device_steps
                    .push(host_to_device::ScenarioStep::Wait { ms: *ms })
                    .unwrap();
            }
            ScenarioStep::HidReport(report) => {
                let id = u8::try_from(hid_report_index.len()).unwrap();
                hid_report_index.push(report.to_owned());

                let hid_request = comms::hid::HidRequest {
                    id,
                    report: report.into(),
                };
                device_steps
                    .push(host_to_device::ScenarioStep::HidRequest(hid_request))
                    .unwrap();
            }
            ScenarioStep::StartTiming => {
                start_recording_at_idx = Some(u8::try_from(idx).unwrap());
            }
        }
    }

    (
        host_to_device::Scenario {
            start_recording_at_idx,
            steps: device_steps,
        },
        hid_report_index,
    )
}

// note that it's different from shared comms stuff becauase it has the actual report,
// not just the ID
pub enum Event {
    LightLevel(u32),
    HidReport(hid::HidReport),
}

impl Event {
    pub fn from_device(device_event: device_to_host::Event, hid_index: &[hid::HidReport]) -> Self {
        match device_event {
            device_to_host::Event::LightLevel(x) => Self::LightLevel(x),
            device_to_host::Event::HidReport(id) => {
                Self::HidReport(hid_index[id as usize].to_owned())
            }
        }
    }
}

pub struct Moment {
    pub microsecond: u32,
    pub event: Event,
}

impl Moment {
    pub fn from_device(
        device_moment: device_to_host::BufferedMoment,
        hid_index: &[hid::HidReport],
    ) -> Self {
        Self {
            microsecond: device_moment.microsecond,
            event: Event::from_device(device_moment.event, hid_index),
        }
    }
}

pub struct Recording {
    pub max_light_level: u32,
    pub timeline: Vec<Moment>,
    // todo: enrich with statistics
}
