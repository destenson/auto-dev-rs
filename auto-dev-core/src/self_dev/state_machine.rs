#![allow(unused)]
//! State machine for managing self-development lifecycle

use super::{Result, SelfDevError};
use serde::{Deserialize, Serialize};
use std::fmt;
use tracing::{debug, info};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DevelopmentState {
    Idle,
    Analyzing,
    Planning,
    Developing,
    Testing,
    Reviewing,
    Deploying,
    Monitoring,
    Learning,
}

impl fmt::Display for DevelopmentState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Idle => write!(f, "Idle"),
            Self::Analyzing => write!(f, "Analyzing Requirements"),
            Self::Planning => write!(f, "Planning Implementation"),
            Self::Developing => write!(f, "Developing Solution"),
            Self::Testing => write!(f, "Testing Changes"),
            Self::Reviewing => write!(f, "Reviewing Safety"),
            Self::Deploying => write!(f, "Deploying Changes"),
            Self::Monitoring => write!(f, "Monitoring Effects"),
            Self::Learning => write!(f, "Learning Patterns"),
        }
    }
}

pub struct DevelopmentStateMachine {
    current_state: DevelopmentState,
    previous_state: Option<DevelopmentState>,
    is_paused: bool,
    transition_history: Vec<StateTransition>,
}

#[derive(Debug, Clone)]
struct StateTransition {
    from: DevelopmentState,
    to: DevelopmentState,
    timestamp: std::time::SystemTime,
    reason: Option<String>,
}

impl DevelopmentStateMachine {
    pub fn new(initial_state: DevelopmentState) -> Self {
        info!("Initializing state machine with state: {}", initial_state);

        Self {
            current_state: initial_state,
            previous_state: None,
            is_paused: false,
            transition_history: Vec::new(),
        }
    }

    pub fn current_state(&self) -> DevelopmentState {
        self.current_state
    }

    pub fn previous_state(&self) -> Option<DevelopmentState> {
        self.previous_state
    }

    pub fn is_paused(&self) -> bool {
        self.is_paused
    }

    pub fn pause(&mut self) -> Result<()> {
        if self.is_paused {
            return Err(SelfDevError::StateTransition(
                "State machine is already paused".to_string(),
            ));
        }

        info!("Pausing state machine at state: {}", self.current_state);
        self.is_paused = true;
        Ok(())
    }

    pub fn resume(&mut self) -> Result<()> {
        if !self.is_paused {
            return Err(SelfDevError::StateTransition("State machine is not paused".to_string()));
        }

        info!("Resuming state machine at state: {}", self.current_state);
        self.is_paused = false;
        Ok(())
    }

    pub fn transition_to(&mut self, new_state: DevelopmentState) -> Result<()> {
        if self.is_paused && new_state != DevelopmentState::Idle {
            return Err(SelfDevError::StateTransition(
                "Cannot transition while paused (except to Idle)".to_string(),
            ));
        }

        if !self.is_valid_transition(self.current_state, new_state) {
            return Err(SelfDevError::StateTransition(format!(
                "Invalid transition from {} to {}",
                self.current_state, new_state
            )));
        }

        debug!("Transitioning from {} to {}", self.current_state, new_state);

        let transition = StateTransition {
            from: self.current_state,
            to: new_state,
            timestamp: std::time::SystemTime::now(),
            reason: None,
        };

        self.transition_history.push(transition);
        self.previous_state = Some(self.current_state);
        self.current_state = new_state;

        info!("State transition complete: {}", new_state);
        Ok(())
    }

    pub fn transition_with_reason(
        &mut self,
        new_state: DevelopmentState,
        reason: String,
    ) -> Result<()> {
        if self.is_paused && new_state != DevelopmentState::Idle {
            return Err(SelfDevError::StateTransition(
                "Cannot transition while paused (except to Idle)".to_string(),
            ));
        }

        if !self.is_valid_transition(self.current_state, new_state) {
            return Err(SelfDevError::StateTransition(format!(
                "Invalid transition from {} to {}",
                self.current_state, new_state
            )));
        }

        debug!("Transitioning from {} to {} (reason: {})", self.current_state, new_state, reason);

        let transition = StateTransition {
            from: self.current_state,
            to: new_state,
            timestamp: std::time::SystemTime::now(),
            reason: Some(reason),
        };

        self.transition_history.push(transition);
        self.previous_state = Some(self.current_state);
        self.current_state = new_state;

        info!("State transition complete: {}", new_state);
        Ok(())
    }

