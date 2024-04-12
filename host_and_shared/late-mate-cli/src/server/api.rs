use crate::nice_hid;

#[derive(Debug, Eq, PartialEq, Clone, serde::Deserialize, serde::Serialize, ts_rs::TS)]
pub struct Followup {
    pub after_ms: u16,
    pub hid_report: nice_hid::HidReport,
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
        before: Vec<nice_hid::HidReport>,
        duration_ms: u16,
        start: nice_hid::HidReport,
        followup: Option<Followup>,
        after: Vec<nice_hid::HidReport>,
    },
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, serde::Deserialize, serde::Serialize, ts_rs::TS)]
pub struct Version {
    pub hardware: u8,
    pub firmware: u32,
}

#[derive(Debug, PartialEq, Clone, serde::Deserialize, serde::Serialize, ts_rs::TS)]
#[ts(export)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerToClient {
    Status {
        version: Version,
        max_light_level: u32,
    },
    BackgroundLightLevel {
        avg: f64,
    },
    Measurement {
        max_light_level: u32,
        /// microsecond, light level
        light_levels: Vec<(u32, u32)>,
        followup_hid_us: Option<u32>,
        change_us: Option<u32>,
    },
}
