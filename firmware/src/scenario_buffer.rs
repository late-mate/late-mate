use crate::RawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::Instant;
use heapless::Vec;
use late_mate_shared::{Measurement, MeasurementEvent, MAX_SCENARIO_DURATION_MS};
use static_cell::ConstStaticCell;

// 2khz measurements
const MAX_SCENARIO_SIZE: u64 = MAX_SCENARIO_DURATION_MS * 2;
// +10% slack
const BUFFER_SIZE: usize = (MAX_SCENARIO_SIZE + MAX_SCENARIO_SIZE / 10) as usize;

pub struct Buffer {
    pub started_at: Instant,
    pub measurements: Vec<Measurement, BUFFER_SIZE>,
}

/// Panics if it's called twice
pub fn init() -> &'static Mutex<RawMutex, Buffer> {
    // ConstStaticCell guarantees that the initialiser (Buffer::new()) never goes on the stack.
    // Normally creating a buffer and assigning it somewhere create the buffer on the stack
    // first, but ConstStaticCell is `const`-fueled, so this is guaranteed to not happen
    static BUFFER: ConstStaticCell<Mutex<RawMutex, Buffer>> =
        ConstStaticCell::new(Mutex::new(Buffer::new()));
    BUFFER.take()
}

impl Buffer {
    const fn new() -> Self {
        Self {
            started_at: Instant::MIN,
            measurements: Vec::new(),
        }
    }

    pub fn clear(&mut self, new_start: Instant) {
        self.measurements.clear();
        self.started_at = new_start;
    }

    pub fn store(&mut self, happened_at: Instant, event: MeasurementEvent) {
        assert!(
            happened_at >= self.started_at,
            "time travellers shouldn't use this code"
        );
        assert!(
            self.measurements.len() < (self.measurements.capacity() - 1),
            "measurement buffer will overflow"
        );

        self.measurements
            .push(Measurement {
                microsecond: (happened_at - self.started_at).as_micros() as u32,
                event,
            })
            .expect("measurement buffer push shouldn't fail")
    }
}
