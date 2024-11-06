use std::{collections::VecDeque, time::Duration};
use web_time::Instant;

pub const FRAME_TIME_SAMPLES: usize = 60;

#[derive(Debug, Default)]
pub struct PerformanceTracker {
    frame_time: VecDeque<Duration>,
    frame_timestamp: VecDeque<Instant>,
}

impl PerformanceTracker {
    pub fn frame_time(&self) -> &VecDeque<Duration> {
        &self.frame_time
    }

    pub fn last_frame_time(&self) -> Option<&Duration> {
        self.frame_time.front()
    }

    pub fn add_sample(&mut self, frame_time: Duration, frame_timestamp: Instant) {
        self.frame_time.push_back(frame_time);
        while self.frame_time.len() > FRAME_TIME_SAMPLES {
            self.frame_time.pop_front();
        }

        self.frame_timestamp.push_back(frame_timestamp);
        while self.frame_timestamp.len() > FRAME_TIME_SAMPLES {
            self.frame_timestamp.pop_front();
        }
    }

    pub fn fps(&self) -> Option<f32> {
        self.frame_timestamp
            .front()
            .zip(self.frame_timestamp.back())
            .take_if(|(first, last)| first != last)
            .map(|(first, last)| {
                let intervals = self.frame_timestamp.len() - 1;
                let duration = *last - *first;
                let avg_duration = duration.as_nanos() as f32 / intervals as f32;
                let one_second = Duration::from_secs(1).as_nanos() as f32;
                one_second / avg_duration
            })
    }
}
