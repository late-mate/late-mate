/// Used by Windows to identify USB device interface, see
/// https://github.com/embassy-rs/embassy/blob/9cbbedef793d619c659c6a81080675282690a8af/examples/rp/src/bin/usb_raw_bulk.rs#L42
/// for an example
/// Randomly generated
pub const GUID: &str = "{561C0186-B13E-41A1-AC0F-57B52F640043}";

// 0 is the first/default, but they are extracted/checked just in case
pub const NUMBER: u8 = 0;
pub const ALT_SETTING_NUMBER: u8 = 0;
pub const ENDPOINT_INDEX: u8 = 1;
pub const PACKET_SIZE: usize = 64;
