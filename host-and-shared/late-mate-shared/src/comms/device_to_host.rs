use crate::comms::hid::HidRequestId;
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
    pub serial_number: [u8; 8],
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
