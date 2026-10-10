// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

pub mod block;
pub mod evaluator;
pub mod manifest;
pub mod renderer;

pub use block::TemplateBlock;
pub use manifest::{ExtensionPointSpec, FileSpec, Reducer, SlotFillSpec, TemplateBlockManifest};
