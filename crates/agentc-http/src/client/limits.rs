// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{sync::Arc, time::Duration};

use tokio::sync::Semaphore;

#[derive(Clone, Default)]
pub(crate) struct Limits {
    pub(crate) request_timeout: Option<Duration>,
    pub(crate) max_response_bytes: Option<u64>,
    pub(crate) concurrency: Option<Arc<Semaphore>>,
}
