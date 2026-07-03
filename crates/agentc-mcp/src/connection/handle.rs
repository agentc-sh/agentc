// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use http::{HeaderName, HeaderValue};
use rmcp::{
    model::{CallToolRequestParams, Content, Tool},
    service::{RoleClient, RunningService},
    transport::{
        TokioChildProcess,
        streamable_http_client::{
            StreamableHttpClientTransport, StreamableHttpClientTransportConfig,
        },
    },
};
use serde_json::{Value, to_value};
use std::{collections::HashMap, str::FromStr, sync::Arc};
use tokio::{
    process::Command,
    sync::{Mutex, RwLock},
    time::{sleep, timeout},
};

use agentc_telemetry::{debug, warn};

use crate::{
    config::{McpServerConfig, McpTransport},
    errors::McpError,
};

/// A live connection handle to a single MCP server.
///
/// Holds a [`rmcp::Peer<rmcp::service::RoleClient>`](rmcp::Peer) cloned from the
/// [`rmcp::service::RunningService`](rmcp::service::RunningService) and reconnects transparently on transport
/// failures. Tool definitions discovered at connect time are cached and
/// refreshed after each successful reconnect.
///
/// All methods are safe to call concurrently. Only one reconnect attempt runs
/// at a time; others wait until it completes before proceeding.
pub struct McpServerHandle {
    config: McpServerConfig,

    /// The live peer used for all MCP requests. `None` while disconnected.
    ///
    /// [`rmcp::Peer`] is cheap to clone (wraps an `Arc` internally), so callers
    /// read-lock, clone, then immediately release the lock before awaiting.
    peer: Arc<RwLock<Option<rmcp::Peer<RoleClient>>>>,

    /// Owns the background service task so the transport is not dropped.
    service: Arc<Mutex<Option<RunningService<RoleClient, ()>>>>,

    /// Serializes concurrent reconnect attempts.
    reconnect_lock: Mutex<()>,

    /// Tool definitions cached from the last successful [`rmcp::Peer::list_all_tools`] call.
    cached_tools: RwLock<Vec<Tool>>,
}

impl McpServerHandle {
    /// Connect to the MCP server described by `config`, run the initialize
    /// handshake, and populate the tool cache.
    pub async fn connect(config: McpServerConfig) -> Result<Arc<Self>, McpError> {
        let (peer, service, tools) = Self::establish(&config).await?;

        Ok(Arc::new(Self {
            config,
            peer: Arc::new(RwLock::new(Some(peer))),
            service: Arc::new(Mutex::new(Some(service))),
            reconnect_lock: Mutex::new(()),
            cached_tools: RwLock::new(tools),
        }))
    }

    /// Returns the configuration used to construct this handle.
    pub fn config(&self) -> &McpServerConfig {
        &self.config
    }

    /// Returns the tool definitions cached from the last successful connect or reconnect.
    pub async fn cached_tools(&self) -> Vec<Tool> {
        self.cached_tools.read().await.clone()
    }

    /// Returns `true` if the underlying transport appears to be open.
    pub async fn is_healthy(&self) -> bool {
        self.peer
            .read()
            .await
            .as_ref()
            .map(|p| !p.is_transport_closed())
            .unwrap_or(false)
    }

    /// Execute a single MCP tool call by its original (un-prefixed) name.
    ///
    /// On a transport or protocol error the handle reconnects according to
    /// [`McpServerConfig::reconnect`] and retries the call once. Tool-level
    /// errors reported by the server (`is_error: true`) are returned as
    /// [`McpError::ToolExecutionFailed`] without triggering a reconnect.
    pub async fn call_tool(&self, mcp_name: &str, arguments: Value) -> Result<Value, McpError> {
        let params = build_params(mcp_name, arguments);

        match self.try_call(params.clone()).await {
            Ok(value) => return Ok(value),
            Err(e) if e.is_connection_error() => debug!(
                event = "AttemptingReconnect",
                server = %self.config.name,
                tool = mcp_name,
                error = %e,
            ),
            Err(e) => return Err(e),
        }

        self.reconnect().await?;
        self.try_call(params).await
    }

    /// Attempt a single tool call using the currently cached peer.
    async fn try_call(&self, params: CallToolRequestParams) -> Result<Value, McpError> {
        let peer = self
            .peer
            .read()
            .await
            .clone()
            .ok_or_else(|| McpError::protocol(&self.config.name, "peer is not connected"))?;

        let result = timeout(self.config.tool_call_timeout, peer.call_tool(params.clone()))
            .await
            .map_err(|_| {
                McpError::timed_out(
                    &self.config.name,
                    params.name.as_ref(),
                    self.config.tool_call_timeout,
                )
            })?
            .map_err(|e| McpError::transport(&self.config.name, e))?;

        if result.is_error.unwrap_or(false) {
            return Err(McpError::execution_failed(
                &self.config.name,
                params.name.as_ref(),
                extract_text(&result.content),
            ));
        }

        Ok(result
            .structured_content
            .unwrap_or_else(|| to_value(&result.content).unwrap_or(Value::Null)))
    }

