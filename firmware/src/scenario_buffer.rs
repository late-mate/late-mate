use crate::MutexKind;
use defmt::error;
use embassy_sync::mutex::Mutex;
use embassy_time::Instant;
use heapless::Vec;
use late_mate_shared::comms::device_to_host;
use late_mate_shared::MAX_SCENARIO_DURATION_MS;
use static_cell::ConstStaticCell;

// 2khz measurements
const MAX_SCENARIO_SIZE: u64 = MAX_SCENARIO_DURATION_MS * 2;
// +10% slack
const BUFFER_SIZE: usize = (MAX_SCENARIO_SIZE + MAX_SCENARIO_SIZE / 10) as usize;

pub struct Buffer {
    pub started_at: Instant,
    pub data: Vec<Moment, BUFFER_SIZE>,
}

#[derive(Debug)]
pub struct Moment {
    pub microsecond: u32,
    pub event: device_to_host::Event,
}

/// Panics if it's called twice
pub fn init() -> &'static Mutex<MutexKind, Buffer> {
    // ConstStaticCell guarantees that the initialiser (Buffer::new()) never goes on the stack.
    // Normally creating a buffer and assigning it somewhere create the buffer on the stack
    // first, but ConstStaticCell is `const`-fueled, so this is guaranteed to not happen
    static BUFFER: ConstStaticCell<Mutex<MutexKind, Buffer>> =
        ConstStaticCell::new(Mutex::new(Buffer::new()));
    BUFFER.take()
}

impl Buffer {
    const fn new() -> Self {
        Self {
            started_at: Instant::MIN,
            data: Vec::new(),
        }
    }

    pub fn clear(&mut self, new_start: Instant) {
        self.data.clear();
        self.started_at = new_start;
    }

    /// Returns Error if the buffer or the time counter will overflow
    pub fn store(&mut self, happened_at: Instant, event: device_to_host::Event) -> Result<(), ()> {
        assert!(
            happened_at >= self.started_at,
            "Time travellers shouldn't use this code"
        );

        if self.data.len() >= (self.data.capacity() - 1) {
            error!("Can't push into the scenario buffer, it will overflow");
            return Err(());
        }

        let microsecond_u64 = (happened_at - self.started_at).as_micros();
        let microsecond = match u32::try_from(microsecond_u64) {
            Ok(x) => x,
            Err(_) => {
                error!(
                    "Time overflow while trying to push into the scenario buffer ({:?})",
                    microsecond_u64
                );
                return Err(());
            }
        };

        self.data
            .push(Moment { microsecond, event })
            .expect("Scenario buffer push shouldn't fail");

        Ok(())
    }
}
