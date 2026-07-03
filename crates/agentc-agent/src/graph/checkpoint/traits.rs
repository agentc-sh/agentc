// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{error::Error, fmt::Display, marker::PhantomData, str::FromStr};
use futures::future::BoxFuture;
use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::graph::{
    checkpoint::{
        errors::CheckpointError,
        types::{
            Checkpoint, CheckpointReason, CheckpointSnapshot, RunStatus,
            LoadCheckpointParams, SaveCheckpointParams, FinishCheckpointParams,
        },
    },
    state::{
        GraphNode, GraphState, GraphStateInput, GraphStateUpdate, InputOf, IntoStateUpdate,
        StateOf, UpdateOf,
    },
};

/// A trait for saving and loading session and run records, which track the lifecycle of graph executions.
#[async_trait]
pub trait SessionStore: Send + Sync {
    type Error: Error + Into<CheckpointError> + Send + Sync;

    /// Saves a new session record.
    async fn save_session(&self, tenant_id: &str, session_id: Uuid) -> Result<(), Self::Error>;

    /// Saves a run record.
    async fn save_run(
        &self,
        tenant_id: &str,
        session_id: Uuid,
        run_id: Uuid,
    ) -> Result<(), Self::Error>;

    /// Updates the status of a run record.
    async fn update_run_status(
        &self,
        tenant_id: &str,
        session_id: Uuid,
        run_id: Uuid,
        status: RunStatus,
    ) -> Result<(), Self::Error>;
}

/// A trait for saving and loading [`CheckpointSnapshot`](crate::graph::checkpoint::types::CheckpointSnapshot) records.
#[async_trait]
pub trait CheckpointSnapshotStore: Send + Sync {
    type Error: Error + Into<CheckpointError> + Send + Sync;

    /// Saves a snapshot. Implementations should insert (not upsert), snapshots are immutable.
    async fn save_snapshot(
        &self,
        snapshot: CheckpointSnapshot,
    ) -> Result<CheckpointSnapshot, Self::Error>;

    /// Loads a snapshot by its checkpoint_id.
    async fn load_snapshot(
        &self,
        tenant_id: &str,
        checkpoint_id: Uuid,
    ) -> Result<Option<CheckpointSnapshot>, Self::Error>;

    /// Loads the latest snapshot for the given session (for normal resume).
    async fn load_latest_for_session(
        &self,
        tenant_id: &str,
        session_id: Uuid,
    ) -> Result<Option<CheckpointSnapshot>, Self::Error>;
}

/// A trait that defines the storage interface for graph states.
#[async_trait]
pub trait StateStore<S: GraphState>: Send + Sync {
    type Error: Error + Into<CheckpointError> + Send + Sync;

    /// Saves state keyed to a specific checkpoint_id. Implementations should insert or upsert.
    async fn save(
        &self,
        tenant_id: &str,
        session_id: Uuid,
        run_id: Uuid,
        checkpoint_id: Uuid,
        state: S,
    ) -> Result<S, Self::Error>;

    /// Loads state for a specific checkpoint id.
    async fn load(
        &self,
        tenant_id: &str,
        session_id: Uuid,
        run_id: Uuid,
        checkpoint_id: Uuid,
    ) -> Result<Option<S>, Self::Error>;
}

/// A trait that defines a scope for checkpoint operations, providing access to both snapshot and state stores.
pub trait CheckpointStoreContext<N: GraphNode>: Send + Sync {
    type SessionStore: SessionStore;
    type SnapshotStore: CheckpointSnapshotStore;
    type StateStore: StateStore<StateOf<N>>;

    fn session_store(&self) -> Self::SessionStore;
    fn snapshot_store(&self) -> Self::SnapshotStore;
    fn state_store(&self) -> Self::StateStore;
}

/// A handle for executing operations within a checkpoint store context, ensuring atomicity.
#[async_trait]
pub trait CheckpointStoreHandle<N: GraphNode>: Send + Sync {
    type Context<'a>: CheckpointStoreContext<N> + 'a
    where
        Self: 'a;

