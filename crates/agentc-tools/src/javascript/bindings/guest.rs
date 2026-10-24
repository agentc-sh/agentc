// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_executor_typescript::{
    backend::ExecutorBackend,
    guestjs::{
        enter::Enter,
        errors::Error,
        guest_class,
        handle::{class::Class, object::Object, promise::Promise, protocol::ObjectProtocol},
        host::class::HostClass,
        marshal::FromGuest,
    },
};

use crate::javascript::bindings::{input::ToolInput, output::ToolOutput, tool::Tool};

guest_class! {
    #[guestjs(crate_path = agentc_executor_typescript::guestjs, payload = Tool::<B>)]
    pub class GuestTool {
        fn execute(input: ToolInput<B>) -> Promise<B, ToolOutput>;
    }
}

pub struct GuestToolClass<B: ExecutorBackend>(Class<B, GuestTool<B>>);

impl<B: ExecutorBackend> GuestToolClass<B> {
    pub fn construct(&self) -> Result<GuestTool<B>, Error> {
        self.0.construct(())
    }

    pub fn class(&self) -> &Class<B, GuestTool<B>> {
        &self.0
    }
}

impl<B: ExecutorBackend> FromGuest<B> for GuestToolClass<B> {
    type Owned = Self;

    fn from_guest<'js>(enter: &Enter<'js, B>, value: B::Value<'js>) -> Result<Self::Owned, Error> {
        let class = Class::<B>::from_guest(enter, value)
            .map_err(|_| Error::unexpected("does not exist or is not a class"))?;

        match class
            .get::<Object<B>, _>("prototype")?
            .is_instance_of(&Tool::<B>::class(enter)?)?
        {
            true => Ok(Self(class.into_result::<GuestTool<B>>())),
            false => Err(Error::unexpected("is not a Tool subclass")),
        }
    }
}
