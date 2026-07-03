// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use crate::cli::ui::types::{Color, Style};

/// A handle to an active spinner or progress indicator.
pub trait Spinner: Send + Sync {
    /// Update the spinner message.
    fn update(&self, text: &str);

    /// Finish the spinner and clear it.
    fn clear(&self);

    /// Finish the spinner with a success message.
    fn finish(&self, text: &str);

    /// Finish the spinner with a failure message.
    fn finish_failure(&self, text: &str);
}

/// A trait for rendering a log viewport that can be updated in place.
pub trait LogViewport: Send + Sync {
    /// Update the header of the log viewport.
    fn update_header(&self, text: &str);

    /// Push a new line of log output to the viewport.
    fn push(&self, line: &str);

    /// Finish the log viewport and clear it.
    fn clear(&self);

    /// Finish the log viewport with a success message.
    fn finish(&self, message: &str);

    /// Finish the log viewport with a failure message.
    fn finish_failure(&self, message: &str);
}

/// A trait for rendering output to the terminal.
pub trait Ui: Send + Sync {
    /// Print text without a newline.
    fn print(&self, text: &str);

    /// Print text with a newline.
    fn println(&self, text: &str);

    /// Print styled text without a newline.
    fn print_styled(&self, text: &str, style: Style);

    /// Print styled text with a newline.
    fn println_styled(&self, text: &str, style: Style);

    /// Create a new spinner with the given initial message.
    fn spinner(&self, text: &str) -> Box<dyn Spinner>;

    /// Create a new log viewport with the given initial header.
    fn log_viewport(&self, header: &str, height: usize) -> Box<dyn LogViewport>;

    /// Print a section header.
    fn section(&self, text: &str) {
        self.println_styled(text, Style::default().bold());
    }

    /// Print a tree item, optionally marking it as the last item.
    fn tree_item(&self, text: &str, last: bool) {
        let prefix = if last { " └─ " } else { " ├─ " };
        self.println(&format!("{prefix}{text}"));
    }

    /// Print a success message.
    fn success(&self, text: &str) {
        self.println_styled(
            &format!(" ✓ {text}"),
            Style::default()
                .color(Color::Green)
                .bold(),
        );
    }

    /// Print a failure message.
    fn failure(&self, text: &str) {
        self.println_styled(
            &format!(" ✗ {text}"),
            Style::default()
                .color(Color::Red)
                .bold(),
        );
    }

    /// Print a warning message.
    fn warning(&self, text: &str) {
        self.println_styled(&format!(" ⚠ {text}"), Style::default().color(Color::Yellow));
    }

    /// Print an informational message.
    fn info(&self, text: &str) {
        self.println(text);
    }

    /// Print a detail message, dimmed.
    fn detail(&self, text: &str) {
        self.println_styled(&format!("   {text}"), Style::default().dim());
    }
}

impl Ui for Box<dyn Ui> {
    fn print(&self, text: &str) {
        (**self).print(text);
    }

    fn println(&self, text: &str) {
        (**self).println(text);
    }

    fn print_styled(&self, text: &str, style: Style) {
        (**self).print_styled(text, style);
    }

    fn println_styled(&self, text: &str, style: Style) {
        (**self).println_styled(text, style);
    }

    fn spinner(&self, text: &str) -> Box<dyn Spinner> {
        (**self).spinner(text)
    }

    fn log_viewport(&self, header: &str, height: usize) -> Box<dyn LogViewport> {
        (**self).log_viewport(header, height)
    }
}

/// A trait for rendering a stream of events.
pub trait StreamRenderer<E> {
    /// Handle a single event from the stream.
    fn on_event(&mut self, event: &E);

    /// Called when the stream completes successfully.
    fn on_success(&mut self);

    /// Called when the stream completes with a failure.
    fn on_failure(&mut self, error: &str);
}
