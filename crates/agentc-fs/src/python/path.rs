// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_executor_python::{
    backend::ExecutorBackend,
    guestpy::{
        errors::Error,
        handle::{Object, ObjectProtocol},
        marshal::{
            FromGuest,
            describe::{Describe, Expected},
        },
        scope::Enter,
    },
};

pub(crate) struct StrPath(String);

impl StrPath {
    pub(crate) fn into_inner(self) -> String {
        self.0
    }
}

impl<B: ExecutorBackend> FromGuest<B> for StrPath {
    type Owned = Self;

    fn from_guest<'py>(enter: &Enter<'py, B>, value: B::Value<'py>) -> Result<Self::Owned, Error> {
        if B::is_str(enter.token(), &value) {
            return Ok(Self(String::from_guest(enter, value)?));
        }

        if !B::has_attr(enter.token(), &value, "__fspath__") {
            return Err(
                Error::mismatch::<Self>(&B::type_name(enter.token(), &value))
            );
        }

        Ok(
            Self(
                Object::<B>::from_guest(enter, value)?
                    .call_method::<_, String>("__fspath__", ())?
            )
        )
    }
}

impl Describe for StrPath {
    fn describe(expected: &mut Expected) {
        expected.push("str");
        expected.push("os.PathLike[str]");
    }
}