    fn is_valid_transition(&self, from: DevelopmentState, to: DevelopmentState) -> bool {
        use DevelopmentState::*;

        match (from, to) {
            (Idle, Analyzing) => true,
            (Idle, Idle) => true,

            (Analyzing, Planning) => true,
            (Analyzing, Idle) => true,

            (Planning, Developing) => true,
            (Planning, Idle) => true,

            (Developing, Testing) => true,
            (Developing, Planning) => true,
            (Developing, Idle) => true,

            (Testing, Reviewing) => true,
            (Testing, Developing) => true,
            (Testing, Idle) => true,

            (Reviewing, Deploying) => true,
            (Reviewing, Planning) => true,
            (Reviewing, Idle) => true,

            (Deploying, Monitoring) => true,
            (Deploying, Idle) => true,

            (Monitoring, Learning) => true,
            (Monitoring, Idle) => true,

            (Learning, Idle) => true,

            _ => false,
        }
    }

    pub fn get_transition_history(&self) -> Vec<(DevelopmentState, DevelopmentState)> {
        self.transition_history.iter().map(|t| (t.from, t.to)).collect()
    }

    pub fn get_time_in_current_state(&self) -> std::time::Duration {
        self.transition_history
            .last()
            .map(|t| {
                std::time::SystemTime::now()
                    .duration_since(t.timestamp)
                    .unwrap_or(std::time::Duration::from_secs(0))
            })
            .unwrap_or(std::time::Duration::from_secs(0))
    }

    pub fn reset(&mut self) {
        info!("Resetting state machine to Idle");

        self.current_state = DevelopmentState::Idle;
        self.previous_state = None;
        self.is_paused = false;
        self.transition_history.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        let sm = DevelopmentStateMachine::new(DevelopmentState::Idle);
        assert_eq!(sm.current_state(), DevelopmentState::Idle);
        assert!(!sm.is_paused());
        assert_eq!(sm.previous_state(), None);
    }

    #[test]
    fn test_valid_transitions() {
        let mut sm = DevelopmentStateMachine::new(DevelopmentState::Idle);

        assert!(sm.transition_to(DevelopmentState::Analyzing).is_ok());
        assert_eq!(sm.current_state(), DevelopmentState::Analyzing);
        assert_eq!(sm.previous_state(), Some(DevelopmentState::Idle));

        assert!(sm.transition_to(DevelopmentState::Planning).is_ok());
        assert_eq!(sm.current_state(), DevelopmentState::Planning);

        assert!(sm.transition_to(DevelopmentState::Developing).is_ok());
        assert!(sm.transition_to(DevelopmentState::Testing).is_ok());
        assert!(sm.transition_to(DevelopmentState::Reviewing).is_ok());
        assert!(sm.transition_to(DevelopmentState::Deploying).is_ok());
        assert!(sm.transition_to(DevelopmentState::Monitoring).is_ok());
        assert!(sm.transition_to(DevelopmentState::Learning).is_ok());
        assert!(sm.transition_to(DevelopmentState::Idle).is_ok());
    }

    #[test]
    fn test_invalid_transitions() {
        let mut sm = DevelopmentStateMachine::new(DevelopmentState::Idle);

        assert!(sm.transition_to(DevelopmentState::Deploying).is_err());

        sm.transition_to(DevelopmentState::Analyzing).unwrap();
        assert!(sm.transition_to(DevelopmentState::Testing).is_err());
    }

    #[test]
    fn test_pause_resume() {
        let mut sm = DevelopmentStateMachine::new(DevelopmentState::Developing);

        assert!(sm.pause().is_ok());
        assert!(sm.is_paused());

        assert!(sm.pause().is_err());

        assert!(sm.transition_to(DevelopmentState::Testing).is_err());

        assert!(sm.transition_to(DevelopmentState::Idle).is_ok());

        assert!(sm.resume().is_ok());
        assert!(!sm.is_paused());

        assert!(sm.resume().is_err());
    }

    #[test]
    fn test_reset() {
        let mut sm = DevelopmentStateMachine::new(DevelopmentState::Testing);
        sm.pause().unwrap();

        sm.reset();

        assert_eq!(sm.current_state(), DevelopmentState::Idle);
        assert_eq!(sm.previous_state(), None);
        assert!(!sm.is_paused());
        assert!(sm.get_transition_history().is_empty());
    }
}
