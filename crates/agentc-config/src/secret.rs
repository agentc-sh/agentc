// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};
use std::{
    fmt::{Debug, Display, Formatter, Result as FmtResult},
    ops::{Deref, DerefMut},
};

#[derive(Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Secret<T>(pub T);

impl<T> Secret<T> {
    pub fn new(value: T) -> Self {
        Self(value)
    }

    pub fn as_inner(&self) -> &T {
        &self.0
    }

    pub fn as_inner_mut(&mut self) -> &mut T {
        &mut self.0
    }

    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> Debug for Secret<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "Secret(*****)")
    }
}

impl<T> Display for Secret<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "*****")
    }
}

impl<T> Default for Secret<T>
where
    T: Default,
{
    fn default() -> Self {
        Self(T::default())
    }
}

impl<T> PartialEq for Secret<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T> Eq for Secret<T> where T: Eq {}

impl<T> Deref for Secret<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Secret<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
