use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathHealthEvent {
    None,
    BecameSuspect,
    Recovered,
    ChallengeRequested,
}

#[derive(Debug)]
pub struct PathHealthMonitor {
    suspect_after: Duration,
    challenge_after: Duration,
    last_progress_at: Instant,
    last_progress_value: u64,
    suspect_since: Option<Instant>,
    challenge_requested: bool,
}

impl PathHealthMonitor {
    pub fn new(
        suspect_after: Duration,
        challenge_after: Duration,
        now: Instant,
        initial_progress_value: u64,
    ) -> Self {
        Self {
            suspect_after,
            challenge_after,
            last_progress_at: now,
            last_progress_value: initial_progress_value,
            suspect_since: None,
            challenge_requested: false,
        }
    }

    pub fn observe(&mut self, now: Instant, progress_value: u64) -> PathHealthEvent {
        if progress_value > self.last_progress_value {
            self.last_progress_value = progress_value;
            self.last_progress_at = now;

            if self.suspect_since.take().is_some() {
                self.challenge_requested = false;
                return PathHealthEvent::Recovered;
            }

            return PathHealthEvent::None;
        }

        if self.suspect_since.is_none() {
            if now.duration_since(self.last_progress_at) >= self.suspect_after {
                self.suspect_since = Some(now);
                return PathHealthEvent::BecameSuspect;
            }

            return PathHealthEvent::None;
        }

        let suspect_since = self
            .suspect_since
            .expect("suspect_since should be set here");

        if !self.challenge_requested && now.duration_since(suspect_since) >= self.challenge_after {
            self.challenge_requested = true;
            return PathHealthEvent::ChallengeRequested;
        }

        PathHealthEvent::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SUSPECT_AFTER: Duration = Duration::from_millis(200);
    const CHALLENGE_AFTER: Duration = Duration::from_millis(300);

    fn monitor(now: Instant) -> PathHealthMonitor {
        PathHealthMonitor::new(SUSPECT_AFTER, CHALLENGE_AFTER, now, 10)
    }

    #[test]
    fn starts_without_emitting_an_event() {
        let now = Instant::now();
        let mut monitor = monitor(now);

        let event = monitor.observe(now, 10);

        assert_eq!(event, PathHealthEvent::None);
    }

    #[test]
    fn progress_resets_progress_timer() {
        let start = Instant::now();
        let mut monitor = monitor(start);

        let event = monitor.observe(start + Duration::from_millis(150), 11);

        assert_eq!(event, PathHealthEvent::None);

        let event = monitor.observe(start + Duration::from_millis(300), 11);

        assert_eq!(event, PathHealthEvent::None);
    }

    #[test]
    fn no_progress_before_threshold_stays_healthy() {
        let start = Instant::now();
        let mut monitor = monitor(start);

        let event = monitor.observe(start + Duration::from_millis(199), 10);

        assert_eq!(event, PathHealthEvent::None);
    }

    #[test]
    fn no_progress_reaching_threshold_becomes_suspect() {
        let start = Instant::now();
        let mut monitor = monitor(start);

        let event = monitor.observe(start + SUSPECT_AFTER, 10);

        assert_eq!(event, PathHealthEvent::BecameSuspect);
    }

    #[test]
    fn suspect_event_is_only_emitted_once() {
        let start = Instant::now();
        let mut monitor = monitor(start);

        assert_eq!(
            monitor.observe(start + SUSPECT_AFTER, 10),
            PathHealthEvent::BecameSuspect
        );

        assert_eq!(
            monitor.observe(start + SUSPECT_AFTER + Duration::from_millis(50), 10,),
            PathHealthEvent::None
        );
    }

    #[test]
    fn progress_while_suspect_reports_recovery() {
        let start = Instant::now();
        let mut monitor = monitor(start);

        assert_eq!(
            monitor.observe(start + SUSPECT_AFTER, 10),
            PathHealthEvent::BecameSuspect
        );

        assert_eq!(
            monitor.observe(start + SUSPECT_AFTER + Duration::from_millis(100), 11,),
            PathHealthEvent::Recovered
        );
    }

    #[test]
    fn persistent_no_progress_requests_challenge() {
        let start = Instant::now();
        let mut monitor = monitor(start);

        let suspect_at = start + SUSPECT_AFTER;

        assert_eq!(
            monitor.observe(suspect_at, 10),
            PathHealthEvent::BecameSuspect
        );

        assert_eq!(
            monitor.observe(suspect_at + CHALLENGE_AFTER, 10,),
            PathHealthEvent::ChallengeRequested
        );
    }

    #[test]
    fn challenge_is_not_requested_before_challenge_threshold() {
        let start = Instant::now();
        let mut monitor = monitor(start);

        let suspect_at = start + SUSPECT_AFTER;

        assert_eq!(
            monitor.observe(suspect_at, 10),
            PathHealthEvent::BecameSuspect
        );

        assert_eq!(
            monitor.observe(suspect_at + CHALLENGE_AFTER - Duration::from_millis(1), 10,),
            PathHealthEvent::None
        );
    }

    #[test]
    fn challenge_event_is_only_emitted_once() {
        let start = Instant::now();
        let mut monitor = monitor(start);

        let suspect_at = start + SUSPECT_AFTER;

        assert_eq!(
            monitor.observe(suspect_at, 10),
            PathHealthEvent::BecameSuspect
        );

        assert_eq!(
            monitor.observe(suspect_at + CHALLENGE_AFTER, 10,),
            PathHealthEvent::ChallengeRequested
        );

        assert_eq!(
            monitor.observe(suspect_at + CHALLENGE_AFTER + Duration::from_millis(50), 10,),
            PathHealthEvent::None
        );
    }

    #[test]
    fn recovery_allows_future_suspicion() {
        let start = Instant::now();
        let mut monitor = monitor(start);

        assert_eq!(
            monitor.observe(start + SUSPECT_AFTER, 10),
            PathHealthEvent::BecameSuspect
        );

        let recovered_at = start + SUSPECT_AFTER + Duration::from_millis(100);

        assert_eq!(
            monitor.observe(recovered_at, 11),
            PathHealthEvent::Recovered
        );

        assert_eq!(
            monitor.observe(recovered_at + SUSPECT_AFTER, 11,),
            PathHealthEvent::BecameSuspect
        );
    }
}
