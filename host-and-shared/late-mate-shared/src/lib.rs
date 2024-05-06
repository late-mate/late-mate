#![cfg_attr(all(not(test), not(feature = "std")), no_std)]

pub mod comms;

// VID/PID pair is allocated for Late Mate
// see https://github.com/raspberrypi/usb-pid
pub const USB_VID: u16 = 0x2E8A;
pub const USB_PID: u16 = 0x108B;

// This is pulled out from the firmware to be sure that the value is validated in the CLI.
// It can't use std::time::Duration because it's used in no_std in the firmware.
// As for the size: u32 = 4 bytes for the microsecond; 1 byte for the enum tag;
// 4 bytes for the datapoint = 9 bytes per datapoint;
// 2khz = 2000 * 9 ~ 18kb of internal buffer for 1 second of data (~100kb per 5sec),
// which should fit no problem (RPi has 264kb of RAM)
pub const MAX_SCENARIO_DURATION_MS: u64 = 5000;
