use postcard::experimental::max_size::MaxSize;
use usbd_hid::descriptor::SerializedDescriptor;

// usbd_hid doesn't implement Deserialize and Eq/PartialEq, so here we use our own
// structs
#[derive(Debug, Eq, PartialEq, Copy, Clone, serde::Deserialize, serde::Serialize, MaxSize)]
pub struct MouseReport {
    pub buttons: u8,
    pub x: i8,
    pub y: i8,
    pub wheel: i8,
    pub pan: i8,
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
    pub reserved: u8,
    pub leds: u8,
    pub keycodes: [u8; 6],
}

impl From<KeyboardReport> for usbd_hid::descriptor::KeyboardReport {
    fn from(report: KeyboardReport) -> Self {
        usbd_hid::descriptor::KeyboardReport {
            modifier: report.modifier,
            reserved: report.reserved,
            leds: report.leds,
            keycodes: report.keycodes,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, serde::Deserialize, serde::Serialize, MaxSize)]
pub enum HidReport {
    Mouse(MouseReport),
    Keyboard(KeyboardReport),
}

impl HidReport {
    pub fn descriptor(&self) -> &'static [u8] {
        match self {
            HidReport::Mouse(_) => usbd_hid::descriptor::MouseReport::desc(),
            HidReport::Keyboard(_) => usbd_hid::descriptor::KeyboardReport::desc(),
        }
    }
}
