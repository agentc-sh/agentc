// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

pub mod block;
pub mod contribution;
pub mod renderer;
pub mod traits;

pub use block::{TemplateBlock, TemplateBlockBuilder};
pub use traits::RenderedContribution;
