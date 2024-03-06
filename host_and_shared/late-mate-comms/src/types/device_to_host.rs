use crate::types::hid::HidReport;
use postcard::experimental::max_size::MaxSize;

#[derive(Debug, Eq, PartialEq, Copy, Clone, serde::Deserialize, serde::Serialize, MaxSize)]
pub struct Version {
    pub hardware: u8,
    pub firmware: u32,
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, serde::Deserialize, serde::Serialize, MaxSize)]
pub struct Status {
    pub version: Version,
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, serde::Deserialize, serde::Serialize, MaxSize)]
pub enum DeviceToHost {
    // see https://docs.rs/embassy-time/latest/embassy_time/
    // and https://docs.rs/embassy-time/latest/embassy_time/struct.Instant.html
    LightLevel { tick: u64, light_level: u32 },
    HidReport { tick: u64, hid_report: HidReport },
    Status(Status),
}
