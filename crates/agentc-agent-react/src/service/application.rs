// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::sync::Arc;

use agentc_agent::agent::Agent;
use agentc_database::Database;
use agentc_domain_sql::scope::SqlScopeFactory;

use crate::{
    graph::ReActNode,
    types::{event::Event, message::Message},
};

#[derive(Clone)]
pub struct ApplicationService {
    pub(crate) agent: Arc<Agent<ReActNode, Event, Message>>,
    pub(crate) scope_factory: Arc<SqlScopeFactory>,
}

impl ApplicationService {
    pub fn new(
        agent: Arc<Agent<ReActNode, Event, Message>>,
        scope_factory: Arc<SqlScopeFactory>,
    ) -> Self {
        Self { agent, scope_factory }
    }

    pub fn builder() -> ApplicationServiceBuilder {
        ApplicationServiceBuilder::new()
    }
}

pub struct ApplicationServiceBuilder {
    agent: Option<Agent<ReActNode, Event, Message>>,
    database: Option<Arc<Database>>,
}

impl Default for ApplicationServiceBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ApplicationServiceBuilder {
    pub fn new() -> Self {
        Self { agent: None, database: None }
    }

    pub fn with_agent(mut self, agent: Agent<ReActNode, Event, Message>) -> Self {
        self.agent = Some(agent);
        self
    }

    pub fn with_database(mut self, database: Arc<Database>) -> Self {
        self.database = Some(database);
        self
    }

    pub fn build(self) -> ApplicationService {
        ApplicationService::new(
            Arc::new(
                self.agent
                    .expect("agent is required for application service"),
            ),
            Arc::new(SqlScopeFactory::new(
                self.database
                    .expect("database is required for application service"),
            )),
        )
    }
}
