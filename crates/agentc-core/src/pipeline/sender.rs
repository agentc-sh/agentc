// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use std::marker::PhantomData;

#[async_trait]
pub trait Tx {
    type Item: Send;
    type Error: Send;

    async fn send(&self, item: Self::Item) -> Result<(), Self::Error>;
}

#[async_trait]
impl<T> Tx for tokio::sync::mpsc::Sender<T>
where
    T: Send,
{
    type Item = T;
    type Error = tokio::sync::mpsc::error::SendError<T>;

    async fn send(&self, item: Self::Item) -> Result<(), Self::Error> {
        self.send(item).await
    }
}

pub struct MapTx<Inner, F, T, R, E> {
    inner: Inner,
    f: F,
    _marker: PhantomData<(T, R, E)>,
}

#[async_trait]
impl<Inner, F, T, R, E> Tx for MapTx<Inner, F, T, R, E>
where
    Inner: Tx<Item = R, Error = E> + Send + Sync,
    F: Fn(T) -> R + Send + Sync,
    T: Send + Sync,
    R: Send + Sync,
    E: Send + Sync,
{
    type Item = T;
    type Error = E;

    async fn send(&self, item: Self::Item) -> Result<(), Self::Error> {
        self.inner.send((self.f)(item)).await
    }
}

pub trait MappableTx: Tx + Sized {
    fn map<F, T2>(self, f: F) -> MapTx<Self, F, T2, Self::Item, Self::Error>
    where
        F: Fn(T2) -> Self::Item + Send + Sync,
        T2: Send + Sync,
        Self::Item: Send + Sync,
        Self::Error: Send + Sync,
    {
        MapTx { inner: self, f, _marker: PhantomData }
    }
}

#[async_trait]
impl<T: Tx> MappableTx for T where T: Sized + Send {}

impl<Inner, F, T, R, E> Clone for MapTx<Inner, F, T, R, E>
where
    Inner: Clone,
    F: Clone,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            f: self.f.clone(),
            _marker: PhantomData,
        }
    }
}
