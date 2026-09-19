// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::time::{SystemTime, UNIX_EPOCH};

use agentc_executor_python::{
    backend::ExecutorBackend,
    guestpy::{
        errors::Error,
        handle::{Object, ObjectProtocol},
        marshal::ToGuest,
        scope::Enter,
    },
};

pub(crate) struct Timestamp(SystemTime);

impl Timestamp {
    fn seconds(&self) -> f64 {
        match self.0.duration_since(UNIX_EPOCH) {
            Ok(duration) => duration.as_secs_f64(),
            Err(error) => -error.duration().as_secs_f64(),
        }
    }
}

impl From<SystemTime> for Timestamp {
    fn from(time: SystemTime) -> Self {
        Self(time)
    }
}

impl<B: ExecutorBackend> ToGuest<B> for Timestamp {
    fn to_guest<'py>(self, enter: &Enter<'py, B>) -> Result<B::Value<'py>, Error> {
        let datetime = enter.guest().import("datetime")?;

        datetime
            .get::<Object<B>>("datetime")?
            .call_method::<_, Object<B>>(
                "fromtimestamp",
                (
                    self.seconds(),
                    datetime
                        .get::<Object<B>>("timezone")?
                        .get::<Object<B>>("utc")?,
                ),
            )?
            .to_guest(enter)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn timestamps_convert_to_signed_epoch_seconds() {
        assert_eq!(
            Timestamp::from(UNIX_EPOCH + Duration::from_millis(1500)).seconds(),
            1.5,
        );
        assert_eq!(
            Timestamp::from(UNIX_EPOCH - Duration::from_millis(1500)).seconds(),
            -1.5,
        );
    }
}
