use crate::nice_hid;

#[derive(Debug, Eq, PartialEq, Clone, serde::Deserialize, serde::Serialize, ts_rs::TS)]
pub struct Followup {
    after: u16,
    hid_report: nice_hid::HidReport,
}

#[derive(Debug, Eq, PartialEq, Clone, serde::Deserialize, serde::Serialize, ts_rs::TS)]
#[ts(export)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientToServer {
    Status,
    StartMonitoring,
    StopMonitoring,
    SendHidReport {
        hid_report: nice_hid::HidReport,
    },
    Measure {
        duration: u16,
        start: nice_hid::HidReport,
        followup: Option<Followup>,
    },
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, serde::Deserialize, serde::Serialize, ts_rs::TS)]
pub struct Version {
    pub hardware: u8,
    pub firmware: u32,
}

#[derive(Debug, Eq, PartialEq, Clone, serde::Deserialize, serde::Serialize, ts_rs::TS)]
#[ts(export)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerToClient {
    Status {
        version: Version,
        max_light_level: u32,
    },
    BackgroundValues {
        max_light_level: u32,
        light_levels: Vec<u32>,
    },
    Measurement {
        max_light_level: u32,
        /// microsecond | light level; None = HID event
        levels: Vec<(u32, Option<u32>)>,
        change_us: u32,
        delay_us: u32,
    },
}
