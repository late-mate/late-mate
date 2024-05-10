use postcard::experimental::max_size::MaxSize;

// All enums are repr(u8) to minimise size (default is isize = 4 bytes on the MCU)
// All enums have explicit discriminants to make reverse compatibility simpler
// I considered making enums non_exhaustive, but I actually want compile time exhaustiveness
// checks, and postcard seemingly won't be able to deal with unknown enum variants
// see https://github.com/jamesmunns/postcard/issues/75

/// It is only unique for the given HostToDevice request
pub type HidRequestId = u8;

#[derive(
    Debug, Eq, PartialEq, Copy, Clone, serde::Deserialize, serde::Serialize, MaxSize, defmt::Format,
)]
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

#[derive(
    Debug, Eq, PartialEq, Copy, Clone, serde::Deserialize, serde::Serialize, MaxSize, defmt::Format,
)]
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

#[repr(u8)]
#[derive(
    Debug, Eq, PartialEq, Copy, Clone, serde::Deserialize, serde::Serialize, MaxSize, defmt::Format,
)]
pub enum HidReport {
    Mouse(MouseReport) = 0,
    Keyboard(KeyboardReport) = 1,
}

#[derive(
    Debug, Eq, PartialEq, Copy, Clone, serde::Deserialize, serde::Serialize, MaxSize, defmt::Format,
)]
pub struct HidRequest {
    pub id: HidRequestId,
    pub report: HidReport,
}
