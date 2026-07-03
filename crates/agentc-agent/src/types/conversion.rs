// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

/// Trait for converting agent types to their corresponding model types.
pub trait ToModelType {
    type ModelType;

    fn to_model_type(&self) -> Self::ModelType;
}

/// Trait for converting model types to their corresponding agent types.
pub trait FromModelType {
    type ModelType;
    type Output;

    fn from_model_type(model: Self::ModelType) -> Self::Output;
}
