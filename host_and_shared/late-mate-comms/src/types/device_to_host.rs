use crate::types::hid::HidRequestId;
use postcard::experimental::max_size::MaxSize;

#[derive(Debug, Eq, PartialEq, Copy, Clone, serde::Deserialize, serde::Serialize, MaxSize)]
pub struct Version {
    pub hardware: u8,
    pub firmware: u32,
}

// todo: inline this?
#[derive(Debug, Eq, PartialEq, Copy, Clone, serde::Deserialize, serde::Serialize, MaxSize)]
pub struct Status {
    pub version: Version,
    pub max_light_level: u32,
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, serde::Deserialize, serde::Serialize, MaxSize)]
pub enum MeasurementEvent {
    LightLevel(u32),
    HidReport(HidRequestId),
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, serde::Deserialize, serde::Serialize, MaxSize)]
pub struct Measurement {
    pub microsecond: u32,
    pub event: MeasurementEvent,
}

// u32 = 4 bytes for the microsecond; 1 byte for the enum tag; 4 bytes for the datapoint
// = 9 bytes per datapoint; 2khz = 2000 * 9 ~ 18kb of internal buffer for 1 second of data,
// which should fit no problem (RPi has 264kb of RAM)

#[derive(Debug, Eq, PartialEq, Copy, Clone, serde::Deserialize, serde::Serialize, MaxSize)]
pub enum DeviceToHost {
    /// GetStatus response
    Status(Status),
    /// Streamed on request (except when measurements are taken)
    CurrentLightLevel(u32),
    /// Just an acknowledgement
    HidReportSent(HidRequestId),
    /// Those are sent after a measurement is requested, streaming from an internal buffer
    BufferedMeasurement {
        measurement: Measurement,
        idx: u16,
        total: u16,
    },
}
