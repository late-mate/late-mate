use crate::comms::hid::HidRequest;
use postcard::experimental::max_size::MaxSize;

// All enums are repr(u8) to minimise size (default is isize = 4 bytes on the MCU)
// All enums have explicit discriminants to make reverse compatibility simpler
// All enums are non_exhaustive because, again, reverse compatibility (apparently
// serde supports ignoring unknown variants on non_exhaustive enums, see
// https://github.com/serde-rs/serde/pull/2570)

pub type RequestId = u32;

// non-exhaustive because CLI and firmware version can be different
#[non_exhaustive]
#[repr(u8)]
#[derive(Debug, Eq, PartialEq, Copy, Clone, serde::Deserialize, serde::Serialize, MaxSize)]
pub enum ScenarioStep {
    HidRequest(HidRequest) = 0,
    StartTiming = 1,
    Wait = 2,
}

#[non_exhaustive]
#[repr(u8)]
#[derive(Debug, Eq, PartialEq, Clone, serde::Deserialize, serde::Serialize, MaxSize)]
pub enum HostToDevice {
    // not 0 to make it less likely to trigger by accident
    ResetToFirmwareUpdate = 254,
    GetStatus = 0,
    // can be called repeatedly with overlapping durations, works as a keepalive
    StreamLightLevel { duration_ms: u16 } = 1,
    SendHidReport(HidRequest) = 2,
    ExecuteScenario(heapless::Vec<ScenarioStep, 16>) = 3,
}

#[derive(Debug, Eq, PartialEq, Clone, serde::Deserialize, serde::Serialize, MaxSize)]
pub struct Envelope {
    /// Unique request ID that is then returned back and can be used to correlate request/response.
    /// Reused for streamed responses.
    pub request_id: RequestId,
    /// Request content
    pub request: HostToDevice,
}
