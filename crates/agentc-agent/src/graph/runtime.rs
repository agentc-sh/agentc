// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

#![allow(deprecated)]

use serde_json::Value;
use std::{
    collections::HashMap,
    sync::{Arc, LazyLock},
    time::Instant,
};
use uuid::Uuid;

use agentc_telemetry::{
    Instrument, field, info_span,
    metrics::{Histogram, KeyValue, meter},
    semconv::{self, attribute},
};

use crate::graph::{
    cancel::Canceller,
    checkpoint::{
        Checkpoint, CheckpointReason, Checkpointer, FinishCheckpointParams, LoadCheckpointParams,
        RunStatus, SaveCheckpointParams,
    },
    command::GraphTransition,
    context::RuntimeContext,
    errors::GraphError,
    handler::{GraphNodeFunction, GraphNodeHandler, GraphNodeHandlerFn},
    state::{CtxOf, GraphNode, GraphStateInput, GraphStateUpdate, InputOf, StateOf},
};

static WORKFLOW_DURATION: LazyLock<Histogram<f64>> = LazyLock::new(|| {
    meter("agentc-agent")
        .f64_histogram(semconv::GEN_AI_WORKFLOW_DURATION)
        .with_unit("s")
        .with_description("Duration of workflow (graph) executions.")
        .build()
});

pub struct SessionConfig {
    /// The session ID to associate this run with. All runs part of the same session should share a session ID.
    pub session_id: Uuid,
    /// The unique run ID for this execution. Must be unique across all runs for a given session.
    pub run_id: Uuid,
    /// The tenant ID for this run, used for multi-tenancy in the checkpointer.
    pub tenant_id: String,
    /// For time travel: resume from a specific historical checkpoint rather than the latest.
    pub checkpoint_id: Option<Uuid>,
    /// The resume payload to inject into the first node execution of this run.
    /// When set, the [`Interrupt`](crate::graph::context::Interrupt) extractor will return
    /// `Ok(resume_payload)` instead of raising
    /// [`GraphError::Interrupt`](crate::graph::errors::GraphError::Interrupt).
    /// Consumed after the first successful node execution.
    pub resume_payload: Option<Value>,
}

#[derive(Debug)]
pub enum RunOutcome<S> {
    Completed(S),
    Interrupted { state: S, payload: Option<Value> },
    Cancelled { state: S },
}

impl<S> RunOutcome<S> {
    pub fn completed(state: S) -> Self {
        RunOutcome::Completed(state)
    }

    pub fn interrupted(state: S, payload: Option<Value>) -> Self {
        RunOutcome::Interrupted { state, payload }
    }

    pub fn cancelled(state: S) -> Self {
        RunOutcome::Cancelled { state }
    }

    pub fn is_completed(&self) -> bool {
        matches!(self, RunOutcome::Completed(_))
    }

    pub fn is_interrupted(&self) -> bool {
        matches!(self, RunOutcome::Interrupted { .. })
    }

    pub fn is_cancelled(&self) -> bool {
        matches!(self, RunOutcome::Cancelled { .. })
    }

    pub fn into_state(self) -> S {
        match self {
            RunOutcome::Completed(result) => result,
            RunOutcome::Interrupted { state, .. } => state,
            RunOutcome::Cancelled { state } => state,
        }
    }
}
/// A graph of nodes representing a workflow, where each node has an associated handler that produces a command
/// indicating the next node to transition to and any state updates to apply. Optionally integrates with a checkpointer
/// to persist state and support resuming interrupted runs.
pub struct Graph<N>
where
    N: GraphNode,
{
    entrypoint: N,
    nodes: HashMap<N, Arc<dyn GraphNodeHandler<N>>>,
    checkpointer: Option<Arc<dyn Checkpointer<N>>>,
    cancellation: Option<Arc<dyn Canceller>>,
    name: Option<String>,
}

