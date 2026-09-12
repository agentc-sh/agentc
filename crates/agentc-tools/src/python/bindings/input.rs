// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::sync::{Arc, Weak};

use agentc_agent::tools::activity::{ActivityDelta, ActivityEmitter};
use agentc_executor_python::guestpy::{
    FromGuest,
    backend::{Backend, BackendCallables, BackendValues},
    errors::Error,
    host::function::HostFn,
    host_class,
};
use json_patch::PatchOperation;
use serde::Deserialize;

use crate::python::bindings::coercion::Decoded;

#[derive(Deserialize, FromGuest)]
#[guestpy(crate_path = agentc_executor_python::guestpy)]
struct GuestActivityPatch(Vec<PatchOperation>);

pub(crate) type ActivityGuard = Arc<ActivityEmitter>;

pub(crate) type WeakActivityGuard = Weak<ActivityEmitter>;

pub struct ToolInput<B: Backend + BackendValues> {
    args: Decoded<B>,
    state: Decoded<B>,
    emitter: Option<WeakActivityGuard>,
}

#[host_class(backend = B, generic, crate_path = agentc_executor_python::guestpy)]
impl<B: Backend + BackendValues + BackendCallables> ToolInput<B> {
    pub(crate) fn new(
        args: Decoded<B>,
        state: Decoded<B>,
        emitter: Option<ActivityGuard>,
    ) -> (Self, Option<ActivityGuard>) {
        (
            Self {
                args,
                state,
                emitter: emitter.as_ref().map(Arc::downgrade),
            },
            emitter,
        )
    }

    #[guestpy(get)]
    fn args(&self) -> Result<Decoded<B>, Error> {
        Ok(self.args.clone())
    }

    #[guestpy(get)]
    fn state(&self) -> Result<Decoded<B>, Error> {
        Ok(self.state.clone())
    }

    #[guestpy(get)]
    fn emit(&self) -> Result<Option<HostFn<B>>, Error> {
        let Some(emitter) = self.emitter.clone() else {
            return Ok(None);
        };

        Ok(Some(HostFn::new(move |enter, args| {
            let delta = ActivityDelta {
                activity_type: args.required::<String>(enter, 0, "activity_type")?,
                patch: args
                    .required::<GuestActivityPatch>(enter, 1, "patch")?
                    .0,
            };

            args.finish()?;

            if let Some(sender) = emitter
                .upgrade()
                .and_then(|emitter| emitter.sender())
            {
                let _ = sender.try_send(delta);
            }

            Ok(())
        })))
    }
}
