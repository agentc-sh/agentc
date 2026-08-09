// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use include_dir::Dir as IncludeDir;

pub struct EmbeddedDirectory {
    inner: IncludeDir<'static>,
}

impl EmbeddedDirectory {
    #[doc(hidden)]
    pub fn __new(inner: IncludeDir<'static>) -> Self {
        EmbeddedDirectory { inner }
    }

    pub(crate) fn as_inner(&self) -> &IncludeDir<'static> {
        &self.inner
    }
}

pub enum EmbeddedSource {
    File { bytes: &'static [u8] },
    Directory { directory: EmbeddedDirectory },
}
