// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::path::Path;
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

/// Creates a symbolic link at `dst` pointing to `src`.
pub async fn symlink(src: &Path, dst: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        tokio::fs::symlink(src, dst).await
    }

    #[cfg(windows)]
    {
        // Windows has distinct file and directory symlink types and no unified
        // constructor, so the target kind must be resolved before linking.
        if tokio::fs::metadata(src).await?.is_dir() {
            tokio::fs::symlink_dir(src, dst).await
        } else {
            tokio::fs::symlink_file(src, dst).await
        }
    }
}
