// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::errors::BlocksError;

pub trait Mergeable: Sized {
    type Key: Ord;

    fn key(&self) -> Self::Key;

    fn merge(&mut self, other: Self) -> Result<(), BlocksError>;
}

pub struct ContributionSet<T: Mergeable> {
    entries: BTreeMap<T::Key, T>,
}

impl<T: Mergeable> ContributionSet<T> {
    pub fn new() -> Self {
        Self { entries: BTreeMap::new() }
    }

    pub fn from_entries(entries: impl IntoIterator<Item = T>) -> Result<Self, BlocksError> {
        entries
            .into_iter()
            .try_fold(Self::new(), |set, entry| set.with(entry))
    }

    pub fn merge_all(sets: impl IntoIterator<Item = Self>) -> Result<Self, BlocksError> {
        sets.into_iter()
            .try_fold(Self::new(), |mut accumulated, set| {
                accumulated.merge(set)?;

                Ok(accumulated)
            })
    }

    pub fn with(mut self, entry: T) -> Result<Self, BlocksError> {
        self.insert(entry)?;

        Ok(self)
    }

    pub fn insert(&mut self, entry: T) -> Result<(), BlocksError> {
        match self.entries.get_mut(&entry.key()) {
            Some(existing) => existing.merge(entry),
            None => {
                self.entries.insert(entry.key(), entry);

                Ok(())
            }
        }
    }

    pub fn merge(&mut self, other: Self) -> Result<(), BlocksError> {
        for entry in other.entries.into_values() {
            self.insert(entry)?;
        }

        Ok(())
    }

    pub fn get(&self, key: &T::Key) -> Option<&T> {
        self.entries.get(key)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn into_values(self) -> impl Iterator<Item = T> {
        self.entries.into_values()
    }
}

impl<T: Mergeable> Default for ContributionSet<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, Eq)]
    struct Entry {
        name: &'static str,
        values: Vec<&'static str>,
    }

    impl Entry {
        fn new(name: &'static str, value: &'static str) -> Self {
            Self { name, values: vec![value] }
        }
    }

    impl Mergeable for Entry {
        type Key = &'static str;

        fn key(&self) -> Self::Key {
            self.name
        }

        fn merge(&mut self, other: Self) -> Result<(), BlocksError> {
            if other.values.contains(&"conflict") {
                return Err(BlocksError::invalid("conflicting entry"));
            }

            self.values.extend(other.values);

            Ok(())
        }
    }

    #[test]
    fn entries_with_the_same_key_merge() {
        let set =
            ContributionSet::from_entries([Entry::new("a", "one"), Entry::new("a", "two")]).unwrap();

        assert_eq!(set.len(), 1);
        assert_eq!(set.get(&"a").unwrap().values, vec!["one", "two"]);
    }

    #[test]
    fn one_set_of_two_equals_two_sets_of_one() {
        let single =
            ContributionSet::from_entries([Entry::new("a", "x"), Entry::new("b", "y")]).unwrap();
        let merged = ContributionSet::merge_all([
            ContributionSet::from_entries([Entry::new("a", "x")]).unwrap(),
            ContributionSet::from_entries([Entry::new("b", "y")]).unwrap(),
        ])
        .unwrap();

        assert_eq!(
            single
                .into_values()
                .collect::<Vec<_>>(),
            merged
                .into_values()
                .collect::<Vec<_>>(),
        );
    }

    #[test]
    fn entries_are_ordered_by_key() {
        assert_eq!(
            ContributionSet::from_entries([Entry::new("zzz", "z"), Entry::new("aaa", "a")])
                .unwrap()
                .into_values()
                .map(|entry| entry.name)
                .collect::<Vec<_>>(),
            vec!["aaa", "zzz"],
        );
    }

    #[test]
    fn a_conflicting_entry_fails_the_set() {
        assert!(
            ContributionSet::from_entries([Entry::new("a", "one"), Entry::new("a", "conflict")])
                .is_err()
        );
    }
}
