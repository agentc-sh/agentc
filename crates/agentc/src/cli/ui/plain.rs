// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use crate::cli::ui::{
    traits::{LogViewport, Spinner, Ui},
    types::Style,
};

/// A [`Spinner`](crate::cli::ui::traits::Spinner) implementation that simply prints messages to the console.
pub struct PlainSpinner;

impl PlainSpinner {
    pub fn new(text: &str) -> Self {
        println!("{text}");
        Self
    }
}

impl Spinner for PlainSpinner {
    fn update(&self, text: &str) {
        println!("{text}");
    }

    fn clear(&self) {}

    fn finish(&self, text: &str) {
        println!("ok: {text}");
    }

    fn finish_failure(&self, text: &str) {
        println!("error: {text}");
    }
}

/// A [`LogViewport`](crate::cli::ui::traits::LogViewport) implementation that simply prints messages to the console.
pub struct PlainLogViewport;

impl PlainLogViewport {
    pub fn new(header: &str) -> Self {
        println!("{header}");
        Self
    }
}

impl LogViewport for PlainLogViewport {
    fn update_header(&self, text: &str) {
        println!("{text}");
    }

    fn clear(&self) {}

    fn push(&self, line: &str) {
        println!("{line}");
    }

    fn finish(&self, message: &str) {
        println!("ok: {message}");
    }

    fn finish_failure(&self, message: &str) {
        println!("error: {message}");
    }
}

/// A [`Ui`](crate::cli::ui::traits::Ui) implementation for non-TTY environments, with no color or styling.
pub struct PlainUi;

impl Ui for PlainUi {
    fn print(&self, text: &str) {
        print!("{text}");
    }

    fn println(&self, text: &str) {
        println!("{text}");
    }

    fn print_styled(&self, text: &str, _style: Style) {
        print!("{text}");
    }

    fn println_styled(&self, text: &str, _style: Style) {
        println!("{text}");
    }

    fn spinner(&self, text: &str) -> Box<dyn Spinner> {
        Box::new(PlainSpinner::new(text))
    }

    fn log_viewport(&self, header: &str, _height: usize) -> Box<dyn LogViewport> {
        Box::new(PlainLogViewport::new(header))
    }

    fn section(&self, text: &str) {
        println!("{text}");
    }

    fn tree_item(&self, text: &str, _last: bool) {
        println!("  {text}");
    }

    fn success(&self, text: &str) {
        println!("ok: {text}");
    }

    fn failure(&self, text: &str) {
        println!("error: {text}");
    }

    fn warning(&self, text: &str) {
        println!("warning: {text}");
    }

    fn detail(&self, text: &str) {
        println!("  {text}");
    }
}
