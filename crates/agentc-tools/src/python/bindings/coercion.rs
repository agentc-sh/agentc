// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::collections::HashSet;

use agentc_executor_python::{
    backend::ExecutorBackend,
    guestpy::{
        backend::{Backend, BackendValues},
        errors::Error,
        guest::Guest,
        handle::{Module, Object, ObjectProtocol},
        marshal::ToGuest,
        scope::Enter,
    },
    json::Json,
};
use serde_json::Value;

pub(crate) trait Coercer<B: ExecutorBackend> {
    fn decodes(&self, target: &Object<B>) -> Result<bool, Error>;

    fn decode(&self, target: &Object<B>, value: Value) -> Result<Decoded<B>, Error>;

    fn encodes(&self, value: &Object<B>) -> Result<bool, Error>;

    fn encode(&self, value: Object<B>) -> Result<Value, Error>;
}

pub(crate) struct DataclassCoercer<B: ExecutorBackend> {
    dataclasses: Module<B>,
}

impl<B: ExecutorBackend> DataclassCoercer<B> {
    pub(crate) fn new(guest: &Guest<B>) -> Result<Self, Error> {
        Ok(Self {
            dataclasses: guest.import("dataclasses")?,
        })
    }
}

impl<B: ExecutorBackend> Coercer<B> for DataclassCoercer<B> {
    fn decodes(&self, target: &Object<B>) -> Result<bool, Error> {
        self.dataclasses
            .function("is_dataclass")?
            .call::<_, bool>((target.clone(),))
    }

    fn decode(&self, target: &Object<B>, value: Value) -> Result<Decoded<B>, Error> {
        let Value::Object(fields) = value else {
            return Err(Error::conversion(format!(
                "{} must be built from a JSON object",
                target.repr()?,
            )));
        };

        let names = self
            .dataclasses
            .function("fields")?
            .call::<_, Vec<Object<B>>>((target.clone(),))?
            .iter()
            .map(|field| field.get::<String>("name"))
            .collect::<Result<HashSet<_>, _>>()?;

        Ok(
            Decoded::Object(
                target.call_with::<_, _, Object<B>>(
                    (),
                    fields
                        .into_iter()
                        .filter(|(name, _)| names.contains(name))
                        .map(|(name, value)| (name, Json(value)))
                        .collect::<Vec<_>>(),
                )?
            )
        )
    }

    fn encodes(&self, value: &Object<B>) -> Result<bool, Error> {
        self.dataclasses
            .function("is_dataclass")?
            .call::<_, bool>((value.clone(),))
    }

    fn encode(&self, value: Object<B>) -> Result<Value, Error> {
        Ok(
            self.dataclasses
                .function("asdict")?
                .call::<_, Json>((value,))?
                .into_inner()
        )
    }
}

pub(crate) struct Coercion<B: ExecutorBackend> {
    coercers: Vec<Box<dyn Coercer<B>>>,
}

impl<B: ExecutorBackend> Coercion<B> {
    pub(crate) fn new(guest: &Guest<B>) -> Result<Self, Error> {
        Ok(Self {
            coercers: vec![Box::new(DataclassCoercer::new(guest)?)],
        })
    }

    pub(crate) fn decode(
        &self,
        target: Option<&Object<B>>,
        value: Value,
    ) -> Result<Decoded<B>, Error> {
        let Some(target) = target else {
            return Ok(Decoded::Json(value));
        };

        for coercer in &self.coercers {
            if coercer.decodes(target)? {
                return coercer.decode(target, value);
            }
        }

        Ok(Decoded::Json(value))
    }

    pub(crate) fn encode(&self, value: Object<B>) -> Result<Value, Error> {
        for coercer in &self.coercers {
            if coercer.encodes(&value)? {
                return coercer.encode(value);
            }
        }

        Ok(value.cast::<Json>()?.into_inner())
    }
}

pub(crate) enum Decoded<B: Backend + BackendValues> {
    Json(Value),
    Object(Object<B>),
}

impl<B: Backend + BackendValues> ToGuest<B> for Decoded<B> {
    fn to_guest<'py>(self, enter: &Enter<'py, B>) -> Result<B::Value<'py>, Error> {
        match self {
            Self::Json(value) => Json(value).to_guest(enter),
            Self::Object(object) => object.to_guest(enter),
        }
    }
}

impl<B: Backend + BackendValues> Clone for Decoded<B> {
    fn clone(&self) -> Self {
        match self {
            Self::Json(value) => Self::Json(value.clone()),
            Self::Object(object) => Self::Object(object.clone()),
        }
    }
}
