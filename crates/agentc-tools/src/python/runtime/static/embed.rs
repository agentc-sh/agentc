// SPDX-FileCopyrightText: 2026 Timothy Pogue
//
// SPDX-License-Identifier: LicenseRef-Proprietary

use include_dir::Dir;
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::python::runtime::errors::RuntimeError;

/// A directory tree embedded into the binary at compile time (via
/// [`embed_dir`](crate::python::runtime::r#static::embed_dir)), destined to be
/// unpacked onto disk when a
/// [`StaticRuntime`](crate::python::runtime::r#static::StaticRuntime) is built.
///
/// The `static` backend treats tools and their dependencies as fully embedded, exactly
/// like the `embedded` backend; the only difference is that the bytes are unpacked to a
/// temporary directory at runtime and placed on CPython's import path.
pub struct EmbeddedTree {
    dir: Dir<'static>,
}

impl EmbeddedTree {
    pub(super) fn extract(&self, base: &Path) -> Result<(), RuntimeError> {
        self.dir
            .extract(base)
            .map_err(RuntimeError::io)
    }
}

impl From<Dir<'static>> for EmbeddedTree {
    fn from(dir: Dir<'static>) -> Self {
        Self { dir }
    }
}

/// A temporary directory that holds the unpacked [`EmbeddedTree`]s for the lifetime of a
/// runtime and removes them from disk on drop.
pub(super) struct StagingDir {
    root: PathBuf,
}

impl StagingDir {
    /// Unpack each embedded tree into its own numbered subdirectory and return the staging
    /// directory alongside those subdirectory paths, which the workers place on `sys.path`.
    pub(super) fn unpack(trees: &[EmbeddedTree]) -> Result<(Self, Vec<String>), RuntimeError> {
        let staging = Self {
            root: std::env::temp_dir().join(Self::unique_name()),
        };

        std::fs::create_dir_all(&staging.root).map_err(RuntimeError::io)?;

        let mut paths = Vec::with_capacity(trees.len());

        for (index, tree) in trees.iter().enumerate() {
            let path = staging.root.join(index.to_string());

            std::fs::create_dir_all(&path).map_err(RuntimeError::io)?;
            tree.extract(&path)?;

            paths.push(path.to_string_lossy().into_owned());
        }

        Ok((staging, paths))
    }

    /// A process-unique directory name so concurrent runtimes never collide on disk.
    fn unique_name() -> String {
        static COUNTER: AtomicU64 = AtomicU64::new(0);

        format!(
            "agentc-static-{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
            COUNTER.fetch_add(1, Ordering::Relaxed),
        )
    }
}

impl Drop for StagingDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
