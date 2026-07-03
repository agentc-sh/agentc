// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};
use std::{
    slice::{Iter, IterMut},
    vec::IntoIter,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub count: u64,
    pub next_page: Option<String>,
}

impl<T> Default for Page<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            count: 0,
            next_page: None,
        }
    }
}

impl<T> Page<T> {
    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn as_slice(&self) -> &[T] {
        &self.items
    }

    pub fn iter(&self) -> Iter<'_, T> {
        self.items.iter()
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        self.items.iter_mut()
    }

    pub fn map<U, F>(self, f: F) -> Page<U>
    where
        F: FnMut(T) -> U,
    {
        Page {
            items: self.items.into_iter().map(f).collect(),
            count: self.count,
            next_page: self.next_page,
        }
    }

    pub fn try_map<U, E, F>(self, f: F) -> Result<Page<U>, E>
    where
        F: FnMut(T) -> Result<U, E>,
    {
        Ok(Page {
            items: self
                .items
                .into_iter()
                .map(f)
                .collect::<Result<Vec<U>, E>>()?,
            count: self.count,
            next_page: self.next_page,
        })
    }

    pub async fn async_map<U, F, Fut>(self, mut f: F) -> Page<U>
    where
        F: FnMut(T) -> Fut,
        Fut: Future<Output = U>,
    {
        let mut items = Vec::with_capacity(self.items.len());

        for item in self.items {
            let mapped = f(item).await;
            items.push(mapped);
        }

        Page {
            items,
            count: self.count,
            next_page: self.next_page,
        }
    }

    pub async fn async_try_map<U, E, F, Fut>(self, mut f: F) -> Result<Page<U>, E>
    where
        F: FnMut(T) -> Fut,
        Fut: Future<Output = Result<U, E>>,
    {
        let mut items = Vec::with_capacity(self.items.len());

        for item in self.items {
            let mapped = f(item).await?;
            items.push(mapped);
        }

        Ok(Page {
            items,
            count: self.count,
            next_page: self.next_page,
        })
    }
}

impl<T> IntoIterator for Page<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

impl<'a, T> IntoIterator for &'a Page<T> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut Page<T> {
    type Item = &'a mut T;
    type IntoIter = IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.iter_mut()
    }
}
