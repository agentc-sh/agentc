// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use std::path::PathBuf;
use thiserror::Error;

use agentc_compiler::transformer::types::TransformedAsset;

use crate::pipeline::{sender::Tx, traits::Step};

#[derive(Debug, Error)]
pub enum CleanupStepError {
    #[error("event channel closed")]
    EventChannelClosed,
}

pub enum CleanupStepEvent {
    Started {
        path_count: usize,
    },
    /// Emitted when a single ephemeral artifact path could not be removed.
    /// The step continues cleaning up remaining paths rather than aborting.
    RemoveFailed {
        path: PathBuf,
        error: String,
    },
    Completed,
}

pub struct CleanupStepInput<T> {
    pub inner: T,
    pub assets: Vec<TransformedAsset>,
}

pub struct CleanupStepOutput<T> {
    pub inner: T,
}

pub struct CleanupStep<T>
where
    T: Send,
{
    skip: bool,
    _marker: std::marker::PhantomData<T>,
}

impl<T> CleanupStep<T>
where
    T: Send,
{
    pub fn new(skip: bool) -> Self {
        Self { skip, _marker: std::marker::PhantomData }
    }
}

#[async_trait]
impl<T> Step for CleanupStep<T>
where
    T: Send + Sync + 'static,
{
    type Input = CleanupStepInput<T>;
    type Output = CleanupStepOutput<T>;
    type Event = CleanupStepEvent;
    type Error = CleanupStepError;

    async fn execute<S>(self, input: Self::Input, tx: S) -> Result<Self::Output, Self::Error>
    where
        S: Tx<Item = Self::Event> + Clone + Send + Sync + 'static,
    {
        let ephemeral_paths: Vec<PathBuf> = input
            .assets
            .iter()
            .flat_map(|asset| &asset.artifacts)
            .filter(|artifact| artifact.ephemeral)
            .filter_map(|artifact| artifact.as_path().cloned())
            .collect();

        tx.send(CleanupStepEvent::Started { path_count: ephemeral_paths.len() })
            .await
            .map_err(|_| CleanupStepError::EventChannelClosed)?;

        if !self.skip {
            for path in &ephemeral_paths {
                match tokio::fs::remove_dir_all(path).await {
                    Ok(()) => {}
                    // NotFound means a previous cleanup already removed this path
                    // (e.g. two artifacts sharing the same temp root). Not an error.
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                    Err(e) => {
                        tx.send(CleanupStepEvent::RemoveFailed {
                            path: path.clone(),
                            error: e.to_string(),
                        })
                        .await
                        .map_err(|_| CleanupStepError::EventChannelClosed)?;
                    }
                }
            }
        }

        tx.send(CleanupStepEvent::Completed)
            .await
            .map_err(|_| CleanupStepError::EventChannelClosed)?;

        Ok(CleanupStepOutput { inner: input.inner })
    }
}
