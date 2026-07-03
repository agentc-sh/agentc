// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    str::FromStr,
};
use uuid::Uuid;

use crate::graph::state::{GraphNode, StateOf, InputOf};

/// The status of a run, which can be used to determine if a run is active, completed, or failed.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Running,
    Interrupted,
    Completed,
    Failed,
}

impl RunStatus {
    pub fn is_complete(&self) -> bool {
        matches!(self, RunStatus::Completed | RunStatus::Failed)
    }

    pub fn is_interrupted(&self) -> bool {
        matches!(self, RunStatus::Interrupted)
    }

    pub fn is_running(&self) -> bool {
        matches!(self, RunStatus::Running)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            RunStatus::Running => "running",
            RunStatus::Interrupted => "interrupted",
            RunStatus::Completed => "completed",
            RunStatus::Failed => "failed",
        }
    }
}

impl FromStr for RunStatus {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "running" => Ok(RunStatus::Running),
            "interrupted" => Ok(RunStatus::Interrupted),
            "completed" => Ok(RunStatus::Completed),
            "failed" => Ok(RunStatus::Failed),
            _ => Ok(RunStatus::Failed),
        }
    }
}

impl Display for RunStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.as_str())
    }
}

/// The reason a checkpoint snapshot was written.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CheckpointReason {
    /// Written when a new run starts, capturing initial input state.
    Input,
    /// Written after each node completes successfully.
    Step,
    /// Written when a node raises an interrupt.
    Interrupt,
    /// Written when the run finishes (completed or failed).
    Finish,
}

impl CheckpointReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            CheckpointReason::Input => "input",
            CheckpointReason::Step => "step",
            CheckpointReason::Interrupt => "interrupt",
            CheckpointReason::Finish => "finish",
        }
    }
}

impl FromStr for CheckpointReason {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "input" => Ok(CheckpointReason::Input),
            "step" => Ok(CheckpointReason::Step),
            "interrupt" => Ok(CheckpointReason::Interrupt),
            "finish" => Ok(CheckpointReason::Finish),
            _ => Err(()),
        }
    }
}

impl Display for CheckpointReason {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.as_str())
    }
}

/// An immutable snapshot of graph execution state at a specific point in time.
///
/// One snapshot is written per meaningful graph event: input, each completed node step,
/// interrupt, and finish. Snapshots form a linked list via
/// [`parent_checkpoint_id`](CheckpointSnapshot::parent_checkpoint_id), enabling time travel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointSnapshot {
    /// The unique ID of this snapshot. Also the key used to save/load state.
    pub checkpoint_id: Uuid,
    pub tenant_id: String,
    pub session_id: Uuid,
    pub run_id: Uuid,
    /// The node that was current when this snapshot was taken.
    pub node: String,
    pub status: RunStatus,
    pub reason: CheckpointReason,
    pub created_at: DateTime<Utc>,
    /// The checkpoint_id of the snapshot that preceded this one in the same run.
    /// `None` for the first (Input) snapshot of a run.
    pub parent_checkpoint_id: Option<Uuid>,
    /// Optional metadata stored with this snapshot (e.g. interrupt payload).
    pub metadata: Option<Value>,
}

/// A checkpoint represents the state of a graph at a specific point in time, which can be used to resume execution from that point.
pub enum Checkpoint<N: GraphNode> {
    /// The initial state of a graph, which can be used to start execution from the beginning.
    Initial(StateOf<N>),
    /// A checkpoint representing an active session that can be resumed.
    Resume {
        state: StateOf<N>,
        checkpoint_id: Uuid,
        node: Option<N>,
    },
}

impl<N: GraphNode> Checkpoint<N> {
    /// Creates a new [`Checkpoint::Initial`](Checkpoint::Initial) with the given state.
    pub fn initial(state: StateOf<N>) -> Self {
        Self::Initial(state)
    }

    /// Creates a new [`Checkpoint::Resume`](Checkpoint::Resume) with the given state and checkpoint ID.
    pub fn resume(state: StateOf<N>, checkpoint_id: impl Into<Uuid>, node: Option<N>) -> Self {
        Self::Resume {
            state,
            checkpoint_id: checkpoint_id.into(),
            node,
        }
    }
}

pub struct LoadCheckpointParams<N: GraphNode> {
    pub tenant_id: String,
    pub session_id: Uuid,
    pub run_id: Uuid,
    pub input: InputOf<N>,
    pub checkpoint_id: Option<Uuid>,
}

pub struct SaveCheckpointParams<N: GraphNode> {
    pub tenant_id: String,
    pub session_id: Uuid,
    pub run_id: Uuid,
    pub node: String,
    pub state: StateOf<N>,
    pub reason: CheckpointReason,
    pub parent_checkpoint_id: Option<Uuid>,
    pub metadata: Option<Value>,
}

pub struct FinishCheckpointParams<N: GraphNode> {
    pub tenant_id: String,
    pub session_id: Uuid,
    pub run_id: Uuid,
    pub node: String,
    pub status: RunStatus,
    pub state: StateOf<N>,
    pub parent_checkpoint_id: Option<Uuid>,
    pub metadata: Option<Value>,
}
