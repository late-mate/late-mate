use crate::comms::hid::HidRequest;
use crate::MAX_SCENARIO_LENGTH;
use postcard::experimental::max_size::MaxSize;

// All enums are repr(u8) to minimise size (default is isize = 4 bytes on the MCU)
// All enums have explicit discriminants to make reverse compatibility simpler
// I considered making enums non_exhaustive, but I actually want compile time exhaustiveness
// checks, and postcard seemingly won't be able to deal with unknown enum variants
// see https://github.com/jamesmunns/postcard/issues/75

pub type RequestId = u32;

#[repr(u8)]
#[derive(
    Debug, Eq, PartialEq, Copy, Clone, serde::Deserialize, serde::Serialize, MaxSize, defmt::Format,
)]
pub enum ScenarioStep {
    HidRequest(HidRequest) = 0,
    Wait { ms: u16 } = 1,
}

#[derive(
    Debug, Eq, PartialEq, Clone, serde::Deserialize, serde::Serialize, MaxSize, defmt::Format,
)]
pub struct Scenario {
    /// Index into the scenario vector (None if no measurement is needed)
    pub start_recording_at_idx: Option<u8>,
    pub steps: heapless::Vec<ScenarioStep, { MAX_SCENARIO_LENGTH }>,
}

#[repr(u8)]
#[derive(
    Debug, Eq, PartialEq, Clone, serde::Deserialize, serde::Serialize, MaxSize, defmt::Format,
)]
pub enum Message {
    // not 0 to make it less likely to trigger by accident
    ResetToFirmwareUpdate = 254,
    GetStatus = 0,
    // can be called repeatedly with overlapping durations, works as a keepalive
    StreamLightLevel { duration_ms: u16 } = 1,
    SendHidReport(HidRequest) = 2,
    ExecuteScenario(Scenario) = 3,
}

#[derive(
    Debug, Eq, PartialEq, Clone, serde::Deserialize, serde::Serialize, MaxSize, defmt::Format,
)]
pub struct Envelope {
    /// Unique request ID that is then returned back and can be used to correlate request/response.
    /// Reused for streamed responses.
    pub request_id: RequestId,
    /// Request content
    pub request: Message,
}
