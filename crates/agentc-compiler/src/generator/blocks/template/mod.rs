// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

pub mod block;
pub mod evaluator;
pub mod manifest;
pub mod renderer;
pub mod traits;

pub use block::{TemplateBlock, TemplateFragmentBlock};
pub use manifest::{ExtensionPointSpec, FileSpec, Reducer, SlotFillSpec, TemplateBlockManifest};
pub use traits::TemplateFragment;
