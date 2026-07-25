use std::{collections::HashSet, time::Duration};

#[derive(Debug, Default)]
pub struct FrameTracker {
    received: u64,
    seen: HashSet<u64>,
    highest_frame_id: Option<u64>,
    out_of_order: u64,
    duplicates: u64,
    last_received_at: Option<Duration>,
    largest_receive_gap: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameSummary {
    pub received: u64,
    pub unique: u64,
    pub missing_within_observed_range: u64,
    pub out_of_order: u64,
    pub duplicates: u64,
    pub largest_receive_gap: Duration,
}

impl FrameTracker {
    pub fn record(&mut self, frame_id: u64, received_at: Duration) {
        self.received += 1;

        if let Some(previous) = self.last_received_at {
            let gap = received_at.saturating_sub(previous);

            if gap > self.largest_receive_gap {
                self.largest_receive_gap = gap;
            }
        }

        self.last_received_at = Some(received_at);

        if !self.seen.insert(frame_id) {
            self.duplicates += 1;
            return;
        }

        match self.highest_frame_id {
            None => {
                self.highest_frame_id = Some(frame_id);
            }

            Some(highest) if frame_id < highest => {
                self.out_of_order += 1;
            }

            Some(_) => {
                self.highest_frame_id = Some(frame_id);
            }
        }
    }
}

impl FrameTracker {
    pub fn missing_within_observed_range(&self) -> u64 {
        let Some(highest) = self.highest_frame_id else {
            return 0;
        };

        (highest + 1).saturating_sub(self.seen.len() as u64)
    }
}

impl FrameTracker {
    pub fn summary(&self) -> FrameSummary {
        FrameSummary {
            received: self.received,
            unique: self.seen.len() as u64,
            missing_within_observed_range: self.missing_within_observed_range(),
            out_of_order: self.out_of_order,
            duplicates: self.duplicates,
            largest_receive_gap: self.largest_receive_gap,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracks_in_order_frames() {
        let mut tracker = FrameTracker::default();

        tracker.record(0, Duration::from_millis(0));
        tracker.record(1, Duration::from_millis(20));
        tracker.record(2, Duration::from_millis(40));

        let summary = tracker.summary();

        assert_eq!(summary.received, 3);
        assert_eq!(summary.unique, 3);
        assert_eq!(summary.missing_within_observed_range, 0);
        assert_eq!(summary.out_of_order, 0);
        assert_eq!(summary.duplicates, 0);
    }

    #[test]
    fn detects_missing_frame() {
        let mut tracker = FrameTracker::default();

        tracker.record(0, Duration::from_millis(0));
        tracker.record(1, Duration::from_millis(20));
        tracker.record(3, Duration::from_millis(40));

        assert_eq!(tracker.summary().missing_within_observed_range, 1);
    }

    #[test]
    fn detects_out_of_order_frame() {
        let mut tracker = FrameTracker::default();

        tracker.record(0, Duration::from_millis(0));
        tracker.record(2, Duration::from_millis(20));
        tracker.record(1, Duration::from_millis(40));

        let summary = tracker.summary();

        assert_eq!(summary.out_of_order, 1);
        assert_eq!(summary.missing_within_observed_range, 0);
    }

    #[test]
    fn detects_duplicate_frame() {
        let mut tracker = FrameTracker::default();

        tracker.record(0, Duration::from_millis(0));
        tracker.record(1, Duration::from_millis(20));
        tracker.record(1, Duration::from_millis(30));
        tracker.record(2, Duration::from_millis(40));

        let summary = tracker.summary();

        assert_eq!(summary.received, 4);
        assert_eq!(summary.unique, 3);
        assert_eq!(summary.duplicates, 1);
    }

    #[test]
    fn tracks_largest_receive_gap() {
        let mut tracker = FrameTracker::default();

        tracker.record(0, Duration::from_millis(0));
        tracker.record(1, Duration::from_millis(20));
        tracker.record(2, Duration::from_millis(75));

        assert_eq!(
            tracker.summary().largest_receive_gap,
            Duration::from_millis(55)
        );
    }
}
