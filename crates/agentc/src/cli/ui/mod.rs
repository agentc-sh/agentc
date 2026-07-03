// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

#![allow(unused)]

pub mod plain;
pub mod silent;
pub mod traits;
pub mod tty;
pub mod types;

pub use plain::PlainUi;
pub use silent::SilentUi;
pub use traits::{Spinner, StreamRenderer, Ui};
pub use tty::TtyUi;
pub use types::{Color, Style};

use clap::ValueEnum;
use std::io::IsTerminal;

/// Factory function to get the appropriate [`Ui`](crate::cli::ui::traits::Ui) implementation based on the environment.
pub fn default() -> Box<dyn Ui> {
    if std::io::stdout().is_terminal() {
        Box::new(TtyUi::stdout())
    } else {
        Box::new(PlainUi)
    }
}

#[derive(Debug, Clone, ValueEnum)]
pub enum UiFormat {
    /// Automatically detect based on TTY (default).
    Auto,
    /// Force TTY output
    Tty,
    /// Force plain output without ANSI codes.
    Plain,
    /// Silent output with no rendering.
    Silent,
}

impl UiFormat {
    pub fn ui(&self) -> Box<dyn Ui> {
        match self {
            UiFormat::Auto => default(),
            UiFormat::Tty => Box::new(TtyUi::stdout()),
            UiFormat::Plain => Box::new(PlainUi),
            UiFormat::Silent => Box::new(SilentUi),
        }
    }
}
