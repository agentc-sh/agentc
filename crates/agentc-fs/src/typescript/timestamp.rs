// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::time::{SystemTime, UNIX_EPOCH};

use agentc_executor_typescript::{
    backend::ExecutorBackend,
    guestjs::{
        enter::Enter, errors::Error, eval::GuestGlobalsExt, handle::protocol::ObjectProtocol,
        marshal::ToGuest,
    },
};

pub(crate) struct Timestamp(SystemTime);

impl Timestamp {
    pub(crate) fn millis(&self) -> f64 {
        match self.0.duration_since(UNIX_EPOCH) {
            Ok(duration) => duration.as_secs_f64() * 1000.0,
            Err(error) => -error.duration().as_secs_f64() * 1000.0,
        }
    }
}

impl From<SystemTime> for Timestamp {
    fn from(time: SystemTime) -> Self {
        Self(time)
    }
}

impl<B: ExecutorBackend> ToGuest<B> for Timestamp {
    fn to_guest<'js>(self, enter: &Enter<'js, B>) -> Result<B::Value<'js>, Error> {
        enter
            .guest()
            .globals()?
            .class("Date")?
            .construct((self.millis(),))?
            .to_guest(enter)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn timestamps_convert_to_signed_epoch_millis() {
        assert_eq!(Timestamp::from(UNIX_EPOCH + Duration::from_millis(1500)).millis(), 1500.0,);
        assert_eq!(Timestamp::from(UNIX_EPOCH - Duration::from_millis(1500)).millis(), -1500.0,);
    }
}
