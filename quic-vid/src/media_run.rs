use std::time::{Duration, Instant};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct MediaRun {
    id: Uuid,
    started_at: Instant,
    fps: u32,
    duration: Duration,
    total_frames: u64,
}

impl MediaRun {
    pub fn new(fps: u32, duration: Duration) -> anyhow::Result<Self> {
        Self::new_at(Uuid::new_v4(), fps, duration, Instant::now())
    }

    fn new_at(id: Uuid, fps: u32, duration: Duration, started_at: Instant) -> anyhow::Result<Self> {
        if fps == 0 {
            anyhow::bail!("fps must be greater than zero");
        }

        if duration.is_zero() {
            anyhow::bail!("media-run duration must be greater than zero");
        }

        let whole_seconds = duration.as_secs();
        let subsecond_frames = u64::from(duration.subsec_nanos()) * u64::from(fps) / 1_000_000_000;
        let total_frames = u64::from(fps)
            .checked_mul(whole_seconds)
            .and_then(|frames| frames.checked_add(subsecond_frames))
            .ok_or_else(|| anyhow::anyhow!("media-run frame count overflow"))?;

        if total_frames == 0 {
            anyhow::bail!("media-run duration is shorter than one frame interval");
        }

        Ok(Self {
            id,
            started_at,
            fps,
            duration,
            total_frames,
        })
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn fps(&self) -> u32 {
        self.fps
    }

    pub fn duration(&self) -> Duration {
        self.duration
    }

    pub fn total_frames(&self) -> u64 {
        self.total_frames
    }

    pub fn elapsed(&self, now: Instant) -> Duration {
        now.saturating_duration_since(self.started_at)
    }

    pub fn current_frame_id(&self, now: Instant) -> u64 {
        let elapsed_nanos = self.elapsed(now).as_nanos();
        let frame = elapsed_nanos.saturating_mul(u128::from(self.fps)) / 1_000_000_000;

        frame.min(u128::from(u64::MAX)) as u64
    }

    pub fn is_complete(&self, now: Instant) -> bool {
        self.elapsed(now) >= self.duration
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_distinct_media_run_identity() {
        let first = MediaRun::new(30, Duration::from_secs(8)).unwrap();
        let second = MediaRun::new(30, Duration::from_secs(8)).unwrap();

        assert_ne!(first.id(), second.id());
    }

    #[test]
    fn calculates_total_frames_for_bounded_run() {
        let run = MediaRun::new(30, Duration::from_secs(8)).unwrap();

        assert_eq!(run.total_frames(), 240);
    }

    #[test]
    fn frame_id_is_derived_from_media_run_start() {
        let started_at = Instant::now();
        let run = MediaRun::new_at(Uuid::nil(), 10, Duration::from_secs(5), started_at).unwrap();

        assert_eq!(run.current_frame_id(started_at), 0);
        assert_eq!(
            run.current_frame_id(started_at + Duration::from_millis(99)),
            0
        );
        assert_eq!(
            run.current_frame_id(started_at + Duration::from_millis(100)),
            1
        );
        assert_eq!(
            run.current_frame_id(started_at + Duration::from_millis(1_250)),
            12
        );
    }

    #[test]
    fn timeline_advances_across_simulated_transport_gap() {
        let started_at = Instant::now();
        let run = MediaRun::new_at(Uuid::nil(), 10, Duration::from_secs(10), started_at).unwrap();

        let before_gap = run.current_frame_id(started_at + Duration::from_millis(8400));
        let after_gap = run.current_frame_id(started_at + Duration::from_millis(10200));

        assert_eq!(before_gap, 84);
        assert_eq!(after_gap, 102);
        assert!(after_gap > before_gap);
    }

    #[test]
    fn completion_depends_on_media_run_timeline() {
        let started_at = Instant::now();
        let run = MediaRun::new_at(Uuid::nil(), 10, Duration::from_secs(2), started_at).unwrap();

        assert!(!run.is_complete(started_at + Duration::from_millis(1_999)));
        assert!(run.is_complete(started_at + Duration::from_secs(2)));
    }

    #[test]
    fn rejects_invalid_configuration() {
        assert!(MediaRun::new(0, Duration::from_secs(1)).is_err());
        assert!(MediaRun::new(10, Duration::ZERO).is_err());
    }
}
