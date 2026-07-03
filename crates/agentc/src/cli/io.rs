// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

#![allow(unused)]

use tokio::{
    fs,
    io::{self, AsyncReadExt, AsyncWrite, AsyncWriteExt},
};

use crate::cli::errors::CliError;

/// Resolves an input to either stdin or a file path.
///
/// Construct from a string flag value; `-` means stdin, anything else is a file path.
#[derive(Debug, Clone)]
pub struct InputSource(String);

impl InputSource {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn is_stdin(&self) -> bool {
        self.0 == "-"
    }

    pub async fn read_to_string(&self) -> Result<String, CliError> {
        if self.is_stdin() {
            let mut buf = String::new();
            io::stdin()
                .read_to_string(&mut buf)
                .await
                .map_err(|e| CliError::io_error(format!("failed to read stdin: {e}")))?;
            Ok(buf)
        } else {
            fs::read_to_string(&self.0)
                .await
                .map_err(|e| CliError::io_error(format!("failed to read '{}': {e}", self.0)))
        }
    }

    pub async fn read_bytes(&self) -> Result<Vec<u8>, CliError> {
        if self.is_stdin() {
            let mut buf = Vec::new();
            io::stdin()
                .read_to_end(&mut buf)
                .await
                .map_err(|e| CliError::io_error(format!("failed to read stdin: {e}")))?;
            Ok(buf)
        } else {
            fs::read(&self.0)
                .await
                .map_err(|e| CliError::io_error(format!("failed to read '{}': {e}", self.0)))
        }
    }
}

impl From<String> for InputSource {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<&str> for InputSource {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

/// Resolves an output to either stdout or a file path.
///
/// Construct from a string flag value; `-` means stdout, anything else is a file path.
#[derive(Debug, Clone)]
pub struct OutputTarget(String);

impl OutputTarget {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn is_stdout(&self) -> bool {
        self.0 == "-"
    }

    pub async fn write(&self, content: &str) -> Result<(), CliError> {
        if self.is_stdout() {
            io::stdout()
                .write_all(content.as_bytes())
                .await
                .map_err(|e| CliError::io_error(format!("failed to write stdout: {e}")))?;

            io::stdout()
                .write_all(b"\n")
                .await
                .map_err(|e| CliError::io_error(format!("failed to write stdout: {e}")))?;
        } else {
            fs::write(&self.0, format!("{content}\n"))
                .await
                .map_err(|e| CliError::io_error(format!("failed to write '{}': {e}", self.0)))?;
        }

        Ok(())
    }

    /// An async byte sink for streaming raw output: stdout when `-`, otherwise a
    /// created (truncated) file.
    pub async fn writer(&self) -> Result<Box<dyn AsyncWrite + Unpin + Send>, CliError> {
        if self.is_stdout() {
            Ok(Box::new(io::stdout()))
        } else {
            fs::File::create(&self.0)
                .await
                .map(|file| Box::new(file) as Box<dyn AsyncWrite + Unpin + Send>)
                .map_err(|e| CliError::io_error(format!("failed to create '{}': {e}", self.0)))
        }
    }
}

impl From<String> for OutputTarget {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<&str> for OutputTarget {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}
