use usbd_hid::descriptor::{KeyboardReport, MouseReport};

// usbd_hid doesn't implement Deserialize, so I have to use this trick:
// https://serde.rs/remote-derive.html
#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(remote="MouseReport")]
pub struct MouseReportRef {
    pub buttons: u8,
    pub x: i8,
    pub y: i8,
    pub wheel: i8,
    pub pan: i8,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(remote="KeyboardReport")]
pub struct KeyboardReportRef {
    pub modifier: u8,
    pub reserved: u8,
    pub leds: u8,
    pub keycodes: [u8; 6],
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub enum HidReport {
    Mouse(#[serde(with="MouseReportRef")] MouseReport),
    Keyboard(#[serde(with="KeyboardReportRef")] KeyboardReport),
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub enum HostToDevice {
    GetStatus,
    // reset needs reports too to make sure that the light level comes back to the baseline
    SendHidEvent { hid_event: u8, max_duration_ms: u32 },
    MeasureBackground { duration_ms: u32 },
    // note: enum has to allocate space for the largest member, so it can't be included
    //       in the enum itself (regardless of allocation issues)
    UpdateFirmware { length: u32 },
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Version {
    hardware: u8,
    firmware: u32,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Status {
    version: Version,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub enum DeviceToHost {
    // see https://docs.rs/embassy-time/latest/embassy_time/
    // and https://docs.rs/embassy-time/latest/embassy_time/struct.Instant.html
    LightLevel { tick: u64, light_level: u32 },
    HidReport { tick: u64, hid_report: HidReport },
    Status(Status),
}
