// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::time::Instant;

use agentc_core::run::types::RunEvent;

use crate::cli::ui::traits::{LogViewport, Spinner, StreamRenderer, Ui};

pub struct RunStreamRenderer<U: Ui> {
    ui: U,
    verbose: bool,
    start: Instant,
    agent_name: Option<String>,
    tool_count: Option<usize>,
    current_spinner: Option<Box<dyn Spinner>>,
    current_viewport: Option<Box<dyn LogViewport>>,
    compiler_stderr: String,
    transformer_stderr: String,
}

impl<U: Ui> RunStreamRenderer<U> {
    pub fn new(ui: U, verbose: bool) -> Self {
        Self {
            ui,
            verbose,
            start: Instant::now(),
            agent_name: None,
            tool_count: None,
            current_spinner: None,
            current_viewport: None,
            compiler_stderr: String::new(),
            transformer_stderr: String::new(),
        }
    }

    fn elapsed(&self) -> String {
        let secs = self.start.elapsed().as_secs_f32();
        format!("{secs:.1}s")
    }

    fn agent(&self) -> &str {
        self.agent_name
            .as_deref()
            .unwrap_or("agent")
    }

    fn update_active(&self, msg: &str) {
        if let Some(vp) = &self.current_viewport {
            vp.update_header(msg);
        } else if let Some(sp) = &self.current_spinner {
            sp.update(msg);
        }
    }

    fn finish_active(&mut self, label: &str) {
        if let Some(vp) = self.current_viewport.take() {
            vp.finish(label);
        } else if let Some(sp) = self.current_spinner.take() {
            sp.finish(label);
        }
    }

    fn fail_active(&mut self, detail: &str) {
        if let Some(vp) = self.current_viewport.take() {
            vp.finish_failure(detail);
        } else if let Some(sp) = self.current_spinner.take() {
            sp.finish_failure(detail);
        }
    }

    fn finish_resolving(&mut self, suffix: &str) {
        self.finish_active(&format!("resolved {}  {suffix}", self.agent()));
    }

    fn finish_compiling(&mut self, suffix: &str) {
        self.finish_active(&format!("compiled {}  {suffix}", self.agent()));
    }
}

impl<U: Ui> StreamRenderer<RunEvent> for RunStreamRenderer<U> {
    fn on_event(&mut self, event: &RunEvent) {
        match event {
            RunEvent::RunStarted { agent_name } => {
                self.agent_name = Some(agent_name.clone());
                self.current_spinner = Some(
                    self.ui
                        .spinner(&format!("resolving {agent_name}")),
                );
            }
            RunEvent::TransformingAssets { count } => {
                let label = format!("resolving {}  transforming {count} assets", self.agent());

                if self.verbose {
                    if let Some(sp) = self.current_spinner.take() {
                        sp.clear();
                    }
                    self.current_viewport = Some(self.ui.log_viewport(&label, 6));
                } else {
                    self.update_active(&label);
                }
            }
            RunEvent::TransformerStdout(line) => {
                if self.verbose
                    && let Some(vp) = &self.current_viewport
                {
                    vp.push(line);
                }
            }
            RunEvent::TransformerStderr(line) => {
                self.transformer_stderr.push_str(line);
                self.transformer_stderr.push('\n');

                if self.verbose
                    && let Some(vp) = &self.current_viewport
                {
                    vp.push(line);
                }
            }
            RunEvent::ManifestResolved { tool_count, .. } => {
                self.tool_count = Some(*tool_count);
                self.update_active(&format!("resolving {}  {tool_count} tools", self.agent()));
            }
            RunEvent::GenerationComplete { file_count } => {
                let tools = self.tool_count.unwrap_or(0);
                self.finish_resolving(&format!("{tools} tools, {file_count} files"));
            }
            RunEvent::Compiling { release } => {
                let mode = if *release { "release" } else { "debug" };
                let label = format!("compiling {}  {mode}", self.agent());

                if self.verbose {
                    self.current_viewport = Some(self.ui.log_viewport(&label, 6));
                } else {
                    self.current_spinner = Some(self.ui.spinner(&label));
                }
            }
            RunEvent::CompilerStdout(line) => {
                if let Some(vp) = &self.current_viewport {
                    vp.push(line);
                }
            }
            RunEvent::CompilerStderr(line) => {
                self.compiler_stderr.push_str(line);
                self.compiler_stderr.push('\n');

                if let Some(vp) = &self.current_viewport {
                    vp.push(line);
                }
            }
            RunEvent::CleanupRemoveFailed { path, error } => {
                self.ui
                    .warning(&format!("failed to remove {}: {}", path.display(), error));
            }
            RunEvent::Compiled { output_dir } => {
                self.finish_compiling(&format!("{} → {}", self.elapsed(), output_dir.display()));
            }
            // The invocation owns the terminal from here on, so every indicator is
            // retired and nothing further is printed until `on_failure`.
            RunEvent::Launching => {
                self.ui
                    .success(&format!("running {}", self.agent()));
                self.ui.println("");
            }
            _ => {}
        }
    }

    fn on_success(&mut self) {}

    fn on_failure(&mut self, error: &str) {
        self.fail_active("failed");

        self.ui.println("");
        self.ui
            .failure(&format!("failed in {}  {error}", self.elapsed()));

        if !self.compiler_stderr.is_empty() {
            self.ui.println("");
            for line in self.compiler_stderr.lines() {
                self.ui.detail(line);
            }
        } else if !self.transformer_stderr.is_empty() {
            self.ui.println("");
            for line in self.transformer_stderr.lines() {
                self.ui.detail(line);
            }
        }
    }
}
