// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    slice::Iter,
    vec::IntoIter,
};

use bstr::BString;
use typed_path::{Component as _, UnixComponents};

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Component {
    inner: BString,
}

impl Component {
    pub(crate) fn new(bytes: impl Into<BString>) -> Self {
        Component { inner: bytes.into() }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.inner
    }

    pub fn to_string_lossy(&self) -> String {
        String::from_utf8_lossy(&self.inner).into_owned()
    }
}

impl Display for Component {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.to_string_lossy())
    }
}

pub struct ComponentsIter<'a> {
    inner: UnixComponents<'a>,
}

impl<'a> ComponentsIter<'a> {
    pub(crate) fn new(inner: UnixComponents<'a>) -> Self {
        ComponentsIter { inner }
    }
}

impl Iterator for ComponentsIter<'_> {
    type Item = Component;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|component| Component::new(component.as_bytes()))
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Components {
    inner: Vec<Component>,
}

impl Components {
    pub fn new(components: impl Into<Vec<Component>>) -> Self {
        Components { inner: components.into() }
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn iter(&self) -> Iter<'_, Component> {
        self.inner.iter()
    }

    pub fn as_slice(&self) -> &[Component] {
        &self.inner
    }
}

impl AsRef<[Component]> for Components {
    fn as_ref(&self) -> &[Component] {
        self.as_slice()
    }
}

impl<'a> IntoIterator for &'a Components {
    type Item = &'a Component;
    type IntoIter = Iter<'a, Component>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl IntoIterator for Components {
    type Item = Component;
    type IntoIter = IntoIter<Component>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl FromIterator<Component> for Components {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = Component>,
    {
        Components::new(iter.into_iter().collect::<Vec<_>>())
    }
}

#[cfg(test)]
mod tests {
    use crate::path::{Components, PathBuf};

    #[test]
    fn lazy_components_collect_into_allocated_components() {
        assert_eq!(
            PathBuf::parse("/workspace/file.txt")
                .unwrap()
                .components()
                .collect::<Components>()
                .iter()
                .map(|component| component.to_string_lossy())
                .collect::<Vec<_>>(),
            vec!["/", "workspace", "file.txt"]
        );
    }

    #[test]
    fn allocated_components_support_borrowed_and_owned_iteration() {
        let components = PathBuf::parse("workspace/file.txt")
            .unwrap()
            .components()
            .collect::<Components>();

        assert_eq!(
            (&components)
                .into_iter()
                .map(|component| component.to_string_lossy())
                .collect::<Vec<_>>(),
            vec!["workspace", "file.txt"]
        );
        assert_eq!(
            components
                .into_iter()
                .map(|component| component.to_string_lossy())
                .collect::<Vec<_>>(),
            vec!["workspace", "file.txt"]
        );
    }
}
