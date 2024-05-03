use crate::comms::hid::HidRequest;
use postcard::experimental::max_size::MaxSize;

#[derive(Debug, Eq, PartialEq, Copy, Clone, serde::Deserialize, serde::Serialize, MaxSize)]
pub struct MeasureFollowup {
    pub after_ms: u16,
    pub hid_request: HidRequest,
}

#[derive(Debug, Eq, PartialEq, Clone, serde::Deserialize, serde::Serialize, MaxSize)]
pub enum HostToDevice {
    ResetToFirmwareUpdate,
    GetStatus,
    // can be called repeatedly with overlapping duration, works as a keepalive
    MeasureBackground {
        duration_ms: u16,
    },
    SendHidReport(HidRequest),
    Measure {
        // must be less than 1000
        duration_ms: u16,
        start: HidRequest,
        followup: Option<MeasureFollowup>,
    },
    // revert is a separate scenario
    ExecuteScenario(heapless::Vec<ScenarioStep, 16>),
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, serde::Deserialize, serde::Serialize, MaxSize)]
pub enum ScenarioStep {
    HidRequest(HidRequest),
    StartTiming,
    Wait,
}
