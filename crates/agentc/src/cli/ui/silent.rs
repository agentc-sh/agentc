// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use crate::cli::ui::{
    traits::{LogViewport, Spinner, Ui},
    types::Style,
};

/// A [`Spinner`](crate::cli::ui::traits::Spinner) implementation that produces no output, useful for testing.
pub struct SilentSpinner;

impl Spinner for SilentSpinner {
    fn update(&self, _text: &str) {}
    fn clear(&self) {}
    fn finish(&self, _text: &str) {}
    fn finish_failure(&self, _text: &str) {}
}

/// A [`LogViewport`](crate::cli::ui::traits::LogViewport) implementation that produces no output, useful for testing.
pub struct SilentLogViewport;

impl LogViewport for SilentLogViewport {
    fn update_header(&self, _text: &str) {}
    fn push(&self, _line: &str) {}
    fn clear(&self) {}
    fn finish(&self, _message: &str) {}
    fn finish_failure(&self, _message: &str) {}
}

/// A [`Ui`](crate::cli::ui::traits::Ui) implementation that produces no output, useful for testing.
pub struct SilentUi;

impl Ui for SilentUi {
    fn print(&self, _text: &str) {}
    fn println(&self, _text: &str) {}
    fn print_styled(&self, _text: &str, _style: Style) {}
    fn println_styled(&self, _text: &str, _style: Style) {}

    fn spinner(&self, _text: &str) -> Box<dyn Spinner> {
        Box::new(SilentSpinner)
    }

    fn log_viewport(&self, _header: &str, _height: usize) -> Box<dyn LogViewport> {
        Box::new(SilentLogViewport)
    }
}
