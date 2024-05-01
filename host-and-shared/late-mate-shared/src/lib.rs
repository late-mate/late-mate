#![cfg_attr(all(not(test), not(feature = "std")), no_std)]

mod types;

pub use crate::types::device_to_host::{
    DeviceToHost, Measurement, MeasurementEvent, Status, Version,
};
pub use crate::types::hid::{HidReport, HidRequest, HidRequestId, KeyboardReport, MouseReport};
pub use crate::types::host_to_device::{HostToDevice, MeasureFollowup};
use postcard::de_flavors::crc::from_bytes_u16;
use postcard::experimental::max_size::MaxSize;
use postcard::ser_flavors::crc::to_slice_u16;

// VID/PID pair is allocated for Late Mate
// see https://github.com/raspberrypi/usb-pid
pub const USB_VID: u16 = 0x2E8A;
pub const USB_PID: u16 = 0x108B;

const fn max(a: usize, b: usize) -> usize {
    [a, b][(a < b) as usize]
}

// CRC16 adds 2 bytes (duh)
// COBS adds 2 bytes: 1 byte of overhead + sentinel (0)
const BUFFER_OVERHEAD: usize = 4;
pub const MAX_BUFFER_SIZE: usize = max(
    HostToDevice::POSTCARD_MAX_SIZE,
    DeviceToHost::POSTCARD_MAX_SIZE,
) + BUFFER_OVERHEAD;

pub struct CrcCobsAccumulator {
    buf: [u8; MAX_BUFFER_SIZE],
    idx: usize,
}

// This is pulled out from the firmware to be sure that the value is validated in the CLI.
// It can't use std::time::Duration because it's used in no_std in the firmware
pub const MAX_SCENARIO_DURATION_MS: u64 = 5000;

// an implementation of CRC32+COBS accummulator inspired by postcard's, as suggested here:
// https://github.com/jamesmunns/postcard/issues/117
// see https://github.com/jamesmunns/postcard/blob/393e18aeee3fe59872ad9231da225170c8296d56/src/accumulator.rs
// for more comments
#[derive(Debug)]
pub enum FeedResult<'a, T> {
    /// Consumed all data, still pending.
    Consumed,

    /// Buffer was filled. Contains remaining section of input, if any.
    OverFull { remaining: &'a [u8] },

    /// Reached end of chunk, but some part of decoding failed. Contains remaining section of
    /// input, if any.
    Error {
        error: postcard::Error,
        remaining: &'a [u8],
    },

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

// todo: expect and skip header bytes
// todo: document LTMT_COBS(packet_CRC)_ZERO and adjust BUFFER_SIZE consts
impl CrcCobsAccumulator {
    /// Create a new accumulator.
    pub const fn new() -> Self {
        CrcCobsAccumulator {
            // todo: I'm not sure that this is 100% correct, recheck/rethink
            buf: [0; MAX_BUFFER_SIZE],
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
            let (taken, remaining) = input.split_at(n + 1);

            // Does it fit?
            if (self.idx + taken.len()) <= MAX_BUFFER_SIZE {
                // Yes, add to the array
                self.extend_unchecked(taken);

                // here's where the meat of the difference with postcard starts
                let result = match cobs::decode_in_place(&mut self.buf[..self.idx]) {
                    Ok(uncobsed_len) => {
                        match from_bytes_u16(&self.buf[..uncobsed_len], CRC_ALG.digest()) {
                            Ok(data) => FeedResult::Success { data, remaining },
                            Err(error) => FeedResult::Error { error, remaining },
                        }
                    }
                    Err(()) => {
                        return FeedResult::Error {
                            error: postcard::Error::DeserializeBadEncoding,
                            remaining,
                        };
                    }
                };
                // ...and here is where it ends

                self.idx = 0;
                result
            } else {
                self.idx = 0;
                FeedResult::OverFull { remaining }
            }
        } else {
            // Does it fit?
            if (self.idx + input.len()) > MAX_BUFFER_SIZE {
                // nope
                let new_start = MAX_BUFFER_SIZE - self.idx;
                self.idx = 0;
                FeedResult::OverFull {
                    remaining: &input[new_start..],
                }
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

// todo: think a bit more about errors
pub fn encode<T: serde::Serialize + MaxSize>(msg: &T, result_buffer: &mut [u8]) -> usize {
    let buffer = &mut [0u8; MAX_BUFFER_SIZE];
    let crc_appended = to_slice_u16(msg, buffer, CRC_ALG.digest()).unwrap();

    // +1 because try_encode doesn't actually add the default sentinel value of 0
    cobs::try_encode(crc_appended, result_buffer).unwrap() + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_roundtrip() {
        let packet = DeviceToHost::BufferedMeasurement {
            measurement: Measurement {
                microsecond: 17,
                event: MeasurementEvent::LightLevel(42),
            },
            idx: 5,
            total: 10,
        };
        let buffer = &mut [0u8; MAX_BUFFER_SIZE];
        let cobs_len = encode(&packet, buffer);

        let mut accumulator = CrcCobsAccumulator::new();
        let result = accumulator.feed::<DeviceToHost>(&buffer[..cobs_len]);
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
    // todo: fuzz test
}
