use crate::comms::hid::HidRequestId;
use crate::comms::host_to_device::RequestId;
use postcard::experimental::max_size::MaxSize;

// All enums are repr(u8) to minimise size (default is isize = 4 bytes on the MCU)
// All enums have explicit discriminants to make reverse compatibility simpler
// All enums are non_exhaustive because, again, reverse compatibility (apparently
// serde supports ignoring unknown variants on non_exhaustive enums, see
// https://github.com/serde-rs/serde/pull/2570)

#[derive(Debug, Eq, PartialEq, Copy, Clone, serde::Deserialize, serde::Serialize, MaxSize)]
pub struct Version {
    pub hardware: u8,
    pub firmware: u32,
}

#[non_exhaustive]
#[repr(u8)]
#[derive(Debug, Eq, PartialEq, Copy, Clone, serde::Deserialize, serde::Serialize, MaxSize)]
pub enum MeasurementEvent {
    LightLevel(u32) = 0,
    HidReport(HidRequestId) = 1,
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, serde::Deserialize, serde::Serialize, MaxSize)]
pub struct Measurement {
    pub microsecond: u32,
    pub event: MeasurementEvent,
}

#[non_exhaustive]
#[repr(u8)]
#[derive(Debug, Eq, PartialEq, Copy, Clone, serde::Deserialize, serde::Serialize, MaxSize)]
pub enum DeviceToHost {
    /// GetStatus response
    Status {
        version: Version,
        max_light_level: u32,
        serial_number: [u8; 8],
    } = 0,
    /// Streamed on request (except when measurements are taken)
    CurrentLightLevel(u32) = 1,
    /// Streamed from an internal buffer after scenario is complete
    BufferedMeasurement {
        measurement: Measurement,
        idx: u16,
        total: u16,
    } = 3,
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, serde::Deserialize, serde::Serialize, MaxSize)]
pub struct Envelope {
    /// The corresponding request's ID. Doesn't need to be unique, sreamed stuff like
    /// light levels or buffered measurements just stream with the same request_id
    pub request_id: RequestId,
    /// Response content. There is no good error representation, so the error type is just ()
    pub response: Result<Option<DeviceToHost>, ()>,
}
