// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct CompileParams {
    pub project_dir: PathBuf,
    pub target_dir: PathBuf,
    pub cache_dir: Option<PathBuf>,
    pub release: bool,
    pub verbose: bool,
    pub args: Vec<String>,
}

impl CompileParams {
    pub fn new(project_dir: impl Into<PathBuf>, target_dir: impl Into<PathBuf>) -> Self {
        Self {
            project_dir: project_dir.into(),
            target_dir: target_dir.into(),
            cache_dir: None,
            release: false,
            verbose: false,
            args: Vec::new(),
        }
    }

    pub fn with_release(mut self, release: bool) -> Self {
        self.release = release;
        self
    }

    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    pub fn maybe_with_verbose(mut self, verbose: Option<bool>) -> Self {
        if let Some(verbose) = verbose {
            self.verbose = verbose;
        }
        self
    }

    pub fn with_cache_dir(mut self, cache_dir: impl Into<PathBuf>) -> Self {
        self.cache_dir = Some(cache_dir.into());
        self
    }

    pub fn maybe_with_cache_dir(mut self, cache_dir: Option<impl Into<PathBuf>>) -> Self {
        if let Some(dir) = cache_dir {
            self.cache_dir = Some(dir.into());
        }
        self
    }

    pub fn with_arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn with_args<I, A>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = A>,
        A: Into<String>,
    {
        self.args
            .extend(args.into_iter().map(Into::into));
        self
    }
}
