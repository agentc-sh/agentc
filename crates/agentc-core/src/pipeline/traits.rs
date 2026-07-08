// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use futures::future::BoxFuture;

use crate::pipeline::sender::{MappableTx, Tx};

#[async_trait]
pub trait Step: Send + Sync {
    type Input: Send;
    type Output: Send;
    type Event: Send;
    type Error: Send;

    async fn execute<S>(self, input: Self::Input, tx: S) -> Result<Self::Output, Self::Error>
    where
        S: Tx<Item = Self::Event> + Clone + Send + Sync + 'static;
}

#[async_trait]
pub trait ErasedStep<I, O, E, Err>: Send + Sync {
    async fn execute_erased<T>(self, input: I, tx: T) -> Result<O, Err>
    where
        T: Tx<Item = E> + Clone + Send + Sync + 'static,
        T::Error: Into<Err> + Send + Sync;
}

#[derive(Clone)]
struct StepWrapper<S>(S);

#[async_trait]
impl<S, I, O, E, Err> ErasedStep<I, O, E, Err> for StepWrapper<S>
where
    S: Step<Input = I, Output = O> + Send + Sync,
    S::Event: Into<E> + Send + Sync + 'static,
    S::Error: Into<Err> + Send + Sync,
    I: Send + 'static,
    O: Send + 'static,
    E: Send + Sync + 'static,
    Err: Send + Sync + 'static,
{
    async fn execute_erased<T>(self, input: I, tx: T) -> Result<O, Err>
    where
        T: Tx<Item = E> + Clone + Send + Sync + 'static,
        T::Error: Into<Err> + Send + Sync,
    {
        self.0
            .execute(
                input,
                tx.clone()
                    .map(|event: S::Event| event.into()),
            )
            .await
            .map_err(|err| err.into())
    }
}

type PipelineRunner<I, O, T, Err> =
    Box<dyn FnOnce(I, T) -> BoxFuture<'static, Result<O, Err>> + Send>;

pub struct Pipeline<I, O, T, E, Err>
where
    T: Tx<Item = E> + Clone + Send + Sync,
    T::Error: Into<Err> + Send + Sync,
    I: Send,
    O: Send,
    E: Send,
    Err: Send,
{
    runner: PipelineRunner<I, O, T, Err>,
}

impl<I, T, E, Err> Default for Pipeline<I, I, T, E, Err>
where
    T: Tx<Item = E> + Clone + Send + Sync,
    T::Error: Into<Err> + Send + Sync,
    I: Send + 'static,
    E: Send + 'static,
    Err: Send + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<I, T, E, Err> Pipeline<I, I, T, E, Err>
where
    T: Tx<Item = E> + Clone + Send + Sync,
    T::Error: Into<Err> + Send + Sync,
    I: Send + 'static,
    E: Send + 'static,
    Err: Send + 'static,
{
    pub fn new() -> Self {
        Self {
            runner: Box::new(|input, _| Box::pin(async move { Ok(input) })),
        }
    }
}

impl<I, O, T, E, Err> Pipeline<I, O, T, E, Err>
where
    T: Tx<Item = E> + Clone + Send + Sync + 'static,
    T::Error: Into<Err> + Send + Sync + 'static,
    I: Send + 'static,
    O: Send + 'static,
    E: Send + 'static,
    Err: Send + 'static,
{
    pub fn step<S>(self, step: S) -> Pipeline<I, S::Output, T, E, Err>
    where
        S: Step + Send + Sync + 'static,
        O: Into<S::Input>,
        S::Output: Send + 'static,
        S::Event: Into<E> + Send + Sync + 'static,
        S::Error: Into<Err> + Send + Sync + 'static,
        E: Sync,
        Err: Sync,
    {
        let prev_runner = self.runner;
        let step_wrapper = StepWrapper(step);

        Pipeline {
            runner: Box::new(move |input, tx| {
                Box::pin(async move {
                    step_wrapper
                        .execute_erased(
                            prev_runner(input, tx.clone())
                                .await?
                                .into(),
                            tx,
                        )
                        .await
                })
            }),
        }
    }

    pub async fn run(self, input: I, tx: T) -> Result<O, Err> {
        (self.runner)(input, tx).await
    }
}
