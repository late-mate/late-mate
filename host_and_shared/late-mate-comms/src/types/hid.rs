use postcard::experimental::max_size::MaxSize;

pub type HidRequestId = u32;

#[derive(Debug, Eq, PartialEq, Copy, Clone, serde::Deserialize, serde::Serialize, MaxSize)]
pub struct MouseReport {
    pub buttons: u8,
    pub x: i8,
    pub y: i8,
    pub wheel: i8,
    pub pan: i8,
}

impl MouseReport {
    pub fn to_usbd_hid(self) -> usbd_hid::descriptor::MouseReport {
        usbd_hid::descriptor::MouseReport::from(self)
    }
}

impl From<MouseReport> for usbd_hid::descriptor::MouseReport {
    fn from(report: MouseReport) -> Self {
        usbd_hid::descriptor::MouseReport {
            buttons: report.buttons,
            x: report.x,
            y: report.y,
            wheel: report.wheel,
            pan: report.pan,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, serde::Deserialize, serde::Serialize, MaxSize)]
pub struct KeyboardReport {
    pub modifier: u8,
    pub keycodes: [u8; 6],
}

impl KeyboardReport {
    pub fn to_usbd_hid(self) -> usbd_hid::descriptor::KeyboardReport {
        usbd_hid::descriptor::KeyboardReport::from(self)
    }
}

impl From<KeyboardReport> for usbd_hid::descriptor::KeyboardReport {
    fn from(report: KeyboardReport) -> Self {
        usbd_hid::descriptor::KeyboardReport {
            modifier: report.modifier,
            reserved: 0,
            leds: 0,
            keycodes: report.keycodes,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, serde::Deserialize, serde::Serialize, MaxSize)]
pub enum HidReport {
    Mouse(MouseReport),
    Keyboard(KeyboardReport),
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, serde::Deserialize, serde::Serialize, MaxSize)]
pub struct HidRequest {
    pub id: HidRequestId,
    pub report: HidReport,
}
