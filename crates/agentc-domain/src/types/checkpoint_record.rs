// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    str::FromStr,
};
use uuid::Uuid;

/// The reason a checkpoint record was written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

/// An immutable record of graph execution state at a specific point in time.
///
/// One record is written per meaningful graph event: input, each completed node step,
/// interrupt, and finish. Records form a linked list via
/// [`parent_checkpoint_id`](CheckpointRecord::parent_checkpoint_id), enabling time travel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointRecord {
    /// The unique ID of this record. Also the key used to save/load state.
    pub id: Uuid,
    pub tenant_id: String,
    pub session_id: Uuid,
    pub run_id: Uuid,
    /// The node that was current when this record was taken.
    pub node: String,
    pub status: crate::types::RunStatus,
    pub reason: CheckpointReason,
    pub created_at: DateTime<Utc>,
    /// The ID of the record that preceded this one in the same run.
    /// `None` for the first (Input) record of a run.
    pub parent_checkpoint_id: Option<Uuid>,
    /// Optional metadata stored with this record (e.g. interrupt payload).
    pub metadata: Option<serde_json::Value>,
}
