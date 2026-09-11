// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_executor_python::{
    backend::ExecutorBackend,
    guestpy::{
        errors::Error,
        guest_class,
        handle::{Class, Coroutine, Named, Object, ObjectProtocol, TypeProtocol},
        marshal::FromGuest,
        scope::Enter,
    },
    json::Json,
};
use serde_json::{Map, Value};

use crate::python::bindings::{input::ToolInput, output::ToolOutput, tool::Tool};

guest_class! {
    #[guestpy(crate_path = agentc_executor_python::guestpy, payload = Tool)]
    pub class GuestTool {
        fn execute(input: ToolInput<B>) -> Coroutine<B, ToolOutput<B>>;
    }
}

impl<B: ExecutorBackend> GuestTool<B> {
    pub(crate) fn args(&self, args: Map<String, Value>) -> Result<Object<B>, Error> {
        self.instance()
            .class("args")?
            .call_with::<_, _, Object<B>>(
                (),
                args.into_iter()
                    .map(|(name, value)| (name, Json(value)))
                    .collect::<Vec<_>>(),
            )
    }
}

pub struct GuestToolClass;

impl<B: ExecutorBackend> FromGuest<B> for GuestToolClass {
    type Owned = Class<B, GuestTool<B>>;

    fn from_guest<'py>(
        enter: &Enter<'py, B>,
        value: B::Value<'py>,
    ) -> Result<Self::Owned, Error> {
        let class = Class::from_guest(enter, value)?;
        let base = Class::of::<Tool>(enter)?;

        if class.is(&base) || !class.is_subclass_of(&base)? {
            return Err(Error::conversion(format!(
                "class '{}' is not a Tool subclass",
                class.name()?,
            )));
        }

        Ok(class)
    }
}
