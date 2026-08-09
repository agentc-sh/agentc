// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;

use crate::path::{Component, Path, PathBuf};

#[derive(Default)]
pub(crate) struct Whiteouts {
    paths: BTreeSet<PathBuf>,
    opaque_directories: BTreeSet<PathBuf>,
}

impl Whiteouts {
    pub(crate) fn new() -> Self {
        Whiteouts {
            paths: BTreeSet::new(),
            opaque_directories: BTreeSet::new(),
        }
    }

    pub(crate) fn contains(&self, path: &Path) -> bool {
        self.paths
            .contains(&PathBuf::from(path))
    }

    pub(crate) fn child_contains(&self, parent: &Path, child: &Component) -> bool {
        match PathBuf::from(parent).join(child.as_bytes()) {
            Ok(path) => self.paths.contains(&path),
            Err(_) => false,
        }
    }

    pub(crate) fn insert(&mut self, path: impl Into<PathBuf>) {
        self.paths.insert(path.into());
    }

    pub(crate) fn remove(&mut self, path: &Path) {
        self.paths.remove(&PathBuf::from(path));
    }

    pub(crate) fn is_opaque(&self, path: &Path) -> bool {
        self.opaque_directories
            .contains(&PathBuf::from(path))
    }

    pub(crate) fn opaque(&mut self, path: impl Into<PathBuf>) {
        self.opaque_directories
            .insert(path.into());
    }
}