    async fn run<F, R>(&self, f: F) -> Result<R, CheckpointError>
    where
        F: for<'a> FnOnce(&'a Self::Context<'a>) -> BoxFuture<'a, Result<R, CheckpointError>>
            + Send,
        R: Send;
}

/// A trait that defines the interface for loading and saving checkpoints for graph nodes.
#[async_trait]
pub trait Checkpointer<N: GraphNode>: Send + Sync {
    /// Loads a checkpoint for the given session and run.
    /// The input is used to determine if a session or run resume is possible, and to initialize state for a fresh run.
    async fn load(
        &self,
        params: LoadCheckpointParams<N>,
    ) -> Result<Checkpoint<N>, CheckpointError>;

    /// Saves a checkpoint snapshot and state after a node completes.
    async fn save(
        &self,
        params: SaveCheckpointParams<N>,
    ) -> Result<Uuid, CheckpointError>;

    /// Finishes a run by writing a final snapshot and saving the final state.
    async fn finish(
        &self,
        params: FinishCheckpointParams<N>,
    ) -> Result<(), CheckpointError>;
}

/// A graph checkpointer that uses a checkpoint store handle to manage checkpoints for graph nodes.
pub struct GraphCheckpointer<N, H>
where
    N: GraphNode,
    H: CheckpointStoreHandle<N>,
{
    handle: H,
    _marker: PhantomData<N>,
}

impl<N, H> GraphCheckpointer<N, H>
where
    N: GraphNode,
    H: CheckpointStoreHandle<N>,
{
    pub fn new(handle: H) -> Self {
        Self { handle, _marker: PhantomData }
    }
}

