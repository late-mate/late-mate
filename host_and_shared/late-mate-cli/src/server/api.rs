use crate::nice_hid;

pub struct Followup {
    after: u16,
    hid_report: nice_hid::HidReport,
}

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

pub enum ServerToClient {
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
