// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{collections::HashMap, time::Duration};

use bashkit::ExecutionLimits;

#[derive(Debug, Clone, Default)]
pub enum CommandPolicy {
    #[default]
    Unrestricted,
    Allow(Vec<String>),
}

#[derive(Debug, Clone, Default)]
pub enum EnvPolicy {
    Inherit,
    Allow(Vec<String>),
    Deny(Vec<String>),
    #[default]
    Empty,
}

impl EnvPolicy {
    pub fn resolve(&self) -> HashMap<String, String> {
        match self {
            EnvPolicy::Inherit => std::env::vars().collect(),
            EnvPolicy::Allow(keys) => std::env::vars()
                .filter(|(key, _)| keys.contains(key))
                .collect(),
            EnvPolicy::Deny(keys) => std::env::vars()
                .filter(|(key, _)| !keys.contains(key))
                .collect(),
            EnvPolicy::Empty => HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExecLimits {
    pub max_execution_time: Duration,
    pub max_output_size: usize,
    pub max_command_count: usize,
    pub max_loop_iterations: usize,
}

impl Default for ExecLimits {
    fn default() -> Self {
        ExecLimits {
            max_execution_time: Duration::from_secs(30),
            max_output_size: 10 * 1024 * 1024,
            max_command_count: 10_000,
            max_loop_iterations: 10_000,
        }
    }
}

impl From<&ExecLimits> for ExecutionLimits {
    fn from(limits: &ExecLimits) -> Self {
        ExecutionLimits {
            timeout: limits.max_execution_time,
            max_commands: limits.max_command_count,
            max_loop_iterations: limits.max_loop_iterations,
            max_stdout_bytes: limits.max_output_size,
            max_stderr_bytes: limits.max_output_size,
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone)]
pub struct BashConfig {
    pub command_policy: CommandPolicy,
    pub env_policy: EnvPolicy,
    pub limits: ExecLimits,
    pub cwd: String,
}

impl Default for BashConfig {
    fn default() -> Self {
        BashConfig {
            command_policy: CommandPolicy::default(),
            env_policy: EnvPolicy::default(),
            limits: ExecLimits::default(),
            cwd: String::from("/"),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bashkit::ExecutionLimits;

    use crate::bash::config::ExecLimits;

    #[test]
    fn maps_tool_limits_to_bashkit_limits() {
        let limits = ExecutionLimits::from(&ExecLimits {
            max_execution_time: Duration::from_secs(7),
            max_output_size: 512,
            max_command_count: 23,
            max_loop_iterations: 29,
        });

        assert_eq!(limits.timeout, Duration::from_secs(7));
        assert_eq!(limits.max_stdout_bytes, 512);
        assert_eq!(limits.max_stderr_bytes, 512);
        assert_eq!(limits.max_commands, 23);
        assert_eq!(limits.max_loop_iterations, 29);
    }
}
