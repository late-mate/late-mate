#[non_exhaustive]
#[derive(Debug, Eq, PartialEq, Clone, Copy, serde::Deserialize, serde::Serialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum MouseButton {
    Left = 0x01,
    Right = 0x02,
    Middle = 0x03,
}

#[derive(Debug, Eq, PartialEq, Clone, Default, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct MouseReport {
    pub buttons: Vec<MouseButton>,
    pub x: i8,
    pub y: i8,
    pub wheel: i8,
    pub pan: i8,
}

impl From<&MouseReport> for late_mate_comms::MouseReport {
    fn from(value: &MouseReport) -> Self {
        let mut byte_buttons = 0u8;
        for button in &value.buttons {
            byte_buttons |= *button as u8
        }

        late_mate_comms::MouseReport {
            buttons: byte_buttons,
            x: value.x,
            y: value.y,
            wheel: value.wheel,
            pan: value.pan,
        }
    }
}

// see https://gist.github.com/MightyPork/6da26e382a7ad91b5496ee55fdc73db2
#[derive(Debug, Eq, PartialEq, Clone, Copy, serde::Deserialize, serde::Serialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum KeyboardModifier {
    LCtrl = 0x01,
    LShift = 0x02,
    LAlt = 0x04,
    LMeta = 0x08,
    RCtrl = 0x10,
    RShift = 0x20,
    RAlt = 0x40,
    RMeta = 0x80,
}

// see https://docs.rs/usbd-hid/latest/usbd_hid/descriptor/enum.KeyboardUsage.html
// adjusted for readability
#[non_exhaustive]
#[derive(Debug, Eq, PartialEq, Clone, Copy, serde::Deserialize, serde::Serialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum Keycode {
    A = 4,
    B = 5,
    C = 6,
    D = 7,
    E = 8,
    F = 9,
    G = 10,
    H = 11,
    I = 12,
    J = 13,
    K = 14,
    L = 15,
    M = 16,
    N = 17,
    O = 18,
    P = 19,
    Q = 20,
    R = 21,
    S = 22,
    T = 23,
    U = 24,
    V = 25,
    W = 26,
    X = 27,
    Y = 28,
    Z = 29,
    OneExclamation = 30,
    TwoAt = 31,
    ThreeHash = 32,
    FourDollar = 33,
    FivePercent = 34,
    SixCaret = 35,
    SevenAmpersand = 36,
    EightAsterisk = 37,
    NineOpenParens = 38,
    ZeroCloseParens = 39,
    Enter = 40,
    Escape = 41,
    Backspace = 42,
    Tab = 43,
    Spacebar = 44,
    DashUnderscore = 45,
    EqualPlus = 46,
    OpenBracketBrace = 47,
    CloseBracketBrace = 48,
    BackslashBar = 49,
    NonUSHash = 50,
    SemiColon = 51,
    SingleDoubleQuote = 52,
    BacktickTilde = 53,
    CommaLess = 54,
    PeriodGreater = 55,
    SlashQuestion = 56,
    CapsLock = 57,
    F1 = 58,
    F2 = 59,
    F3 = 60,
    F4 = 61,
    F5 = 62,
    F6 = 63,
    F7 = 64,
    F8 = 65,
    F9 = 66,
    F10 = 67,
    F11 = 68,
    F12 = 69,
    PrintScreen = 70,
    ScrollLock = 71,
    Pause = 72,
    Insert = 73,
    Home = 74,
    PageUp = 75,
    Delete = 76,
    End = 77,
    PageDown = 78,
    Right = 79,
    Left = 80,
    Down = 81,
    Up = 82,
    Mute = 127,
    VolumeUp = 128,
    VolumeDown = 129,
}

#[derive(Debug, Eq, PartialEq, Clone, Default, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct KeyboardReport {
    pub modifiers: Vec<KeyboardModifier>,
    // at most six keycodes are actually used
    // todo: think more about the API design here. can it be more transparent?
    pub keycodes: Vec<Keycode>,
}

impl From<&KeyboardReport> for late_mate_comms::KeyboardReport {
    fn from(value: &KeyboardReport) -> Self {
        let mut byte_modifier = 0u8;
        for modifier in &value.modifiers {
            byte_modifier |= *modifier as u8
        }

        let mut byte_keycodes = [0u8; 6];
        for (i, keycode) in value.keycodes.iter().take(6).enumerate() {
            byte_keycodes[i] = *keycode as u8;
        }

        late_mate_comms::KeyboardReport {
            modifier: byte_modifier,
            keycodes: byte_keycodes,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, serde::Deserialize, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HidReport {
    Mouse(MouseReport),
    Keyboard(KeyboardReport),
}

impl From<&HidReport> for late_mate_comms::HidReport {
    fn from(value: &HidReport) -> Self {
        match value {
            HidReport::Mouse(report) => late_mate_comms::HidReport::Mouse(report.into()),
            HidReport::Keyboard(report) => late_mate_comms::HidReport::Keyboard(report.into()),
        }
    }
}
