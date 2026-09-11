// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_executor_python::guestpy::{
    FromGuest, backend::Backend, errors::Error, handle::Object, host_class,
};
use json_patch::Patch;
use serde::Deserialize;

#[derive(Deserialize, FromGuest)]
#[guestpy(crate_path = agentc_executor_python::guestpy)]
#[serde(transparent)]
struct StateUpdate(Patch);

impl StateUpdate {
    fn into_inner(self) -> Patch {
        self.0
    }
}

pub struct ToolOutput<B: Backend> {
    result: Object<B>,
    state_update: Option<Patch>,
}

#[host_class(backend = B, generic, crate_path = agentc_executor_python::guestpy)]
impl<B: Backend> ToolOutput<B> {
    #[guestpy(constructor)]
    fn new(
        result: Object<B>,
        #[guestpy(kw)] state_update: Option<Option<StateUpdate>>,
    ) -> Result<Self, Error> {
        Ok(Self {
            result,
            state_update: state_update.flatten().map(StateUpdate::into_inner),
        })
    }

    pub(crate) fn result(&self) -> &Object<B> {
        &self.result
    }

    pub(crate) fn state_update(&self) -> Option<&Patch> {
        self.state_update.as_ref()
    }
}
