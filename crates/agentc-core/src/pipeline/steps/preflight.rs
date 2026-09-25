// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use thiserror::Error;

use crate::pipeline::{sender::Tx, traits::Step};

#[derive(Debug, Error)]
#[error("{name}: {reason}")]
pub struct PreconditionError {
    pub name: String,
    pub reason: String,
}

impl PreconditionError {
    pub fn new(name: impl Into<String>, reason: impl Into<String>) -> Self {
        Self { name: name.into(), reason: reason.into() }
    }
}

#[derive(Debug, Error)]
pub enum PreflightStepError {
    #[error("precondition failed: {0}")]
    Precondition(#[from] PreconditionError),

    #[error("event channel closed")]
    EventChannelClosed,
}

pub enum PreflightStepEvent {
    Started { precondition_count: usize },
    Completed,
}

/// A condition that must hold before the pipeline continues.
pub trait Precondition<T>: Send + Sync {
    /// The identifier for this condition.
    fn name(&self) -> &str;

    fn verify(&self, value: &T) -> Result<(), PreconditionError>;
}

/// Verifies a set of [`Precondition`](crate::pipeline::steps::preflight::Precondition)
/// values against a stage's output and passes that output through unchanged.
pub struct PreflightStep<T> {
    preconditions: Vec<Box<dyn Precondition<T>>>,
}

impl<T> PreflightStep<T> {
    pub fn new() -> Self {
        Self { preconditions: Vec::new() }
    }

    pub fn with<P>(mut self, precondition: P) -> Self
    where
        P: Precondition<T> + 'static,
    {
        self.preconditions
            .push(Box::new(precondition));
        self
    }
}

impl<T> Default for PreflightStep<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl<T> Step for PreflightStep<T>
where
    T: Send + Sync + 'static,
{
    type Input = T;
    type Output = T;
    type Event = PreflightStepEvent;
    type Error = PreflightStepError;

    async fn execute<S>(self, input: Self::Input, tx: S) -> Result<Self::Output, Self::Error>
    where
        S: Tx<Item = Self::Event> + Clone + Send + Sync + 'static,
    {
        tx.send(PreflightStepEvent::Started {
            precondition_count: self.preconditions.len(),
        })
        .await
        .map_err(|_| PreflightStepError::EventChannelClosed)?;

        for precondition in &self.preconditions {
            precondition.verify(&input)?;
        }

        tx.send(PreflightStepEvent::Completed)
            .await
            .map_err(|_| PreflightStepError::EventChannelClosed)?;

        Ok(input)
    }
}

#[cfg(test)]
mod tests {
    use tokio::sync::mpsc;

    use super::*;

    struct Payload {
        id: &'static str,
    }

    struct AlwaysPasses;

    impl Precondition<Payload> for AlwaysPasses {
        fn name(&self) -> &str {
            "always_passes"
        }

        fn verify(&self, _value: &Payload) -> Result<(), PreconditionError> {
            Ok(())
        }
    }

    struct AlwaysFails;

    impl Precondition<Payload> for AlwaysFails {
        fn name(&self) -> &str {
            "always_fails"
        }

        fn verify(&self, value: &Payload) -> Result<(), PreconditionError> {
            Err(PreconditionError::new(
                self.name(),
                format!("payload {:?} is rejected", value.id),
            ))
        }
    }

    #[tokio::test]
    async fn passing_preconditions_return_the_input_unchanged() {
        let (tx, _rx) = mpsc::channel(8);

        assert_eq!(
            PreflightStep::new()
                .with(AlwaysPasses)
                .execute(Payload { id: "payload" }, tx)
                .await
                .unwrap()
                .id,
            "payload",
        );
    }

    #[tokio::test]
    async fn a_failing_precondition_reports_its_name_and_reason() {
        let (tx, _rx) = mpsc::channel(8);

        assert!(matches!(
            PreflightStep::new()
                .with(AlwaysPasses)
                .with(AlwaysFails)
                .execute(Payload { id: "payload" }, tx)
                .await,
            Err(PreflightStepError::Precondition(e))
                if e.name == "always_fails" && e.reason.contains("payload")
        ));
    }

    #[tokio::test]
    async fn a_step_with_no_preconditions_passes_the_input_through() {
        let (tx, _rx) = mpsc::channel(8);

        assert_eq!(
            PreflightStep::<Payload>::new()
                .execute(Payload { id: "payload" }, tx)
                .await
                .unwrap()
                .id,
            "payload",
        );
    }
}