impl<N> Graph<N>
where
    N: GraphNode,
{
    pub fn new(entrypoint: N) -> Self {
        Self {
            entrypoint,
            nodes: HashMap::new(),
            checkpointer: None,
            cancellation: None,
            name: None,
        }
    }

    pub fn builder(entrypoint: N) -> GraphBuilder<N> {
        GraphBuilder::new(entrypoint)
    }

    pub fn add_node<H>(&mut self, node: N, handler: H)
    where
        H: GraphNodeHandler<N> + 'static,
    {
        self.nodes
            .insert(node, Arc::new(handler));
    }

    pub fn set_checkpointer<C>(&mut self, checkpointer: C)
    where
        C: Checkpointer<N> + 'static,
    {
        self.checkpointer = Some(Arc::new(checkpointer));
    }

    pub fn set_canceller<C>(&mut self, canceller: C)
    where
        C: Canceller + 'static,
    {
        self.cancellation = Some(Arc::new(canceller));
    }

    pub async fn cancel(&self, tenant_id: &str, run_id: Uuid) -> Result<bool, GraphError> {
        match &self.cancellation {
            Some(canceller) => canceller
                .cancel(tenant_id, run_id)
                .await
                .map_err(|e| GraphError::cancellation_error(e.to_string())),
            None => Err(GraphError::cancellation_error("no canceller configured")),
        }
    }

    pub async fn run(
        &self,
        ctx: CtxOf<N>,
        input: InputOf<N>,
        config: SessionConfig,
    ) -> Result<RunOutcome<StateOf<N>>, GraphError> {
        let name = self.name.clone().unwrap_or_default();
        let span = info_span!(
            "invoke_workflow",
            otel.name = %format!("invoke_workflow {name}"),
            otel.kind = "internal",
            gen_ai.operation.name = "invoke_workflow",
            gen_ai.workflow.name = %name,
            error.type = field::Empty,
        );
        let start = Instant::now();

        let result = self
            .execute(ctx, input, config)
            .instrument(span.clone())
            .await;

        let mut attributes = vec![KeyValue::new(attribute::GEN_AI_WORKFLOW_NAME, name)];
        if let Err(error) = &result {
            span.record("error.type", error.error_type());
            attributes.push(KeyValue::new(attribute::ERROR_TYPE, error.error_type()));
        }

        WORKFLOW_DURATION.record(start.elapsed().as_secs_f64(), &attributes);

        result
    }

    async fn execute(
        &self,
        ctx: CtxOf<N>,
        input: InputOf<N>,
        config: SessionConfig,
    ) -> Result<RunOutcome<StateOf<N>>, GraphError> {
        let (mut state, mut current_node, mut parent_checkpoint_id) = match &self.checkpointer {
            Some(checkpointer) => match checkpointer
                .load(LoadCheckpointParams {
                    tenant_id: config.tenant_id.clone(),
                    session_id: config.session_id,
                    run_id: config.run_id,
                    input,
                    checkpoint_id: config.checkpoint_id,
                })
                .await
                .map_err(GraphError::checkpoint_error)?
            {
                Checkpoint::Initial(state) => {
                    (state, GraphTransition::Node(self.entrypoint.clone()), None)
                }
                Checkpoint::Resume { state, checkpoint_id, node } => (
                    state,
                    GraphTransition::Node(node.unwrap_or_else(|| self.entrypoint.clone())),
                    Some(checkpoint_id),
                ),
            },
            None => (input.initialize(), GraphTransition::Node(self.entrypoint.clone()), None),
        };

        if let Some(checkpointer) = &self.checkpointer
            && let GraphTransition::Node(ref node) = current_node
        {
            parent_checkpoint_id = Some(
                checkpointer
                    .save(SaveCheckpointParams {
                        tenant_id: config.tenant_id.clone(),
                        session_id: config.session_id,
                        run_id: config.run_id,
                        node: node.to_string(),
                        state: state.clone(),
                        reason: CheckpointReason::Input,
                        parent_checkpoint_id,
                        metadata: None,
                    })
                    .await
                    .map_err(GraphError::checkpoint_error)?,
            );
        }

        let mut resume_payload = config.resume_payload;
        let mut last_node = self.entrypoint.to_string();

        while let GraphTransition::Node(ref node) = current_node {
            if let Some(canceller) = &self.cancellation
                && canceller
                    .is_cancelled(&config.tenant_id, config.run_id)
                    .await
                    .map_err(|e| GraphError::cancellation_error(e.to_string()))?
            {
                return Ok(RunOutcome::cancelled(state));
            }

            last_node = node.to_string();

            let handler = match self.nodes.get(node) {
                Some(h) => h,
                None => return Err(GraphError::node_not_found(node)),
            };

            let rtx = RuntimeContext {
                ctx: ctx.clone(),
                state: state.clone(),
                resume_payload: resume_payload.clone(),
            };

            let command = match handler.handle(&rtx).await {
                Ok(cmd) => {
                    resume_payload = None;
                    cmd
                }
                Err(GraphError::Interrupt(payload)) => {
                    if let Some(checkpointer) = &self.checkpointer {
                        checkpointer
                            .finish(FinishCheckpointParams {
                                tenant_id: config.tenant_id.clone(),
                                session_id: config.session_id,
                                run_id: config.run_id,
                                node: last_node,
                                status: RunStatus::Interrupted,
                                state: state.clone(),
                                parent_checkpoint_id,
                                metadata: None,
                            })
                            .await
                            .map_err(GraphError::checkpoint_error)?;
                    }

                    return Ok(RunOutcome::interrupted(state, Some(payload)));
                }
                Err(e) => {
                    if let Some(checkpointer) = &self.checkpointer {
                        checkpointer
                            .finish(FinishCheckpointParams {
                                tenant_id: config.tenant_id.clone(),
                                session_id: config.session_id,
                                run_id: config.run_id,
                                node: last_node,
                                status: RunStatus::Failed,
                                state: state.clone(),
                                parent_checkpoint_id,
                                metadata: None,
                            })
                            .await
                            .map_err(GraphError::checkpoint_error)?;
                    }

                    return Err(e);
                }
            };

            if let Some(update) = command.update {
                update.apply(&mut state);
            }

            if let Some(checkpointer) = &self.checkpointer {
                parent_checkpoint_id = Some(
                    checkpointer
                        .save(SaveCheckpointParams {
                            tenant_id: config.tenant_id.clone(),
                            session_id: config.session_id,
                            run_id: config.run_id,
                            node: node.to_string(),
                            state: state.clone(),
                            reason: CheckpointReason::Step,
                            parent_checkpoint_id,
                            metadata: None,
                        })
                        .await
                        .map_err(GraphError::checkpoint_error)?,
                );
            }

            current_node = command
                .goto
                .unwrap_or(GraphTransition::End);
        }

        if let Some(checkpointer) = &self.checkpointer {
            checkpointer
                .finish(FinishCheckpointParams {
                    tenant_id: config.tenant_id.clone(),
                    session_id: config.session_id,
                    run_id: config.run_id,
                    node: last_node,
                    status: RunStatus::Completed,
                    state: state.clone(),
                    parent_checkpoint_id,
                    metadata: None,
                })
                .await
                .map_err(GraphError::checkpoint_error)?;
        }

        Ok(RunOutcome::completed(state))
    }
}

