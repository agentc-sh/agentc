// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::time::Duration;

/// A style to apply to a piece of text when rendering.
#[derive(Debug, Clone, Default)]
pub struct Style {
    pub color: Option<Color>,
    pub bold: bool,
    pub dim: bool,
}

impl Style {
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    pub fn dim(mut self) -> Self {
        self.dim = true;
        self
    }
}

/// A color to apply to a piece of text when rendering.
#[derive(Debug, Clone)]
pub enum Color {
    Default,
    Green,
    Yellow,
    Red,
    Cyan,
    White,
    Blue,
    Magenta,
}
