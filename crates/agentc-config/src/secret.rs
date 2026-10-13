// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};
use std::{
    fmt::{Debug, Display, Formatter, Result as FmtResult},
    ops::{Deref, DerefMut},
};
use url::{ParseError, Url};

#[derive(Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Secret<T>(T);

impl<T> Secret<T> {
    pub fn new(value: T) -> Self {
        Self(value)
    }

    pub fn as_inner(&self) -> &T {
        &self.0
    }

    pub fn as_inner_mut(&mut self) -> &mut T {
        &mut self.0
    }

    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> Debug for Secret<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "Secret(*****)")
    }
}

impl<T> Display for Secret<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "*****")
    }
}

impl<T> Default for Secret<T>
where
    T: Default,
{
    fn default() -> Self {
        Self(T::default())
    }
}

impl<T> PartialEq for Secret<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T> Eq for Secret<T> where T: Eq {}

impl<T> Deref for Secret<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Secret<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SecretUrl(Url);

impl SecretUrl {
    pub fn new(value: Url) -> Self {
        Self(value)
    }

    pub fn parse(input: &str) -> Result<Self, ParseError> {
        Url::parse(input).map(Self)
    }

    pub fn as_inner(&self) -> &Url {
        &self.0
    }

    pub fn as_inner_mut(&mut self) -> &mut Url {
        &mut self.0
    }

    pub fn into_inner(self) -> Url {
        self.0
    }
}

impl Debug for SecretUrl {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "SecretUrl({self})")
    }
}

impl Display for SecretUrl {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let mut redacted = self.0.clone();

        if !redacted.username().is_empty() {
            redacted
                .set_username("*****")
                .ok();
        }

        if redacted.password().is_some() {
            redacted
                .set_password(Some("*****"))
                .ok();
        }

        write!(f, "{redacted}")
    }
}

impl Deref for SecretUrl {
    type Target = Url;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for SecretUrl {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_url_display_redacts_the_username_and_password() {
        assert_eq!(
            SecretUrl::parse("postgresql://agent:hunter2@db:5432/agent")
                .unwrap()
                .to_string(),
            "postgresql://*****:*****@db:5432/agent",
        );
    }

    #[test]
    fn secret_url_display_redacts_a_username_without_a_password() {
        assert_eq!(
            SecretUrl::parse("postgresql://agent@db/agent")
                .unwrap()
                .to_string(),
            "postgresql://*****@db/agent",
        );
    }

    #[test]
    fn secret_url_display_keeps_a_url_without_credentials() {
        assert_eq!(
            SecretUrl::parse("sqlite://database.db?mode=rwc")
                .unwrap()
                .to_string(),
            "sqlite://database.db?mode=rwc",
        );
    }

    #[test]
    fn secret_url_debug_wraps_the_redacted_url() {
        assert_eq!(
            format!(
                "{:?}",
                SecretUrl::parse("postgresql://agent:hunter2@db:5432/agent").unwrap(),
            ),
            "SecretUrl(postgresql://*****:*****@db:5432/agent)",
        );
    }

    #[test]
    fn secret_url_inner_keeps_the_credentials() {
        let url = SecretUrl::parse("postgresql://agent:hunter2@db:5432/agent").unwrap();

        assert_eq!(url.as_inner().password(), Some("hunter2"));
        assert_eq!(
            url.into_inner().as_str(),
            "postgresql://agent:hunter2@db:5432/agent",
        );
    }
}
