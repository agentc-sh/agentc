// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use futures::stream::{Stream, StreamExt};
use std::{
    pin::Pin,
    task::{Context, Poll},
};

use crate::orm::{
    ConnectionTrait, DatabaseConnection, DatabaseTransaction, DbBackend, DbErr, EntityTrait,
    ExecResult, FromQueryResult, QueryResult, QueryStream, QueryTrait, Select, Statement,
    StreamTrait, TransactionStream,
};

#[derive(Clone, Copy)]
pub enum ConnectionContext<'a> {
    Connection(&'a DatabaseConnection),
    Transaction(&'a DatabaseTransaction),
}

impl<'a> ConnectionContext<'a> {
    pub fn as_connection(&self) -> &dyn ConnectionTrait {
        match self {
            ConnectionContext::Connection(conn) => *conn,
            ConnectionContext::Transaction(tx) => *tx,
        }
    }

    pub async fn stream(&self, stmt: Statement) -> Result<ContextStream<'a>, DbErr> {
        match self {
            ConnectionContext::Connection(conn) => conn
                .stream(stmt)
                .await
                .map(ContextStream::Connection),
            ConnectionContext::Transaction(tx) => tx
                .stream(stmt)
                .await
                .map(ContextStream::Transaction),
        }
    }
}

#[async_trait]
impl<'a> ConnectionTrait for ConnectionContext<'a> {
    fn get_database_backend(&self) -> DbBackend {
        self.as_connection()
            .get_database_backend()
    }

    async fn execute(&self, stmt: Statement) -> Result<ExecResult, DbErr> {
        self.as_connection().execute(stmt).await
    }

    async fn execute_unprepared(&self, sql: &str) -> Result<ExecResult, DbErr> {
        self.as_connection()
            .execute_unprepared(sql)
            .await
    }

    async fn query_one(&self, stmt: Statement) -> Result<Option<QueryResult>, DbErr> {
        self.as_connection()
            .query_one(stmt)
            .await
    }

    async fn query_all(&self, stmt: Statement) -> Result<Vec<QueryResult>, DbErr> {
        self.as_connection()
            .query_all(stmt)
            .await
    }
}

pub enum ContextStream<'a> {
    Connection(QueryStream),
    Transaction(TransactionStream<'a>),
}

impl<'a> Stream for ContextStream<'a> {
    type Item = Result<QueryResult, DbErr>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.get_mut() {
            ContextStream::Connection(stream) => Pin::new(stream).poll_next(cx),
            ContextStream::Transaction(stream) => Pin::new(stream).poll_next(cx),
        }
    }
}

#[async_trait]
pub trait StreamContextExt<M>
where
    M: FromQueryResult + Send + 'static,
{
    async fn stream_ctx<'a>(
        self,
        ctx: &ConnectionContext<'a>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<M, DbErr>> + Send + 'a>>, DbErr>;
}

#[async_trait]
impl<E, M> StreamContextExt<M> for Select<E>
where
    E: EntityTrait<Model = M>,
    M: FromQueryResult + Send + 'static,
{
    async fn stream_ctx<'a>(
        self,
        ctx: &ConnectionContext<'a>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<M, DbErr>> + Send + 'a>>, DbErr> {
        Ok(Box::pin(
            ctx.stream(self.build(ctx.get_database_backend()))
                .await?
                .map(|res| res.and_then(|qr| M::from_query_result(&qr, ""))),
        ))
    }
}

impl<'a> From<&'a DatabaseConnection> for ConnectionContext<'a> {
    fn from(conn: &'a DatabaseConnection) -> Self {
        ConnectionContext::Connection(conn)
    }
}

impl<'a> From<&'a DatabaseTransaction> for ConnectionContext<'a> {
    fn from(tx: &'a DatabaseTransaction) -> Self {
        ConnectionContext::Transaction(tx)
    }
}
