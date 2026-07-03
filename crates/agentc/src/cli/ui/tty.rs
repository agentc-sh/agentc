// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use console::Term;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::{sync::Mutex, time::Duration};

use crate::cli::ui::{
    traits::{LogViewport, Spinner, Ui},
    types::{Color, Style},
};

/// A [`Spinner`](crate::cli::ui::traits::Spinner) implementation using `indicatif` spinners.
pub struct TtySpinner {
    bar: ProgressBar,
}

impl TtySpinner {
    pub fn new(text: &str) -> Self {
        let bar = ProgressBar::new_spinner();

        bar.set_style(
            ProgressStyle::default_spinner()
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );

        bar.set_message(text.to_string());
        bar.enable_steady_tick(Duration::from_millis(80));

        Self { bar }
    }
}

impl Spinner for TtySpinner {
    fn update(&self, text: &str) {
        self.bar.set_message(text.to_string());
    }

    fn clear(&self) {
        self.bar.finish_and_clear();
    }

    fn finish(&self, text: &str) {
        self.bar.finish_and_clear();
        println!(
            "{}",
            console::style(format!(" ✓ {text}"))
                .green()
                .bold()
        );
    }

    fn finish_failure(&self, text: &str) {
        self.bar.finish_and_clear();
        println!(
            "{}",
            console::style(format!(" ✗ {text}"))
                .red()
                .bold()
        );
    }
}

/// A [`LogViewport`](crate::cli::ui::traits::LogViewport) implementation using `indicatif` progress bars to render a scrollable log viewport.
pub struct TtyLogViewport {
    multi: MultiProgress,
    spinner: ProgressBar,
    log: ProgressBar,
    height: usize,
    lines: Mutex<Vec<String>>,
}

impl TtyLogViewport {
    pub fn new(header: &str, height: usize) -> Self {
        let multi = MultiProgress::new();

        let spinner = multi.add(ProgressBar::new_spinner());
        spinner.set_style(
            ProgressStyle::default_spinner()
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        spinner.set_message(header.to_string());
        spinner.enable_steady_tick(Duration::from_millis(80));

        let log = multi.add(ProgressBar::new_spinner());
        log.set_style(
            ProgressStyle::default_spinner()
                .template("{msg}")
                .unwrap(),
        );
        log.set_message("");

        Self {
            multi,
            spinner,
            log,
            height,
            lines: Mutex::new(Vec::new()),
        }
    }

    fn render_log(&self) {
        let lines = self.lines.lock().unwrap();
        let visible = lines
            .iter()
            .rev()
            .take(self.height)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>();
        let rendered = visible
            .iter()
            .map(|l| format!("  {}", console::style(l).dim()))
            .collect::<Vec<_>>()
            .join("\n");

        self.log.set_message(rendered);
    }
}

impl LogViewport for TtyLogViewport {
    fn update_header(&self, text: &str) {
        self.spinner
            .set_message(text.to_string());
    }

    fn push(&self, line: &str) {
        self.lines
            .lock()
            .unwrap()
            .push(line.to_string());
        self.render_log();
    }

    fn clear(&self) {
        self.lines.lock().unwrap().clear();
        self.log.finish_and_clear();
        self.spinner.finish_and_clear();
    }

    fn finish(&self, message: &str) {
        self.log.finish_and_clear();
        self.spinner.finish_and_clear();
        println!(
            "{}",
            console::style(format!(" ✓ {message}"))
                .green()
                .bold()
        );
    }

    fn finish_failure(&self, message: &str) {
        self.log.finish_and_clear();
        self.spinner.finish_and_clear();
        println!(
            "{}",
            console::style(format!(" ✗ {message}"))
                .red()
                .bold()
        );
    }
}

/// A [`Ui`](crate::cli::ui::traits::Ui) implementation for TTY terminals, with color and styling support.
pub struct TtyUi {
    term: Term,
}

impl TtyUi {
    pub fn stdout() -> Self {
        Self { term: Term::stdout() }
    }

    pub fn stderr() -> Self {
        Self { term: Term::stderr() }
    }

    fn apply_style(&self, text: &str, style: Style) -> String {
        let mut styled = console::style(text);

        if style.bold {
            styled = styled.bold();
        }
        if style.dim {
            styled = styled.dim();
        }

        styled = match style.color {
            Some(Color::Default) => styled,
            Some(Color::Red) => styled.red(),
            Some(Color::Green) => styled.green(),
            Some(Color::Yellow) => styled.yellow(),
            Some(Color::Blue) => styled.blue(),
            Some(Color::Magenta) => styled.magenta(),
            Some(Color::Cyan) => styled.cyan(),
            Some(Color::White) => styled.white(),
            None => styled,
        };

        styled.to_string()
    }
}

impl Ui for TtyUi {
    fn print(&self, text: &str) {
        let _ = self.term.write_str(text);
    }

    fn println(&self, text: &str) {
        let _ = self.term.write_line(text);
    }

    fn print_styled(&self, text: &str, style: Style) {
        let _ = self
            .term
            .write_str(&self.apply_style(text, style));
    }

    fn println_styled(&self, text: &str, style: Style) {
        let _ = self
            .term
            .write_line(&self.apply_style(text, style));
    }

    fn spinner(&self, text: &str) -> Box<dyn Spinner> {
        Box::new(TtySpinner::new(text))
    }

    fn log_viewport(&self, header: &str, height: usize) -> Box<dyn LogViewport> {
        Box::new(TtyLogViewport::new(header, height))
    }
}
