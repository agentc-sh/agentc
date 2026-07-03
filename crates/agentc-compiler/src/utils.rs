// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::process::Stdio;
use tokio::process::Command;

pub async fn command_exists(cmd: &str) -> bool {
    #[cfg(unix)]
    let check = Command::new("which")
        .arg(cmd)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .await;

    #[cfg(windows)]
    let check = Command::new("where")
        .arg(cmd)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .await;

    match check {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}
