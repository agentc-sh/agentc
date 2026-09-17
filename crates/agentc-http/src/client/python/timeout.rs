// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::time::Duration;

use agentc_executor_python::{
    backend::ExecutorBackend,
    guestpy::{
        errors::Error,
        host::exception::{ExceptionClass, Raise},
    },
};

pub(crate) trait FromSeconds: Sized {
    fn from_seconds<B: ExecutorBackend>(seconds: f64) -> Result<Self, Error>;
}

impl FromSeconds for Duration {
    fn from_seconds<B: ExecutorBackend>(seconds: f64) -> Result<Self, Error> {
        if !seconds.is_finite() || seconds < 0.0 {
            return Err(Raise::<B>::new(ExceptionClass::builtin("ValueError"))
                .arg(format!("invalid timeout: {seconds}"))
                .into());
        }

        Ok(Self::from_secs_f64(seconds))
    }
}
