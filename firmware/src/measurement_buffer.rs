use embassy_time::{Duration, Instant};
use heapless::Vec;
use late_mate_comms::{Measurement, MeasurementEvent};

pub const MAX_MEASUREMENT_DURATION: Duration = Duration::from_millis(1000);

pub struct Buffer {
    pub started_at: Instant,
    // MAX_MEASUREMENT_DURATION's worth of measurement at 2khz + 10% slack
    pub measurements: Vec<Measurement, 2200>,
}

impl Buffer {
    pub const fn new() -> Self {
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
