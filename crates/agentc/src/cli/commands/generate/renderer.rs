// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::time::Instant;

use agentc_core::generate::types::GenerateEvent;

use crate::cli::ui::traits::{LogViewport, Spinner, StreamRenderer, Ui};

pub struct GenerateStreamRenderer<U: Ui> {
    ui: U,
    verbose: bool,
    start: Instant,
    agent_name: Option<String>,
    tool_count: Option<usize>,
    current_spinner: Option<Box<dyn Spinner>>,
    current_viewport: Option<Box<dyn LogViewport>>,
    transformer_output: String,
}

impl<U: Ui> GenerateStreamRenderer<U> {
    pub fn new(ui: U, verbose: bool) -> Self {
        Self {
            ui,
            verbose,
            start: Instant::now(),
            agent_name: None,
            tool_count: None,
            current_spinner: None,
            current_viewport: None,
            transformer_output: String::new(),
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

    fn finish_generating(&mut self, suffix: &str) {
        self.finish_active(&format!("generated {}  {suffix}", self.agent()));
    }
}

impl<U: Ui> StreamRenderer<GenerateEvent> for GenerateStreamRenderer<U> {
    fn on_event(&mut self, event: &GenerateEvent) {
        match event {
            GenerateEvent::GenerateStarted { agent_name } => {
                self.agent_name = Some(agent_name.clone());
                self.current_spinner = Some(
                    self.ui
                        .spinner(&format!("resolving {agent_name}")),
                );
            }
            GenerateEvent::TransformingAssets { count } => {
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
            GenerateEvent::TransformerStdout(line) => {
                if self.verbose
                    && let Some(vp) = &self.current_viewport
                {
                    vp.push(line);
                }
            }
            GenerateEvent::TransformerStderr(line) => {
                self.transformer_output.push_str(line);
                self.transformer_output.push('\n');

                if self.verbose
                    && let Some(vp) = &self.current_viewport
                {
                    vp.push(line);
                }
            }
            GenerateEvent::ManifestResolved { tool_count, .. } => {
                self.tool_count = Some(*tool_count);
                self.update_active(&format!("resolving {}  {tool_count} tools", self.agent()));
            }
            GenerateEvent::Success { vfs } => {
                let tools = self.tool_count.unwrap_or(0);
                self.finish_generating(&format!(
                    "{tools} tools, {} files  {}",
                    vfs.len(),
                    self.elapsed()
                ));
            }
            GenerateEvent::CleanupRemoveFailed { path, error } => {
                self.ui
                    .warning(&format!("failed to remove {}: {}", path.display(), error));
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

        if !self.transformer_output.is_empty() {
            self.ui.println("");
            for line in self.transformer_output.lines() {
                self.ui.detail(line);
            }
        }
    }
}
