#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationState {
    Healthy,
    Suspect,
    Challenging,
    Migrating,
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

    pub fn transition(&mut self, next: MigrationState) -> anyhow::Result<()> {
        if !is_valid_transition(self.state, next) {
            anyhow::bail!(
                "invalid migration state transition: {:?} -> {:?}",
                self.state,
                next
            );
        }

        self.state = next;
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

    #[test]
    fn controller_starts_healthy() {
        let controller = MigrationController::new();

        assert_eq!(controller.state(), MigrationState::Healthy);
    }

    #[test]
    fn valid_migration_sequence_is_allowed() {
        let mut controller = MigrationController::new();

        controller.transition(MigrationState::Suspect).unwrap();
        controller.transition(MigrationState::Challenging).unwrap();
        controller.transition(MigrationState::Migrating).unwrap();
        controller.transition(MigrationState::Healthy).unwrap();

        assert_eq!(controller.state(), MigrationState::Healthy);
    }

    #[test]
    fn suspect_can_recover_without_migration() {
        let mut controller = MigrationController::new();

        controller.transition(MigrationState::Suspect).unwrap();
        controller.transition(MigrationState::Healthy).unwrap();

        assert_eq!(controller.state(), MigrationState::Healthy);
    }

    #[test]
    fn invalid_transition_is_rejected() {
        let mut controller = MigrationController::new();

        let result = controller.transition(MigrationState::Migrating);

        assert!(result.is_err());
        assert_eq!(controller.state(), MigrationState::Healthy);
    }
}
