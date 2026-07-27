use std::fmt;
use std::net::SocketAddr;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationState {
    Healthy,
    Suspect,
    Challenging,
    Migrating,
}

impl fmt::Display for MigrationState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            MigrationState::Healthy => "Healthy",
            MigrationState::Suspect => "Suspect",
            MigrationState::Challenging => "Challenging",
            MigrationState::Migrating => "Migrating",
        };

        write!(f, "{value}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationReason {
    ControlledTrigger,
    ConditionCleared,
    ConditionPersisted,
    AlternatePathReady,
    MigrationCompleted,
}

impl fmt::Display for MigrationReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            MigrationReason::ControlledTrigger => "controlled_trigger",
            MigrationReason::ConditionCleared => "condition_cleared",
            MigrationReason::ConditionPersisted => "condition_persisted",
            MigrationReason::AlternatePathReady => "alternate_path_ready",
            MigrationReason::MigrationCompleted => "migration_completed",
        };

        write!(f, "{value}")
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MigrationContext {
    pub elapsed: Duration,
    pub active_local: SocketAddr,
    pub candidate_local: Option<SocketAddr>,
    pub connection_id: usize,
}

#[derive(Debug)]
pub struct MigrationController {
    state: MigrationState,
}

impl MigrationController {
    pub fn new() -> Self {
        Self {
            state: MigrationState::Healthy,
        }
    }

    pub fn state(&self) -> MigrationState {
        self.state
    }

    pub fn transition(
        &mut self,
        next: MigrationState,
        reason: MigrationReason,
        context: MigrationContext,
    ) -> anyhow::Result<()> {
        if !is_valid_transition(self.state, next) {
            anyhow::bail!(
                "invalid migration state transition: {} -> {}",
                self.state,
                next
            );
        }

        let previous = self.state;
        self.state = next;

        println!(
            "event=migration_state \
             elapsed_seconds={:.3} \
             from={} \
             to={} \
             reason={} \
             active_local={} \
             candidate_local={} \
             connection={}",
            context.elapsed.as_secs_f64(),
            previous,
            next,
            reason,
            context.active_local,
            context
                .candidate_local
                .map(|addr| addr.to_string())
                .unwrap_or_else(|| "none".to_string()),
            context.connection_id,
        );

        Ok(())
    }
}

fn is_valid_transition(current: MigrationState, next: MigrationState) -> bool {
    matches!(
        (current, next),
        (MigrationState::Healthy, MigrationState::Suspect)
            | (MigrationState::Suspect, MigrationState::Healthy)
            | (MigrationState::Suspect, MigrationState::Challenging)
            | (MigrationState::Challenging, MigrationState::Healthy)
            | (MigrationState::Challenging, MigrationState::Migrating)
            | (MigrationState::Migrating, MigrationState::Healthy)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_context() -> MigrationContext {
        MigrationContext {
            elapsed: Duration::from_millis(500),
            active_local: "127.0.0.1:4000".parse().unwrap(),
            candidate_local: Some("127.0.0.1:5000".parse().unwrap()),
            connection_id: 42,
        }
    }

    #[test]
    fn controller_starts_healthy() {
        let controller = MigrationController::new();

        assert_eq!(controller.state(), MigrationState::Healthy);
    }

    #[test]
    fn valid_migration_sequence_is_allowed() {
        let mut controller = MigrationController::new();

        controller
            .transition(
                MigrationState::Suspect,
                MigrationReason::ControlledTrigger,
                test_context(),
            )
            .unwrap();

        controller
            .transition(
                MigrationState::Challenging,
                MigrationReason::ConditionPersisted,
                test_context(),
            )
            .unwrap();

        controller
            .transition(
                MigrationState::Migrating,
                MigrationReason::AlternatePathReady,
                test_context(),
            )
            .unwrap();

        controller
            .transition(
                MigrationState::Healthy,
                MigrationReason::MigrationCompleted,
                test_context(),
            )
            .unwrap();

        assert_eq!(controller.state(), MigrationState::Healthy);
    }

    #[test]
    fn suspect_can_recover_without_migration() {
        let mut controller = MigrationController::new();

        controller
            .transition(
                MigrationState::Suspect,
                MigrationReason::ControlledTrigger,
                test_context(),
            )
            .unwrap();

        controller
            .transition(
                MigrationState::Healthy,
                MigrationReason::ConditionCleared,
                test_context(),
            )
            .unwrap();

        assert_eq!(controller.state(), MigrationState::Healthy);
    }

    #[test]
    fn invalid_transition_is_rejected() {
        let mut controller = MigrationController::new();

        let result = controller.transition(
            MigrationState::Migrating,
            MigrationReason::AlternatePathReady,
            test_context(),
        );

        assert!(result.is_err());
        assert_eq!(controller.state(), MigrationState::Healthy);
    }
}
