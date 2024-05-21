use late_mate_device::scenario::{Moment, Recording};

#[derive(Debug)]
pub struct ProcessedRecording {
    pub recording: Recording,
    pub changepoint_us: Option<u32>,
}

fn find_changepoint(timeline: &[Moment]) -> Option<u32> {
    // it's unlikely there's any meaningful change in the first 7ms,
    // so I use it to infer the range of noise
    let noise_window = 7_000;
    // require at least 2 noise ranges between start and end to detect change
    let change_detect_gap_multiplier = 2;
    // but for the actual moment of change, use just one noise range
    let change_gap_multiplier = 1;

    let (start_min, start_max) = timeline
        .iter()
        .take_while(|m| m.microsecond < noise_window)
        .filter_map(Moment::to_light_level)
        .fold((u32::MAX, 0u32), |(min, max), light_level| {
            (light_level.min(min), light_level.max(max))
        });

    let last_time = timeline
        .last()
        .expect("light_levels shouldn't be empty at this point")
        .microsecond;

    let (end_min, end_max) = timeline
        .iter()
        .rev()
        .take_while(|m| m.microsecond > (last_time - noise_window))
        .filter_map(Moment::to_light_level)
        .fold((u32::MAX, 0u32), |(min, max), light_level| {
            (light_level.min(min), light_level.max(max))
        });

    let change_detect_gap = (start_max - start_min) * change_detect_gap_multiplier;

    if !(end_min > (start_max + change_detect_gap) || start_min > (end_max + change_detect_gap)) {
        return None;
    }

    let change_gap = (start_max - start_min) * change_gap_multiplier;
    if end_min > start_max {
        // raising signal
        let threshold = start_max + change_gap;
        for moment in timeline.iter() {
            if moment
                .to_light_level()
                .is_some_and(|value| value > threshold)
            {
                return Some(moment.microsecond);
            }
        }
    } else {
        // dropping signal, non-changing signal is already discarded above
        // there must be enough between the ends for this sum to be always positive
        assert!(
            start_min > change_gap,
            "expected started_min ({start_min}) > change_gap ({change_gap})"
        );
        let threshold = start_min - change_gap;
        for moment in timeline.iter() {
            if moment
                .to_light_level()
                .is_some_and(|value| value < threshold)
            {
                return Some(moment.microsecond);
            }
        }
    };

    unreachable!("the signal must cross the threshold given the above")
}

pub fn process_recording(recording: Recording) -> ProcessedRecording {
    let changepoint_us = find_changepoint(&recording.timeline);
    ProcessedRecording {
        recording,
        changepoint_us,
    }
}

#[derive(Debug)]
pub enum FinalStats {
    NoRuns,
    NoSuccesses,
    SingleMeasurement {
        latency: f64,
    },
    MultipleMeasurements {
        has_missing: bool,
        n_samples: usize,
        mean: f64,
        stddev: f64,
        median: f64,
        max: f64,
        min: f64,
    },
}

pub fn process_changepoints(values: &[Option<u32>]) -> FinalStats {
    if values.is_empty() {
        return FinalStats::NoRuns;
    }

    let millis = values
        .iter()
        .filter_map(|x| x.map(|us| f64::from(us) / 1000f64))
        .collect::<Vec<_>>();

    if millis.is_empty() {
        return FinalStats::NoSuccesses;
    }

    if millis.len() == 1 {
        return FinalStats::SingleMeasurement { latency: millis[0] };
    }

    let has_missing = values.len() != millis.len();
    let n = millis.len();
    let mean = statistical::mean(&millis);
    let stddev = statistical::standard_deviation(&millis, Some(mean));
    let median = statistical::median(&millis);
    let max = millis
        .iter()
        .max_by(|a, b| {
            a.partial_cmp(b)
                .expect("There must be no NaNs among changepoints")
        })
        .expect("Floats must not be empty at this point")
        .to_owned();
    let min = millis
        .iter()
        .min_by(|a, b| {
            a.partial_cmp(b)
                .expect("There must be no NaNs among changepoints")
        })
        .expect("Floats must not be empty at this point")
        .to_owned();

    FinalStats::MultipleMeasurements {
        has_missing,
        n_samples: n,
        mean,
        stddev,
        median,
        max,
        min,
    }
}