    /// Reconnect to the server, refreshing the peer and tool cache.
    async fn reconnect(&self) -> Result<(), McpError> {
        let _guard = self.reconnect_lock.lock().await;

        if self.is_healthy().await {
            debug!(
                event = "ReconnectNotNeeded",
                server = %self.config.name,
                reason = "peer already healthy",
            );
            return Ok(());
        }

        let mut backoff = self.config.reconnect.initial_backoff;
        let mut last_error: Option<McpError> = None;

        for attempt in 1..=self.config.reconnect.max_retries {
            debug!(
                event = "ReconnectAttempt",
                server = %self.config.name,
                attempt,
            );

            if let Some(mut old) = self.service.lock().await.take() {
                let _ = old.close().await;
            }

            *self.peer.write().await = None;

            match Self::establish(&self.config).await {
                Ok((peer, service, tools)) => {
                    *self.peer.write().await = Some(peer);
                    *self.service.lock().await = Some(service);
                    *self.cached_tools.write().await = tools;

                    debug!(
                        event = "ReconnectSuccessful",
                        server = %self.config.name,
                        attempt,
                    );

                    return Ok(());
                }
                Err(e) => {
                    warn!(
                        event = "ReconnectFailed",
                        server = %self.config.name,
                        attempt,
                        error = %e,
                    );

                    last_error = Some(e);
                    sleep(backoff).await;
                    backoff = backoff
                        .mul_f64(self.config.reconnect.multiplier)
                        .min(self.config.reconnect.max_backoff);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            McpError::unavailable(&self.config.name, self.config.reconnect.max_retries)
        }))
    }

    /// Create a new transport, run the MCP initialize handshake, and return the
    /// live peer, the running service, and discovered tools.
    async fn establish(
        config: &McpServerConfig,
    ) -> Result<(rmcp::Peer<RoleClient>, RunningService<RoleClient, ()>, Vec<Tool>), McpError> {
        let running = timeout(config.connect_timeout, Self::connect_transport(config))
            .await
            .map_err(|_| {
                McpError::connection_failed(
                    &config.name,
                    std::io::Error::other("connect timeout exceeded"),
                )
            })??;

        let peer = running.peer().clone();

        let tools = peer
            .list_all_tools()
            .await
            .map_err(|e| McpError::connection_failed(&config.name, e))?;

        debug!(
            event = "Connected",
            server = %config.name,
            tool_count = tools.len(),
        );

        Ok((peer, running, tools))
    }

    /// Establish the transport and complete the MCP initialize handshake.
    async fn connect_transport(
        config: &McpServerConfig,
    ) -> Result<RunningService<RoleClient, ()>, McpError> {
        match &config.transport {
            McpTransport::Stdio { command, args, env } => {
                let mut cmd = Command::new(command);
                cmd.args(args);

                for (k, v) in env {
                    cmd.env(k, v);
                }

                rmcp::serve_client(
                    (),
                    TokioChildProcess::new(cmd)
                        .map_err(|e| McpError::transport(&config.name, e))?,
                )
                .await
                .map_err(|e| McpError::connection_failed(&config.name, e))
            }

            McpTransport::StreamableHttp { url, auth_token, headers } => {
                let mut transport_config = match auth_token {
                    Some(token) => StreamableHttpClientTransportConfig::with_uri(url.as_str())
                        .auth_header(token),
                    None => StreamableHttpClientTransportConfig::with_uri(url.as_str()),
                };

                transport_config = transport_config.custom_headers(
                    headers
                        .iter()
                        .map(|(k, v)| {
                            Ok((
                                HeaderName::from_str(k)
                                    .map_err(|e| McpError::transport(&config.name, e))?,
                                HeaderValue::from_str(v)
                                    .map_err(|e| McpError::transport(&config.name, e))?,
                            ))
                        })
                        .collect::<Result<HashMap<_, _>, _>>()?,
                );

                rmcp::serve_client((), StreamableHttpClientTransport::from_config(transport_config))
                    .await
                    .map_err(|e| McpError::connection_failed(&config.name, e))
            }
        }
    }
}

/// Build a [`CallToolRequestParams`] from a tool name and JSON arguments value.
fn build_params(name: &str, arguments: Value) -> CallToolRequestParams {
    let mut params = CallToolRequestParams::new(name.to_string());
    params.arguments = match arguments {
        Value::Object(map) => Some(map),
        Value::Null => None,
        other => {
            let mut map = serde_json::Map::new();
            map.insert("value".to_string(), other);
            Some(map)
        }
    };
    params
}

/// Extract the text from the first text content block in `content`, if any.
fn extract_text(content: &[Content]) -> String {
    content
        .iter()
        .find_map(|c| c.as_text().map(|t| t.text.clone()))
        .unwrap_or_default()
}
