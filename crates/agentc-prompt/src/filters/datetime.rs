// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use chrono::{DateTime, format::StrftimeItems};
use chrono_tz::Tz;
use minijinja::{Error, ErrorKind, value::Kwargs};

pub struct DateTimeFilters;

impl DateTimeFilters {
    pub fn strftime(value: &str, format: &str, kwargs: Kwargs) -> Result<String, Error> {
        let tz = kwargs.get::<Option<&str>>("tz")?;

        kwargs.assert_all_used()?;

        let datetime = DateTime::parse_from_rfc3339(value).map_err(|e| {
            Error::new(
                ErrorKind::InvalidOperation,
                format!("'{value}' is not an RFC 3339 date and time"),
            )
            .with_source(e)
        })?;
        let items = StrftimeItems::new(format)
            .parse()
            .map_err(|e| {
                Error::new(
                    ErrorKind::InvalidOperation,
                    format!("'{format}' is not a valid strftime format"),
                )
                .with_source(e)
            })?;

        Ok(
            match tz {
                Some(tz) => datetime
                    .with_timezone(&tz.parse::<Tz>().map_err(|e| {
                        Error::new(
                            ErrorKind::InvalidOperation,
                            format!("'{tz}' is not an IANA time zone"),
                        )
                        .with_source(e)
                    })?)
                    .format_with_items(items.iter())
                    .to_string(),
                None => datetime
                    .format_with_items(items.iter())
                    .to_string(),
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use minijinja::{ErrorKind, Value, value::Kwargs};

    use crate::filters::datetime::DateTimeFilters;

    const VALUE: &str = "2026-10-06T17:00:00.123456789Z";

    fn kwargs(pairs: &[(&'static str, &'static str)]) -> Kwargs {
        pairs
            .iter()
            .map(|(key, value)| (*key, Value::from(*value)))
            .collect()
    }

    #[test]
    fn formats_date_only() {
        assert_eq!(
            DateTimeFilters::strftime(VALUE, "%Y-%m-%d", kwargs(&[])).unwrap(),
            "2026-10-06",
        );
    }

    #[test]
    fn formats_time_only() {
        assert_eq!(
            DateTimeFilters::strftime(VALUE, "%H:%M", kwargs(&[])).unwrap(),
            "17:00",
        );
    }

    #[test]
    fn formats_in_the_offset_the_value_carries_without_tz() {
        assert_eq!(
            DateTimeFilters::strftime(VALUE, "%H:%M %Z", kwargs(&[])).unwrap(),
            "17:00 +00:00",
        );
    }

    #[test]
    fn converts_to_daylight_saving_time_in_tz() {
        assert_eq!(
            DateTimeFilters::strftime(
                VALUE,
                "%H:%M %Z",
                kwargs(&[("tz", "America/New_York")]),
            )
            .unwrap(),
            "13:00 EDT",
        );
    }

    #[test]
    fn converts_to_standard_time_in_tz() {
        assert_eq!(
            DateTimeFilters::strftime(
                "2026-01-06T17:00:00Z",
                "%H:%M %Z",
                kwargs(&[("tz", "America/New_York")]),
            )
            .unwrap(),
            "12:00 EST",
        );
    }

    #[test]
    fn rejects_invalid_format_naming_it() {
        let error = DateTimeFilters::strftime(VALUE, "%Q", kwargs(&[])).unwrap_err();

        assert_eq!(error.kind(), ErrorKind::InvalidOperation);
        assert!(error.to_string().contains("'%Q'"));
    }

    #[test]
    fn rejects_value_that_is_not_rfc_3339_naming_it() {
        let error = DateTimeFilters::strftime("yesterday", "%Y", kwargs(&[])).unwrap_err();

        assert_eq!(error.kind(), ErrorKind::InvalidOperation);
        assert!(error.to_string().contains("'yesterday'"));
    }

    #[test]
    fn rejects_unknown_tz_naming_it() {
        let error = DateTimeFilters::strftime(
            VALUE,
            "%H:%M",
            kwargs(&[("tz", "Mars/Olympus_Mons")]),
        )
        .unwrap_err();

        assert_eq!(error.kind(), ErrorKind::InvalidOperation);
        assert!(error.to_string().contains("'Mars/Olympus_Mons'"));
    }

    #[test]
    fn rejects_unexpected_keyword_argument() {
        let error = DateTimeFilters::strftime(
            VALUE,
            "%H:%M",
            kwargs(&[("zone", "UTC")]),
        )
        .unwrap_err();

        assert_eq!(error.kind(), ErrorKind::TooManyArguments);
    }
}
