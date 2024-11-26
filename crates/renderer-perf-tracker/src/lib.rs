use std::{collections::VecDeque, time::Duration};
use web_time::Instant;

#[derive(Debug)]
pub struct PerformanceTracker {
    frame_time_samples: usize,
    frame_time: VecDeque<Duration>,
    frame_timestamp: VecDeque<Instant>,
    frame_time_sum: Duration,
}

impl PerformanceTracker {
    pub fn new(frame_time_samples: usize) -> Self {
        Self {
            frame_time_samples,
            frame_time: VecDeque::new(),
            frame_timestamp: VecDeque::new(),
            frame_time_sum: Duration::ZERO,
        }
    }

    pub fn frame_time(&self) -> &VecDeque<Duration> {
        &self.frame_time
    }

    pub fn avg_frame_time(&self) -> Option<Duration> {
        if self.frame_time.is_empty() {
            None
        } else {
            Some(self.frame_time_sum / self.frame_time.len() as u32)
        }
    }

    pub fn last_frame_time(&self) -> Option<&Duration> {
        self.frame_time.front()
    }

    pub fn add_sample(&mut self, frame_time: Duration, frame_timestamp: Instant) {
        self.frame_time.push_back(frame_time);
        while self.frame_time.len() > self.frame_time_samples {
            if let Some(first_frame_time) = self.frame_time.pop_front() {
                self.frame_time_sum -= first_frame_time;
            }
        }

        self.frame_timestamp.push_back(frame_timestamp);
        while self.frame_timestamp.len() > self.frame_time_samples {
            self.frame_timestamp.pop_front();
        }

        self.frame_time_sum += frame_time;
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
