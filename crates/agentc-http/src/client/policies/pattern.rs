// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use regex::Regex;
use url::Url;
use urlpattern::{UrlPattern as CompiledPattern, UrlPatternInit, UrlPatternMatchInput};

use crate::client::{
    errors::HttpClientError,
    policy::{Denied, Policy},
};

/// The parts of a URL a pattern constrains.
///
/// Every unset field matches anything.
#[derive(Clone, Debug, Default)]
pub struct UrlPattern {
    pub protocol: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub hostname: Option<String>,
    pub port: Option<String>,
    pub pathname: Option<String>,
    pub search: Option<String>,
    pub hash: Option<String>,
}

impl UrlPattern {
    fn compile(self) -> Result<CompiledPattern, HttpClientError> {
        CompiledPattern::parse(self.into(), Default::default())
            .map_err(|error| HttpClientError::configuration(error.to_string()))
    }

    /// Parses a pattern written in the `https://*.example.com/v1/*` form.
    pub fn parse(pattern: impl AsRef<str>) -> Result<Self, HttpClientError> {
        UrlPatternInit::parse_constructor_string::<Regex>(pattern.as_ref(), None)
            .map(Self::from)
            .map_err(|error| HttpClientError::configuration(error.to_string()))
    }
}

impl From<UrlPattern> for UrlPatternInit {
    fn from(pattern: UrlPattern) -> Self {
        Self {
            protocol: pattern.protocol,
            username: pattern.username,
            password: pattern.password,
            hostname: pattern.hostname,
            port: pattern.port,
            pathname: pattern.pathname,
            search: pattern.search,
            hash: pattern.hash,
            base_url: None,
        }
    }
}

impl From<UrlPatternInit> for UrlPattern {
    fn from(init: UrlPatternInit) -> Self {
        Self {
            protocol: init.protocol,
            username: init.username,
            password: init.password,
            hostname: init.hostname,
            port: init.port,
            pathname: init.pathname,
            search: init.search,
            hash: init.hash,
        }
    }
}

/// Restricts requests to a set of URL patterns.
pub struct PatternPolicy {
    patterns: Vec<CompiledPattern>,
}

impl PatternPolicy {
    /// Permits destinations matching any of the given patterns.
    pub fn allow<I>(patterns: I) -> Result<Self, HttpClientError>
    where
        I: IntoIterator<Item = UrlPattern>,
    {
        Ok(Self {
            patterns: patterns
                .into_iter()
                .map(UrlPattern::compile)
                .collect::<Result<_, _>>()?,
        })
    }
}

impl Policy for PatternPolicy {
    fn name(&self) -> &'static str {
        "url-pattern"
    }

    fn check_url(&self, url: &Url) -> Result<(), Denied> {
        match self.patterns.iter().any(|pattern| {
            pattern
                .test(UrlPatternMatchInput::Url(url.clone()))
                .unwrap_or(false)
        }) {
            true => Ok(()),
            false => Err(Denied::new(format!("{url} matches no permitted pattern"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy(pattern: &str) -> PatternPolicy {
        PatternPolicy::allow([UrlPattern::parse(pattern).expect("test pattern parses")])
            .expect("test policy builds")
    }

    fn url(value: &str) -> Url {
        Url::parse(value).expect("test url parses")
    }

    #[test]
    fn a_wildcard_matches_any_number_of_labels() {
        assert!(
            policy("https://*.example.com/*")
                .check_url(&url("https://db.internal.example.com/health"))
                .is_ok()
        );
    }

    #[test]
    fn a_named_group_matches_exactly_one_label() {
        assert!(
            policy("https://:sub.example.com/*")
                .check_url(&url("https://api.example.com/v1"))
                .is_ok()
        );
        assert!(
            policy("https://:sub.example.com/*")
                .check_url(&url("https://db.internal.example.com/v1"))
                .is_err()
        );
    }

    #[test]
    fn a_wildcard_does_not_admit_a_sibling_domain() {
        assert!(
            policy("https://*.internal.example.com/*")
                .check_url(&url("https://evilinternal.example.com/"))
                .is_err()
        );
    }

    #[test]
    fn an_unset_component_matches_anything() {
        let policy = PatternPolicy::allow([UrlPattern {
            hostname: Some(String::from("api.example.com")),
            ..Default::default()
        }])
        .expect("test policy builds");

        assert!(
            policy
                .check_url(&url("http://api.example.com:8080/any"))
                .is_ok()
        );
        assert!(
            policy
                .check_url(&url("https://api.example.org/"))
                .is_err()
        );
    }

    #[test]
    fn a_relative_pattern_is_reported() {
        assert!(matches!(
            UrlPattern::parse("*.example.com"),
            Err(HttpClientError::Configuration { .. }),
        ));
    }
}
