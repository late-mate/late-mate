use crate::MutexKind;
use defmt::error;
use embassy_sync::mutex::Mutex;
use embassy_time::Instant;
use heapless::Vec;
use late_mate_shared::comms::device_to_host::{Measurement, MeasurementEvent};
use late_mate_shared::MAX_SCENARIO_DURATION_MS;
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
            measurements: Vec::new(),
        }
    }

    pub fn clear(&mut self, new_start: Instant) {
        self.measurements.clear();
        self.started_at = new_start;
    }

    /// Returns Error if the buffer will overflow
    pub fn store(&mut self, happened_at: Instant, event: MeasurementEvent) -> Result<(), ()> {
        assert!(
            happened_at >= self.started_at,
            "Time travellers shouldn't use this code"
        );

        if self.measurements.len() >= (self.measurements.capacity() - 1) {
            error!("Can't push into the scenario buffer, it will overflow");
            return Err(());
        }

        self.measurements
            .push(Measurement {
                // todo: check for overflow
                microsecond: (happened_at - self.started_at).as_micros() as u32,
                event,
            })
            .expect("Measurement buffer push shouldn't fail");

        Ok(())
    }
}