pub struct GraphBuilder<N>
where
    N: GraphNode,
{
    graph: Graph<N>,
}

impl<N> GraphBuilder<N>
where
    N: GraphNode,
{
    pub fn new(entrypoint: N) -> Self {
        Self { graph: Graph::new(entrypoint) }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.graph.name = Some(name.into());
        self
    }

    pub fn with_node<H>(mut self, node: N, handler: H) -> Self
    where
        H: GraphNodeHandler<N> + 'static,
    {
        self.graph.add_node(node, handler);
        self
    }

    pub fn with_node_fn<F, Args>(mut self, node: N, func: F) -> Self
    where
        N: GraphNode + 'static,
        F: GraphNodeFunction<N, Args> + Send + Sync + 'static,
        Args: Send + 'static,
    {
        self.graph
            .add_node(node, GraphNodeHandlerFn::new(func));
        self
    }

    pub fn with_checkpointer<C>(mut self, checkpointer: C) -> Self
    where
        C: Checkpointer<N> + 'static,
    {
        self.graph
            .set_checkpointer(checkpointer);
        self
    }

    pub fn with_canceller<C>(mut self, canceller: C) -> Self
    where
        C: Canceller + 'static,
    {
        self.graph.set_canceller(canceller);
        self
    }

    pub fn build(self) -> Graph<N> {
        self.graph
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use futures::future::BoxFuture;
    use serde::{Deserialize, Serialize};
    use std::{
        collections::HashMap,
        fmt::{Display, Formatter, Result as FmtResult},
        str::FromStr,
        sync::{
            Arc, Mutex,
            atomic::{AtomicBool, Ordering},
        },
    };
    use uuid::Uuid;

    use crate::graph::{
        cancel::CancellationError,
        checkpoint::{
            CheckpointError, CheckpointReason, CheckpointSnapshot, CheckpointSnapshotStore,
            CheckpointStoreContext, CheckpointStoreHandle, GraphCheckpointer, RunStatus,
            SessionStore, StateStore,
        },
        command::GraphNodeCommand,
        context::State,
        state::{
            GraphContext, GraphNode, GraphState, GraphStateInput, GraphStateUpdate, IntoStateUpdate,
        },
    };

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

    impl TestStateUpdate {
        fn visit(node: impl Into<String>) -> Self {
            Self { visit: Some(node.into()), increment: 0 }
        }

        fn increment(n: u32) -> Self {
            Self { visit: None, increment: n }
        }
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

    impl Display for TestNode {
        fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
            match self {
                TestNode::A => write!(f, "a"),
                TestNode::B => write!(f, "b"),
                TestNode::C => write!(f, "c"),
            }
        }
    }

    impl FromStr for TestNode {
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

    fn config() -> SessionConfig {
        SessionConfig {
            tenant_id: "tenant".to_string(),
            session_id: Uuid::new_v4(),
            run_id: Uuid::new_v4(),
            checkpoint_id: None,
            resume_payload: None,
        }
    }

    #[derive(Debug, Clone, Default)]
    struct InMemorySessionStore {
        sessions: Arc<Mutex<HashMap<Uuid, bool>>>,
        runs: Arc<Mutex<HashMap<Uuid, RunStatus>>>,
    }

    #[derive(Debug, thiserror::Error)]
    #[error("{0}")]
    struct TestError(String);

    impl From<TestError> for CheckpointError {
        fn from(e: TestError) -> Self {
            CheckpointError::checkpoint_store_error(e.to_string())
        }
    }

    #[async_trait]
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

    #[derive(Debug, Clone, Default)]
    struct InMemorySnapshotStore {
        snapshots: Arc<Mutex<HashMap<Uuid, CheckpointSnapshot>>>,
    }

    #[async_trait]
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

    #[derive(Debug, Clone, Default)]
    struct InMemoryStateStore {
        states_by_checkpoint: Arc<Mutex<HashMap<Uuid, TestState>>>,
    }

    #[async_trait]
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

    struct TestStoreContext {
        session_store: InMemorySessionStore,
        snapshot_store: InMemorySnapshotStore,
        state_store: InMemoryStateStore,
    }

    impl CheckpointStoreContext<TestNode> for TestStoreContext {
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

    #[async_trait]
    impl CheckpointStoreHandle<TestNode> for TestHandle {
        type Context<'a>
            = TestStoreContext
        where
            Self: 'a;

        async fn run<F, R>(&self, f: F) -> Result<R, CheckpointError>
        where
            F: for<'a> FnOnce(&'a Self::Context<'a>) -> BoxFuture<'a, Result<R, CheckpointError>>
                + Send,
            R: Send,
        {
            f(&TestStoreContext {
                session_store: self.session_store.clone(),
                snapshot_store: self.snapshot_store.clone(),
                state_store: self.state_store.clone(),
            })
            .await
        }
    }

    #[tokio::test]
    async fn executes_nodes_in_order() {
        let graph = Graph::builder(TestNode::A)
            .with_node_fn(TestNode::A, |_: State<TestState>| async {
                Ok(GraphNodeCommand::goto_and_update(TestNode::B, TestStateUpdate::visit("a")))
            })
            .with_node_fn(TestNode::B, |_: State<TestState>| async {
                Ok(GraphNodeCommand::goto_and_update(TestNode::C, TestStateUpdate::visit("b")))
            })
            .with_node_fn(TestNode::C, |_: State<TestState>| async {
                Ok(GraphNodeCommand::end_and_update(TestStateUpdate::visit("c")))
            })
            .build();

        let outcome = graph
            .run(TestContext, TestStateInput::default(), config())
            .await
            .unwrap();

        assert_eq!(outcome.into_state().visited, vec!["a", "b", "c"]);
    }

    #[tokio::test]
    async fn applies_state_updates_correctly() {
        let graph = Graph::builder(TestNode::A)
            .with_node_fn(TestNode::A, |_: State<TestState>| async {
                Ok(GraphNodeCommand::end_and_update(TestStateUpdate::increment(5)))
            })
            .build();

        let outcome = graph
            .run(TestContext, TestStateInput { initial_counter: 10 }, config())
            .await
            .unwrap();

        assert_eq!(outcome.into_state().counter, 15);
    }

    #[tokio::test]
    async fn returns_node_not_found_for_missing_node() {
        let graph = Graph::builder(TestNode::A)
            .with_node_fn(TestNode::A, |_: State<TestState>| async {
                Ok(GraphNodeCommand::goto(TestNode::B))
            })
            .build();

        let err = graph
            .run(TestContext, TestStateInput::default(), config())
            .await
            .unwrap_err();

        assert!(matches!(err, GraphError::NodeNotFound(_)));
    }

    #[tokio::test]
    async fn end_command_terminates_execution() {
        let graph = Graph::builder(TestNode::A)
            .with_node_fn(TestNode::A, |_: State<TestState>| async {
                Ok(GraphNodeCommand::end_and_update(TestStateUpdate::visit("a")))
            })
            .with_node_fn(TestNode::B, |_: State<TestState>| async {
                Ok(GraphNodeCommand::end_and_update(TestStateUpdate::visit("b_should_not_run")))
            })
            .build();

        let outcome = graph
            .run(TestContext, TestStateInput::default(), config())
            .await
            .unwrap();
        let state = outcome.into_state();

        assert_eq!(state.visited, vec!["a"]);
        assert!(
            !state
                .visited
                .contains(&"b_should_not_run".to_string())
        );
    }

    #[tokio::test]
    async fn no_update_command_leaves_state_unchanged() {
        let graph = Graph::builder(TestNode::A)
            .with_node_fn(TestNode::A, |_: State<TestState>| async { Ok(GraphNodeCommand::end()) })
            .build();

        let outcome = graph
            .run(TestContext, TestStateInput { initial_counter: 42 }, config())
            .await
            .unwrap();
        let state = outcome.into_state();

        assert_eq!(state.counter, 42);
        assert!(state.visited.is_empty());
    }

    // ── Graph + checkpointer integration tests ────────────────────────────────

    #[tokio::test]
    async fn graph_with_checkpointer_saves_state_after_each_node() {
        let handle = TestHandle::default();
        let cfg = SessionConfig {
            tenant_id: "tenant".to_string(),
            session_id: Uuid::new_v4(),
            run_id: Uuid::new_v4(),
            checkpoint_id: None,
            resume_payload: None,
        };
        let run_id = cfg.run_id;
        let session_id = cfg.session_id;

        let graph = Graph::builder(TestNode::A)
            .with_node_fn(TestNode::A, |_: State<TestState>| async {
                Ok(GraphNodeCommand::goto_and_update(TestNode::B, TestStateUpdate::visit("a")))
            })
            .with_node_fn(TestNode::B, |_: State<TestState>| async {
                Ok(GraphNodeCommand::end_and_update(TestStateUpdate::visit("b")))
            })
            .with_checkpointer(GraphCheckpointer::new(handle.clone()))
            .build();

        graph
            .run(TestContext, TestStateInput::default(), cfg)
            .await
            .unwrap()
            .into_state();

        let snapshot = handle
            .snapshot_store
            .load_latest_for_session("tenant", session_id)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(snapshot.status, RunStatus::Completed);

        let persisted = handle
            .state_store
            .load("tenant", session_id, run_id, snapshot.checkpoint_id)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(persisted.visited, vec!["a", "b"]);
    }

    #[tokio::test]
    async fn graph_resumes_interrupted_run_from_correct_node() {
        let handle = TestHandle::default();
        let session_id = Uuid::new_v4();
        let run_id = Uuid::new_v4();
        let cid = Uuid::new_v4();

        handle
            .snapshot_store
            .save_snapshot(CheckpointSnapshot {
                checkpoint_id: cid,
                tenant_id: "tenant".to_string(),
                session_id,
                run_id,
                node: "b".to_string(),
                status: RunStatus::Interrupted,
                reason: CheckpointReason::Interrupt,
                created_at: chrono::Utc::now(),
                parent_checkpoint_id: None,
                metadata: None,
            })
            .await
            .unwrap();

        handle
            .state_store
            .save(
                "tenant",
                session_id,
                run_id,
                cid,
                TestState {
                    visited: vec!["a".to_string()],
                    counter: 0,
                },
            )
            .await
            .unwrap();

        let graph = Graph::builder(TestNode::A)
            .with_node_fn(TestNode::A, |_: State<TestState>| async {
                Ok(GraphNodeCommand::goto_and_update(
                    TestNode::B,
                    TestStateUpdate::visit("a_should_not_run"),
                ))
            })
            .with_node_fn(TestNode::B, |_: State<TestState>| async {
                Ok(GraphNodeCommand::end_and_update(TestStateUpdate::visit("b")))
            })
            .with_checkpointer(GraphCheckpointer::new(handle))
            .build();

        let state = graph
            .run(
                TestContext,
                TestStateInput::default(),
                SessionConfig {
                    tenant_id: "tenant".to_string(),
                    session_id,
                    run_id,
                    checkpoint_id: None,
                    resume_payload: None,
                },
            )
            .await
            .unwrap()
            .into_state();

        assert_eq!(state.visited, vec!["a", "b"]);
        assert!(
            !state
                .visited
                .contains(&"a_should_not_run".to_string())
        );
    }

    #[tokio::test]
    async fn graph_session_resume_carries_over_prior_state() {
        let handle = TestHandle::default();
        let session_id = Uuid::new_v4();
        let prior_run_id = Uuid::new_v4();
        let cid = Uuid::new_v4();

        handle
            .snapshot_store
            .save_snapshot(CheckpointSnapshot {
                checkpoint_id: cid,
                tenant_id: "tenant".to_string(),
                session_id,
                run_id: prior_run_id,
                node: "a".to_string(),
                status: RunStatus::Completed,
                reason: CheckpointReason::Finish,
                created_at: chrono::Utc::now(),
                parent_checkpoint_id: None,
                metadata: None,
            })
            .await
            .unwrap();

        handle
            .state_store
            .save(
                "tenant",
                session_id,
                prior_run_id,
                cid,
                TestState {
                    visited: vec!["prior".to_string()],
                    counter: 10,
                },
            )
            .await
            .unwrap();

        let graph = Graph::builder(TestNode::A)
            .with_node_fn(TestNode::A, |_: State<TestState>| async {
                Ok(GraphNodeCommand::end_and_update(TestStateUpdate::visit("new")))
            })
            .with_checkpointer(GraphCheckpointer::new(handle))
            .build();

        let state = graph
            .run(
                TestContext,
                TestStateInput::default(),
                SessionConfig {
                    tenant_id: "tenant".to_string(),
                    session_id,
                    run_id: Uuid::new_v4(),
                    checkpoint_id: None,
                    resume_payload: None,
                },
            )
            .await
            .unwrap()
            .into_state();

        assert!(
            state
                .visited
                .contains(&"prior".to_string())
        );
        assert!(
            state
                .visited
                .contains(&"new".to_string())
        );
        assert_eq!(state.counter, 10);
    }

    // ── Interrupt tests ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn interrupt_ends_run_cleanly_without_checkpointer() {
        use crate::graph::context::Interrupt;

        let graph = Graph::builder(TestNode::A)
            .with_node_fn(TestNode::A, |interrupt: Interrupt| async move {
                interrupt.interrupt(serde_json::json!({"question": "continue?"}))?;
                Ok(GraphNodeCommand::end_and_update(TestStateUpdate::visit("should_not_reach")))
            })
            .build();

        let outcome = graph
            .run(TestContext, TestStateInput::default(), config())
            .await
            .unwrap();

        assert!(matches!(outcome, RunOutcome::Interrupted { .. }));
        let state = outcome.into_state();
        assert!(state.visited.is_empty());
    }

    #[tokio::test]
    async fn interrupt_sets_run_status_to_interrupted() {
        use crate::graph::context::Interrupt;

        let handle = TestHandle::default();
        let session_id = Uuid::new_v4();
        let run_id = Uuid::new_v4();

        let graph = Graph::builder(TestNode::A)
            .with_node_fn(TestNode::A, |interrupt: Interrupt| async move {
                interrupt.interrupt(serde_json::json!({"question": "continue?"}))?;
                Ok(GraphNodeCommand::end_and_update(TestStateUpdate::visit("should_not_reach")))
            })
            .with_checkpointer(GraphCheckpointer::new(handle.clone()))
            .build();

        graph
            .run(
                TestContext,
                TestStateInput::default(),
                SessionConfig {
                    tenant_id: "tenant".to_string(),
                    session_id,
                    run_id,
                    checkpoint_id: None,
                    resume_payload: None,
                },
            )
            .await
            .unwrap();

        let snapshot = handle
            .snapshot_store
            .load_latest_for_session("tenant", session_id)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(snapshot.status, RunStatus::Interrupted);
    }

    #[tokio::test]
    async fn node_error_sets_run_status_to_failed() {
        let handle = TestHandle::default();
        let session_id = Uuid::new_v4();
        let run_id = Uuid::new_v4();

        let graph = Graph::builder(TestNode::A)
            .with_node_fn(TestNode::A, |_: State<TestState>| async {
                Err::<GraphNodeCommand<TestNode>, GraphError>(GraphError::execution_error_message(
                    "boom",
                ))
            })
            .with_checkpointer(GraphCheckpointer::new(handle.clone()))
            .build();

        let result = graph
            .run(
                TestContext,
                TestStateInput::default(),
                SessionConfig {
                    tenant_id: "tenant".to_string(),
                    session_id,
                    run_id,
                    checkpoint_id: None,
                    resume_payload: None,
                },
            )
            .await;

        assert!(result.is_err());

        let snapshot = handle
            .snapshot_store
            .load_latest_for_session("tenant", session_id)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(snapshot.status, RunStatus::Failed);
    }

    #[tokio::test]
    async fn interrupt_extractor_resumes_with_payload() {
        use crate::graph::context::Interrupt;

        let graph = Graph::builder(TestNode::A)
            .with_node_fn(TestNode::A, |interrupt: Interrupt| async move {
                let resume = interrupt.interrupt(serde_json::json!({"question": "continue?"}))?;
                let answer = resume["answer"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();
                Ok(GraphNodeCommand::end_and_update(TestStateUpdate::visit(answer)))
            })
            .build();

        let state = graph
            .run(
                TestContext,
                TestStateInput::default(),
                SessionConfig {
                    tenant_id: "tenant".to_string(),
                    session_id: Uuid::new_v4(),
                    run_id: Uuid::new_v4(),
                    checkpoint_id: None,
                    resume_payload: Some(serde_json::json!({"answer": "yes"})),
                },
            )
            .await
            .unwrap()
            .into_state();

        assert_eq!(state.visited, vec!["yes"]);
    }

    #[tokio::test]
    async fn resume_payload_consumed_after_first_node() {
        use crate::graph::context::Interrupt;

        // Node A consumes the resume payload via the Interrupt extractor.
        // Node B also declares Interrupt, it should NOT see the payload (it was consumed).
        let graph = Graph::builder(TestNode::A)
            .with_node_fn(TestNode::A, |interrupt: Interrupt| async move {
                // On resume this returns Ok("yes"), consuming the payload.
                interrupt.interrupt(serde_json::json!({}))?;
                Ok(GraphNodeCommand::goto(TestNode::B))
            })
            .with_node_fn(TestNode::B, |interrupt: Interrupt| async move {
                // Should see no resume payload.
                // We catch it here so the test can assert on state instead.
                match interrupt.interrupt(serde_json::json!({"node": "b"})) {
                    Ok(_) => Ok(GraphNodeCommand::end_and_update(TestStateUpdate::visit(
                        "b_saw_payload",
                    ))),
                    Err(GraphError::Interrupt(_)) => {
                        Ok(GraphNodeCommand::end_and_update(TestStateUpdate::visit("b_no_payload")))
                    }
                    Err(e) => Err(e),
                }
            })
            .build();

        let state = graph
            .run(
                TestContext,
                TestStateInput::default(),
                SessionConfig {
                    tenant_id: "tenant".to_string(),
                    session_id: Uuid::new_v4(),
                    run_id: Uuid::new_v4(),
                    checkpoint_id: None,
                    resume_payload: Some(serde_json::json!({"answer": "yes"})),
                },
            )
            .await
            .unwrap()
            .into_state();

        assert_eq!(state.visited, vec!["b_no_payload"]);
    }

    #[derive(Clone, Default)]
    struct StubCanceller {
        cancelled: Arc<AtomicBool>,
    }

    #[async_trait]
    impl Canceller for StubCanceller {
        async fn cancel(&self, _tenant_id: &str, _run_id: Uuid) -> Result<bool, CancellationError> {
            self.cancelled
                .store(true, Ordering::SeqCst);
            Ok(true)
        }

        async fn is_cancelled(
            &self,
            _tenant_id: &str,
            _run_id: Uuid,
        ) -> Result<bool, CancellationError> {
            Ok(self.cancelled.load(Ordering::SeqCst))
        }
    }

    #[tokio::test]
    async fn cancellation_stops_run_before_later_node() {
        let canceller = StubCanceller::default();
        let cancelled = canceller.cancelled.clone();

        let graph = Graph::builder(TestNode::A)
            .with_node_fn(TestNode::A, move |_: State<TestState>| {
                let cancelled = cancelled.clone();
                async move {
                    cancelled.store(true, Ordering::SeqCst);
                    Ok(GraphNodeCommand::goto_and_update(TestNode::B, TestStateUpdate::visit("a")))
                }
            })
            .with_node_fn(TestNode::B, |_: State<TestState>| async {
                Ok(GraphNodeCommand::end_and_update(TestStateUpdate::visit("b_should_not_run")))
            })
            .with_canceller(canceller)
            .build();

        let outcome = graph
            .run(TestContext, TestStateInput::default(), config())
            .await
            .unwrap();

        assert!(matches!(outcome, RunOutcome::Cancelled { .. }));

        let state = outcome.into_state();

        assert_eq!(state.visited, vec!["a"]);
        assert!(
            !state
                .visited
                .contains(&"b_should_not_run".to_string())
        );
    }
}
