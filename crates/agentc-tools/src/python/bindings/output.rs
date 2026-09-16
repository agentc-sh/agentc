// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_executor_python::guestpy::{
    FromGuest,
    host_class,
    backend::{Backend, BackendValues},
    errors::Error,
    handle::Object,
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

pub struct ToolOutput<B: Backend + BackendValues> {
    output: Object<B>,
    state_update: Option<Patch>,
}

#[host_class(backend = B, generic, crate_path = agentc_executor_python::guestpy)]
impl<B: Backend + BackendValues> ToolOutput<B> {
    #[guestpy(constructor)]
    fn new(
        output: Object<B>,
        #[guestpy(kw)] state_update: Option<Option<StateUpdate>>,
    ) -> Result<Self, Error> {
        Ok(Self {
            output,
            state_update: state_update
                .flatten()
                .map(StateUpdate::into_inner),
        })
    }

    pub(crate) fn output(&self) -> &Object<B> {
        &self.output
    }

    pub(crate) fn state_update(&self) -> Option<&Patch> {
        self.state_update.as_ref()
    }
}
