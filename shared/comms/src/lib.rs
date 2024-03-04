#![cfg_attr(not(test), no_std)]

use postcard::de_flavors::crc::from_bytes_u16;

// usbd_hid doesn't implement Deserialize and Eq/PartialEq, so here we use our own
// structs
#[derive(Debug, Eq, PartialEq, Copy, Clone, serde::Deserialize, serde::Serialize)]
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

#[derive(Debug, Eq, PartialEq, Copy, Clone, serde::Deserialize, serde::Serialize)]
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

#[derive(Debug, Eq, PartialEq, Copy, Clone, serde::Deserialize, serde::Serialize)]
pub enum HidReport {
    Mouse(MouseReport),
    Keyboard(KeyboardReport),
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, serde::Deserialize, serde::Serialize)]
pub enum HostToDevice {
    GetStatus,
    // reset needs reports too to make sure that the light level comes back to the baseline
    SendHidEvent { hid_event: u8, max_duration_ms: u32 },
    MeasureBackground { duration_ms: u32 },
    // note: enum has to allocate space for the largest member, so it can't be included
    //       in the enum itself (regardless of allocation issues)
    UpdateFirmware { length: u32 },
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, serde::Deserialize, serde::Serialize)]
pub struct Version {
    hardware: u8,
    firmware: u32,
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, serde::Deserialize, serde::Serialize)]
pub struct Status {
    version: Version,
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, serde::Deserialize, serde::Serialize)]
pub enum DeviceToHost {
    // see https://docs.rs/embassy-time/latest/embassy_time/
    // and https://docs.rs/embassy-time/latest/embassy_time/struct.Instant.html
    LightLevel { tick: u64, light_level: u32 },
    HidReport { tick: u64, hid_report: HidReport },
    Status(Status),
}

pub struct CrcCobsAccumulator<const N: usize> {
    buf: [u8; N],
    idx: usize,
}

// an implementation of CRC32+COBS accummulator inspired by postcard's, as suggested here:
// https://github.com/jamesmunns/postcard/issues/117
// see https://github.com/jamesmunns/postcard/blob/393e18aeee3fe59872ad9231da225170c8296d56/src/accumulator.rs
// for more comments
#[derive(Debug)]
pub enum FeedResult<'a, T> {
    /// Consumed all data, still pending.
    Consumed,

    /// Buffer was filled. Contains remaining section of input, if any.
    OverFull(&'a [u8]),

    /// Reached end of chunk, but some part of decoding failed. Contains remaining section of
    /// input, if any.
    Error {
        error: postcard::Error,
        release: &'a [u8],
    },

    /// Reached end of chunk, but deserialization failed. Contains remaining section of input, if
    /// any.
    DeserError(&'a [u8]),

    /// Deserialization complete. Contains deserialized data and remaining section of input, if any.
    Success {
        /// Deserialize data.
        data: T,

        /// Remaining data left in the buffer after deserializing.
        remaining: &'a [u8],
    },
}

// 16 bits to make sure a random packet is ~guaranteed to be invalid
// CRC_16_KERMIT is also known as CRC-16-CCITT and seems to be pretty popular
const CRC_ALG: crc::Crc<u16> = crc::Crc::<u16>::new(&crc::CRC_16_KERMIT);

// todo: await and skip header bytes
// todo: document LTMT_packet_CRC_ZERO
impl<const N: usize> CrcCobsAccumulator<N> {
    /// Create a new accumulator.
    pub const fn new() -> Self {
        CrcCobsAccumulator {
            buf: [0; N],
            idx: 0,
        }
    }

    /// Appends data to the internal buffer and attempts to deserialize the accumulated data into
    /// `T`.
    #[inline]
    pub fn feed<'a, T>(&mut self, input: &'a [u8]) -> FeedResult<'a, T>
    where
        T: for<'de> serde::Deserialize<'de>,
    {
        self.feed_ref(input)
    }

    /// Appends data to the internal buffer and attempts to deserialize the accumulated data into
    /// `T`.
    ///
    /// This differs from feed, as it allows the `T` to reference data within the internal buffer, but
    /// mutably borrows the accumulator for the lifetime of the deserialization.
    /// If `T` does not require the reference, the borrow of `self` ends at the end of the function.
    pub fn feed_ref<'de, 'a, T>(&'de mut self, input: &'a [u8]) -> FeedResult<'a, T>
    where
        T: serde::Deserialize<'de>,
    {
        if input.is_empty() {
            return FeedResult::Consumed;
        }

        let zero_pos = input.iter().position(|&i| i == 0);

        if let Some(n) = zero_pos {
            // Yes! We have an end of message here.
            // Add one to include the zero in the "take" portion
            // of the buffer, rather than in "release".
            let (take, release) = input.split_at(n + 1);

            // Does it fit?
            if (self.idx + take.len()) <= N {
                // Yes, add to the array
                self.extend_unchecked(take);

                // here's where the meat of the difference with postcard starts
                let result = match cobs::decode_in_place(&mut self.buf[..self.idx]) {
                    Ok(uncobsed_len) => {
                        match from_bytes_u16(&self.buf[..uncobsed_len], CRC_ALG.digest()) {
                            Ok(t) => FeedResult::Success {
                                data: t,
                                remaining: release,
                            },
                            Err(error) => FeedResult::Error { error, release },
                        }
                    }
                    Err(_) => {
                        return FeedResult::Error {
                            error: postcard::Error::DeserializeBadEncoding,
                            release,
                        };
                    }
                };
                // ...and here is where it ends

                self.idx = 0;
                result
            } else {
                self.idx = 0;
                FeedResult::OverFull(release)
            }
        } else {
            // Does it fit?
            if (self.idx + input.len()) > N {
                // nope
                let new_start = N - self.idx;
                self.idx = 0;
                FeedResult::OverFull(&input[new_start..])
            } else {
                // yup!
                self.extend_unchecked(input);
                FeedResult::Consumed
            }
        }
    }

    /// Extend the internal buffer with the given input.
    ///
    /// # Panics
    ///
    /// Will panic if the input does not fit in the internal buffer.
    fn extend_unchecked(&mut self, input: &[u8]) {
        let new_end = self.idx + input.len();
        self.buf[self.idx..new_end].copy_from_slice(input);
        self.idx = new_end;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use postcard::ser_flavors::crc::to_slice_u16;

    #[test]
    fn test_basic_roundtrip() {
        let packet = DeviceToHost::HidReport {
            tick: 17,
            hid_report: HidReport::Mouse(MouseReport {
                buttons: 0,
                x: 15,
                y: 14,
                wheel: 2,
                pan: 0,
            }),
        };
        let buffer = &mut [0u8; 64];
        let crc_appended = to_slice_u16(&packet, buffer, CRC_ALG.digest()).unwrap();

        let cobs_buffer = &mut [0u8; 64];
        // +1 because try_encode doesn't actually add the default sentinel value of 0
        let cobs_len = cobs::try_encode(crc_appended, cobs_buffer).unwrap() + 1;

        let mut accumulator = CrcCobsAccumulator::<32>::new();
        let result = accumulator.feed::<DeviceToHost>(&cobs_buffer[..cobs_len]);
        match result {
            FeedResult::Success { data, remaining } => {
                assert_eq!(packet, data);
                assert_eq!(remaining.len(), 0);
            }
            other => panic!("unexpected result: {other:?}"),
        }
    }

    // todo: quickcheck test for the roundtrip
    // todo: check that arbitrary prefixes are ignored
}