#[async_trait]
impl<N, H> Checkpointer<N> for GraphCheckpointer<N, H>
where
    N: GraphNode + FromStr + 'static,
    <N as FromStr>::Err: Display,
    H: CheckpointStoreHandle<N> + 'static,
    StateOf<N>: 'static,
    InputOf<N>: IntoStateUpdate<UpdateOf<N>> + Send + 'static,
{
    async fn load(
        &self,
        params: LoadCheckpointParams<N>
    ) -> Result<Checkpoint<N>, CheckpointError> {
        self.handle
            .run(|ctx| {
                Box::pin(async move {
                    ctx.session_store()
                        .save_session(&params.tenant_id, params.session_id)
                        .await
                        .map_err(|e| CheckpointError::session_store_error(e.to_string()))?;

                    // 1. Time travel: explicit checkpoint_id provided
                    if let Some(cid) = params.checkpoint_id
                        && let Some(snapshot) = ctx
                            .snapshot_store()
                            .load_snapshot(&params.tenant_id, cid)
                            .await
                            .map_err(|e| CheckpointError::checkpoint_store_error(e.to_string()))?
                            && let Some(mut state) = ctx
                                .state_store()
                                .load(&params.tenant_id, params.session_id, params.run_id, snapshot.checkpoint_id)
                                .await
                                .map_err(|e| CheckpointError::state_store_error(e.to_string()))?
                            {
                                params.input
                                    .into_update()
                                    .map_err(|e| {
                                        CheckpointError::unexpected_error(format!(
                                            "Failed to convert input into state update: {e}"
                                        ))
                                    })?
                                    .ok_or_else(|| {
                                        CheckpointError::unexpected_error(
                                            "Input cannot be converted into state update",
                                        )
                                    })?
                                    .apply(&mut state);

                                return Ok(Checkpoint::resume(
                                    state,
                                    snapshot.checkpoint_id,
                                    Some(
                                        snapshot
                                            .node
                                            .parse::<N>()
                                            .map_err(|e| {
                                                CheckpointError::unexpected_error(format!(
                                                    "Failed to parse node from snapshot: {e}"
                                                ))
                                            })?,
                                    ),
                                ));
                            }

                    // 2. Session resume: check for the most recent snapshot for this session, regardless of its
                    // status.
                    if let Some(snapshot) = ctx
                        .snapshot_store()
                        .load_latest_for_session(&params.tenant_id, params.session_id)
                        .await
                        .map_err(|e| CheckpointError::checkpoint_store_error(e.to_string()))?
                        && let Some(mut state) = ctx
                            .state_store()
                            .load(&params.tenant_id, params.session_id, params.run_id, snapshot.checkpoint_id)
                            .await
                            .map_err(|e| CheckpointError::state_store_error(e.to_string()))?
                        {
                            ctx.session_store()
                                .save_run(&params.tenant_id, params.session_id, params.run_id)
                                .await
                                .map_err(|e| CheckpointError::session_store_error(e.to_string()))?;

                            params.input
                                .into_update()
                                .map_err(|e| {
                                    CheckpointError::unexpected_error(format!(
                                        "Failed to convert input into state update: {e}"
                                    ))
                                })?
                                .ok_or_else(|| {
                                    CheckpointError::unexpected_error(
                                        "Input cannot be converted into state update",
                                    )
                                })?
                                .apply(&mut state);

                            return Ok(Checkpoint::resume(
                                state,
                                snapshot.checkpoint_id,
                                if matches!(snapshot.reason, CheckpointReason::Interrupt) {
                                    Some(
                                        snapshot
                                            .node
                                            .parse::<N>()
                                            .map_err(|e| {
                                                CheckpointError::unexpected_error(format!(
                                                    "Failed to parse node from snapshot: {e}"
                                                ))
                                            })?,
                                    )
                                } else {
                                    None
                                },
                            ));
                        }

                    // Fresh run: no snapshots exist for this session, start with initial state
                    ctx.session_store()
                        .save_run(&params.tenant_id, params.session_id, params.run_id)
                        .await
                        .map_err(|e| CheckpointError::session_store_error(e.to_string()))?;

                    Ok(Checkpoint::initial(params.input.initialize()))
                })
            })
            .await
    }

    async fn save(
        &self,
        params: SaveCheckpointParams<N>
    ) -> Result<Uuid, CheckpointError> {
        self.handle
            .run(|ctx| {
                Box::pin(async move {
                    let snapshot = ctx
                        .snapshot_store()
                        .save_snapshot(CheckpointSnapshot {
                            checkpoint_id: Uuid::new_v4(),
                            tenant_id: params.tenant_id.clone(),
                            session_id: params.session_id,
                            run_id: params.run_id,
                            node: params.node,
                            status: if matches!(params.reason, CheckpointReason::Interrupt) {
                                RunStatus::Interrupted
                            } else {
                                RunStatus::Running
                            },
                            reason: params.reason,
                            created_at: Utc::now(),
                            parent_checkpoint_id: params.parent_checkpoint_id,
                            metadata: params.metadata,
                        })
                        .await
                        .map_err(|e| CheckpointError::checkpoint_store_error(e.to_string()))?;

                    ctx.state_store()
                        .save(&params.tenant_id, params.session_id, params.run_id, snapshot.checkpoint_id, params.state.clone())
                        .await
                        .map_err(|e| CheckpointError::state_store_error(e.to_string()))?;

                    Ok(snapshot.checkpoint_id)
                })
            })
            .await
    }

    async fn finish(
        &self,
        params: FinishCheckpointParams<N>
    ) -> Result<(), CheckpointError> {
        self.handle
            .run(|ctx| {
                Box::pin(async move {
                    let checkpoint_id = Uuid::new_v4();

                    ctx.session_store()
                        .update_run_status(&params.tenant_id, params.session_id, params.run_id, params.status)
                        .await
                        .map_err(|e| CheckpointError::session_store_error(e.to_string()))?;

                    ctx.snapshot_store()
                        .save_snapshot(CheckpointSnapshot {
                            checkpoint_id,
                            tenant_id: params.tenant_id.clone(),
                            session_id: params.session_id,
                            run_id: params.run_id,
                            node: params.node.clone(),
                            status: params.status,
                            reason: match params.status {
                                RunStatus::Interrupted => CheckpointReason::Interrupt,
                                _ => CheckpointReason::Finish,
                            },
                            created_at: Utc::now(),
                            parent_checkpoint_id: params.parent_checkpoint_id,
                            metadata: params.metadata,
                        })
                        .await
                        .map_err(|e| CheckpointError::checkpoint_store_error(e.to_string()))?;

                    ctx.state_store()
                        .save(&params.tenant_id, params.session_id, params.run_id, checkpoint_id, params.state)
                        .await
                        .map_err(|e| CheckpointError::state_store_error(e.to_string()))?;

                    Ok(())
                })
            })
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use serde::{Deserialize, Serialize};
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex},
    };
    use uuid::Uuid;

    use crate::graph::{errors::GraphError, state::GraphContext};

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestState {
        pub visited: Vec<String>,
        pub counter: u32,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Default)]
    struct TestStateUpdate {
        pub visit: Option<String>,
        pub increment: u32,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Default)]
    struct TestStateInput {
        pub initial_counter: u32,
    }

    impl GraphState for TestState {
        type Update = TestStateUpdate;
        type Input = TestStateInput;
    }

    impl GraphStateUpdate for TestStateUpdate {
        type State = TestState;

        fn apply(self, state: &mut Self::State) {
            if let Some(visit) = self.visit {
                state.visited.push(visit);
            }
            state.counter += self.increment;
        }

        fn merge(mut self, other: Self) -> Self {
            if self.visit.is_none() {
                self.visit = other.visit;
            }
            self.increment += other.increment;
            self
        }
    }

    impl GraphStateInput for TestStateInput {
        type State = TestState;

        fn initialize(self) -> Self::State {
            TestState {
                visited: Vec::new(),
                counter: self.initial_counter,
            }
        }
    }

    impl IntoStateUpdate<TestStateUpdate> for TestStateInput {
        fn into_update(self) -> Result<Option<TestStateUpdate>, GraphError> {
            Ok(Some(TestStateUpdate {
                visit: None,
                increment: self.initial_counter,
            }))
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    enum TestNode {
        A,
        B,
        C,
    }

    impl std::fmt::Display for TestNode {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                TestNode::A => write!(f, "a"),
                TestNode::B => write!(f, "b"),
                TestNode::C => write!(f, "c"),
            }
        }
    }

    impl std::str::FromStr for TestNode {
        type Err = String;
        fn from_str(s: &str) -> Result<Self, Self::Err> {
            match s {
                "a" => Ok(TestNode::A),
                "b" => Ok(TestNode::B),
                "c" => Ok(TestNode::C),
                _ => Err(format!("unknown node: {s}")),
            }
        }
    }

    #[derive(Debug, Clone)]
    struct TestContext;
    impl GraphContext for TestContext {}

    impl GraphNode for TestNode {
        type Context = TestContext;
        type State = TestState;
    }

    #[derive(Debug, thiserror::Error)]
    #[error("{0}")]
    struct TestError(String);

    impl From<TestError> for CheckpointError {
        fn from(e: TestError) -> Self {
            CheckpointError::checkpoint_store_error(e.to_string())
        }
    }

    // In-memory session/run store
    #[derive(Debug, Clone, Default)]
    struct InMemorySessionStore {
        sessions: Arc<Mutex<HashMap<Uuid, bool>>>,
        runs: Arc<Mutex<HashMap<Uuid, RunStatus>>>,
    }

    #[async_trait::async_trait]
    impl SessionStore for InMemorySessionStore {
        type Error = TestError;

        async fn save_session(
            &self,
            _tenant_id: &str,
            session_id: Uuid,
        ) -> Result<(), Self::Error> {
            self.sessions
                .lock()
                .unwrap()
                .entry(session_id)
                .or_insert(true);
            Ok(())
        }

        async fn save_run(
            &self,
            _tenant_id: &str,
            _session_id: Uuid,
            run_id: Uuid,
        ) -> Result<(), Self::Error> {
            self.runs
                .lock()
                .unwrap()
                .entry(run_id)
                .or_insert(RunStatus::Running);
            Ok(())
        }

        async fn update_run_status(
            &self,
            _tenant_id: &str,
            _session_id: Uuid,
            run_id: Uuid,
            status: RunStatus,
        ) -> Result<(), Self::Error> {
            if let Some(s) = self
                .runs
                .lock()
                .unwrap()
                .get_mut(&run_id)
            {
                *s = status;
            }
            Ok(())
        }
    }

    // In-memory snapshot store keyed by checkpoint_id
    #[derive(Debug, Clone, Default)]
    struct InMemorySnapshotStore {
        snapshots: Arc<Mutex<HashMap<Uuid, CheckpointSnapshot>>>,
    }

    #[async_trait::async_trait]
    impl CheckpointSnapshotStore for InMemorySnapshotStore {
        type Error = TestError;

        async fn save_snapshot(
            &self,
            snapshot: CheckpointSnapshot,
        ) -> Result<CheckpointSnapshot, Self::Error> {
            self.snapshots
                .lock()
                .unwrap()
                .insert(snapshot.checkpoint_id, snapshot.clone());
            Ok(snapshot)
        }

        async fn load_snapshot(
            &self,
            _tenant_id: &str,
            checkpoint_id: Uuid,
        ) -> Result<Option<CheckpointSnapshot>, Self::Error> {
            Ok(self
                .snapshots
                .lock()
                .unwrap()
                .get(&checkpoint_id)
                .cloned())
        }

        async fn load_latest_for_session(
            &self,
            _tenant_id: &str,
            session_id: Uuid,
        ) -> Result<Option<CheckpointSnapshot>, Self::Error> {
            Ok(self
                .snapshots
                .lock()
                .unwrap()
                .values()
                .filter(|s| s.session_id == session_id)
                .max_by_key(|s| s.created_at)
                .cloned())
        }
    }

    // In-memory state store keyed by checkpoint_id
    #[derive(Debug, Clone, Default)]
    struct InMemoryStateStore {
        states_by_checkpoint: Arc<Mutex<HashMap<Uuid, TestState>>>,
    }

    #[async_trait::async_trait]
    impl StateStore<TestState> for InMemoryStateStore {
        type Error = TestError;

        async fn save(
            &self,
            _tenant_id: &str,
            _session_id: Uuid,
            _run_id: Uuid,
            checkpoint_id: Uuid,
            state: TestState,
        ) -> Result<TestState, Self::Error> {
            self.states_by_checkpoint
                .lock()
                .unwrap()
                .insert(checkpoint_id, state.clone());
            Ok(state)
        }

        async fn load(
            &self,
            _tenant_id: &str,
            _session_id: Uuid,
            _run_id: Uuid,
            checkpoint_id: Uuid,
        ) -> Result<Option<TestState>, Self::Error> {
            Ok(self
                .states_by_checkpoint
                .lock()
                .unwrap()
                .get(&checkpoint_id)
                .cloned())
        }
    }

    struct TestContext_ {
        session_store: InMemorySessionStore,
        snapshot_store: InMemorySnapshotStore,
        state_store: InMemoryStateStore,
    }

    impl CheckpointStoreContext<TestNode> for TestContext_ {
        type SessionStore = InMemorySessionStore;
        type SnapshotStore = InMemorySnapshotStore;
        type StateStore = InMemoryStateStore;

        fn session_store(&self) -> Self::SessionStore {
            self.session_store.clone()
        }

        fn snapshot_store(&self) -> Self::SnapshotStore {
            self.snapshot_store.clone()
        }

        fn state_store(&self) -> Self::StateStore {
            self.state_store.clone()
        }
    }

    #[derive(Clone, Default)]
    struct TestHandle {
        session_store: InMemorySessionStore,
        snapshot_store: InMemorySnapshotStore,
        state_store: InMemoryStateStore,
    }

    #[async_trait::async_trait]
    impl CheckpointStoreHandle<TestNode> for TestHandle {
        type Context<'a>
            = TestContext_
        where
            Self: 'a;

        async fn run<F, R>(&self, f: F) -> Result<R, CheckpointError>
        where
            F: for<'a> FnOnce(
                    &'a Self::Context<'a>,
                )
                    -> futures::future::BoxFuture<'a, Result<R, CheckpointError>>
                + Send,
            R: Send,
        {
            f(&TestContext_ {
                session_store: self.session_store.clone(),
                snapshot_store: self.snapshot_store.clone(),
                state_store: self.state_store.clone(),
            })
            .await
        }
    }

    fn make_ids() -> (String, Uuid, Uuid) {
        ("tenant".to_string(), Uuid::new_v4(), Uuid::new_v4())
    }

    #[tokio::test]
    async fn load_produces_initial_for_fresh_run() {
        let handle = TestHandle::default();
        let checkpointer = GraphCheckpointer::new(handle);
        let (tenant_id, session_id, run_id) = make_ids();

        let result = checkpointer
            .load(LoadCheckpointParams {
                tenant_id,
                session_id,
                run_id,
                input: TestStateInput::default(),
                checkpoint_id: None,
            })
            .await
            .unwrap();

        assert!(matches!(result, Checkpoint::Initial(_)));
    }

    #[tokio::test]
    async fn load_produces_resume_after_completed_run() {
        let handle = TestHandle::default();
        let (tenant_id, session_id, run_id) = make_ids();
        let cid = Uuid::new_v4();

        handle
            .snapshot_store
            .save_snapshot(CheckpointSnapshot {
                checkpoint_id: cid,
                tenant_id: tenant_id.clone(),
                session_id,
                run_id,
                node: "c".to_string(),
                status: RunStatus::Completed,
                reason: CheckpointReason::Finish,
                created_at: Utc::now(),
                parent_checkpoint_id: None,
                metadata: None,
            })
            .await
            .unwrap();

        handle
            .state_store
            .save(
                &tenant_id,
                session_id,
                run_id,
                cid,
                TestState {
                    visited: vec!["a".to_string(), "b".to_string(), "c".to_string()],
                    counter: 3,
                },
            )
            .await
            .unwrap();

        let checkpointer = GraphCheckpointer::new(handle);
        let result = checkpointer
            .load(LoadCheckpointParams {
                tenant_id,
                session_id,
                run_id,
                input: TestStateInput::default(),
                checkpoint_id: None,
            })
            .await
            .unwrap();

        // After a completed run, resume with node=None (restart at entrypoint)
        assert!(matches!(result, Checkpoint::Resume { node: None, .. }));
        if let Checkpoint::Resume { state, node, .. } = result {
            assert_eq!(node, None);
            assert_eq!(state.visited, vec!["a", "b", "c"]);
            assert_eq!(state.counter, 3);
        }
    }

    #[tokio::test]
    async fn load_produces_resume_with_node_for_interrupted_run() {
        let handle = TestHandle::default();
        let (tenant_id, session_id, run_id) = make_ids();
        let cid = Uuid::new_v4();

        handle
            .snapshot_store
            .save_snapshot(CheckpointSnapshot {
                checkpoint_id: cid,
                tenant_id: tenant_id.clone(),
                session_id,
                run_id,
                node: "b".to_string(),
                status: RunStatus::Interrupted,
                reason: CheckpointReason::Interrupt,
                created_at: Utc::now(),
                parent_checkpoint_id: None,
                metadata: None,
            })
            .await
            .unwrap();

        handle
            .state_store
            .save(
                &tenant_id,
                session_id,
                run_id,
                cid,
                TestState {
                    visited: vec!["a".to_string()],
                    counter: 3,
                },
            )
            .await
            .unwrap();

        let checkpointer = GraphCheckpointer::new(handle);
        let result = checkpointer
            .load(LoadCheckpointParams {
                tenant_id,
                session_id,
                run_id,
                input: TestStateInput::default(),
                checkpoint_id: None,
            })
            .await
            .unwrap();

        // After an interrupted run, resume at the interrupted node
        assert!(matches!(result, Checkpoint::Resume { node: Some(TestNode::B), .. }));
        if let Checkpoint::Resume { state, node, .. } = result {
            assert_eq!(node, Some(TestNode::B));
            assert_eq!(state.visited, vec!["a"]);
            assert_eq!(state.counter, 3);
        }
    }

    #[tokio::test]
    async fn load_produces_initial_when_no_prior_snapshot_exists() {
        let handle = TestHandle::default();
        let (tenant_id, session_id, run_id) = make_ids();

        let checkpointer = GraphCheckpointer::new(handle);
        let result = checkpointer
            .load(LoadCheckpointParams {
                tenant_id,
                session_id,
                run_id,
                input: TestStateInput::default(),
                checkpoint_id: None,
            })
            .await
            .unwrap();

        assert!(matches!(result, Checkpoint::Initial(_)));
    }

    #[tokio::test]
    async fn save_persists_snapshot_and_state_together() {
        let handle = TestHandle::default();
        let checkpointer = GraphCheckpointer::new(handle.clone());
        let (tenant_id, session_id, run_id) = make_ids();
        let state = TestState {
            visited: vec!["a".to_string()],
            counter: 1,
        };

        let cid = checkpointer
            .save(SaveCheckpointParams {
                tenant_id: tenant_id.clone(),
                session_id,
                run_id,
                node: "b".to_string(),
                state,
                reason: CheckpointReason::Step,
                parent_checkpoint_id: None,
                metadata: None,
            })
            .await
            .unwrap();

        let saved = handle
            .state_store
            .load(&tenant_id, session_id, run_id, cid)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(saved.counter, 1);

        let snapshot = handle
            .snapshot_store
            .load_snapshot(&tenant_id, cid)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(snapshot.node, "b");
        assert_eq!(snapshot.reason, CheckpointReason::Step);
    }

    #[tokio::test]
    async fn finish_writes_interrupt_snapshot_and_saves_state() {
        let handle = TestHandle::default();
        let checkpointer = GraphCheckpointer::new(handle.clone());
        let (tenant_id, session_id, run_id) = make_ids();

        handle
            .session_store
            .save_session(&tenant_id, session_id)
            .await
            .unwrap();
        handle
            .session_store
            .save_run(&tenant_id, session_id, run_id)
            .await
            .unwrap();

        let state = TestState {
            visited: vec!["a", "b", "c"]
                .into_iter()
                .map(String::from)
                .collect(),
            counter: 3,
        };

        checkpointer
            .finish(FinishCheckpointParams {
                tenant_id: tenant_id.clone(),
                session_id,
                run_id,
                node: "c".to_string(),
                status: RunStatus::Interrupted,
                state,
                parent_checkpoint_id: None,
                metadata: Some(serde_json::json!({"q": "continue?"})),
            })
            .await
            .unwrap();

        let snapshot = handle
            .snapshot_store
            .load_latest_for_session(&tenant_id, session_id)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(snapshot.reason, CheckpointReason::Interrupt);
        assert_eq!(snapshot.status, RunStatus::Interrupted);

        let saved = handle
            .state_store
            .load(&tenant_id, session_id, run_id, snapshot.checkpoint_id)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(saved.visited, vec!["a", "b", "c"]);
        assert_eq!(saved.counter, 3);
    }
}
