// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    ops::Deref,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Segment {
    Key(String),
    Index(usize),
}

impl Segment {
    pub fn key(key: impl Into<String>) -> Self {
        Self::Key(key.into())
    }

    pub fn index(index: usize) -> Self {
        Self::Index(index)
    }
}

impl Display for Segment {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Segment::Key(key) => write!(f, "{}", key),
            Segment::Index(index) => write!(f, "[{}]", index),
        }
    }
}

impl From<String> for Segment {
    fn from(key: String) -> Self {
        Self::key(key)
    }
}

impl From<&str> for Segment {
    fn from(key: &str) -> Self {
        Self::key(key)
    }
}

impl From<usize> for Segment {
    fn from(index: usize) -> Self {
        Self::index(index)
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Path(pub Vec<Segment>);

impl Path {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn push(&mut self, segment: Segment) {
        self.0.push(segment);
    }

    pub fn child(&self, segment: Segment) -> Self {
        let mut child = self.clone();
        child.push(segment);
        child
    }

    pub fn as_inner(&self) -> &[Segment] {
        &self.0
    }

    pub fn as_inner_mut(&mut self) -> &mut Vec<Segment> {
        &mut self.0
    }

    pub fn into_inner(self) -> Vec<Segment> {
        self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Deref for Path {
    type Target = [Segment];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for Path {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(
            f,
            "{}",
            self.as_inner()
                .iter()
                .map(|segment| segment.to_string())
                .collect::<Vec<_>>()
                .join(".")
        )
    }
}

impl From<Vec<Segment>> for Path {
    fn from(segments: Vec<Segment>) -> Self {
        Self(segments)
    }
}

impl IntoIterator for Path {
    type Item = Segment;
    type IntoIter = std::vec::IntoIter<Segment>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a Path {
    type Item = &'a Segment;
    type IntoIter = std::slice::Iter<'a, Segment>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

#[macro_export]
macro_rules! path {
    ( $( $x:expr ),* $(,)? ) => {
        ::agentc_config::path::Path::from(vec![ $( ::agentc_config::path::Segment::from($x) ),* ])
    };
}
