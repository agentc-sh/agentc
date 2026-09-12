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
};

use crate::python::bindings::{input::ToolInput, output::ToolOutput, tool::Tool};

guest_class! {
    #[guestpy(crate_path = agentc_executor_python::guestpy, payload = Tool)]
    pub class GuestTool {
        fn execute(input: ToolInput<B>) -> Coroutine<B, ToolOutput<B>>;
    }
}

pub(crate) struct ToolClass<B: ExecutorBackend> {
    class: Class<B, GuestTool<B>>,
    arguments: Vec<Object<B>>,
}

impl<B: ExecutorBackend> ToolClass<B> {
    pub(crate) fn class(&self) -> &Class<B, GuestTool<B>> {
        &self.class
    }

    pub(crate) fn args(&self) -> Option<&Object<B>> {
        self.arguments.first()
    }

    pub(crate) fn state(&self) -> Option<&Object<B>> {
        self.arguments.get(2)
    }
}

pub(crate) struct GuestToolClass;

impl<B: ExecutorBackend> FromGuest<B> for GuestToolClass {
    type Owned = ToolClass<B>;

    fn from_guest<'py>(enter: &Enter<'py, B>, value: B::Value<'py>) -> Result<Self::Owned, Error> {
        let class = Class::from_guest(enter, value)?;
        let base = Class::of::<Tool>(enter)?;

        if class.is(&base) || !class.is_subclass_of(&base)? {
            return Err(Error::conversion(format!(
                "class '{}' is not a Tool subclass",
                class.name()?,
            )));
        }

        Ok(ToolClass {
            arguments: class
                .generic_base_of(&base)?
                .map(|alias| alias.arguments())
                .transpose()?
                .unwrap_or_default(),
            class,
        })
    }
}
