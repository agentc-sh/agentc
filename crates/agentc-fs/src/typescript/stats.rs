// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::time::SystemTime;

use agentc_executor_typescript::guestjs::{
    errors::Error,
    handle::{Class, Object, Scoped, Value},
    host::{ClassSpec, HostClass},
    marshal::{FromGuest, FromGuestBound, ToGuestBound},
    runtime::Scope,
};

use crate::fs::{FileType, Metadata};

struct GuestDate {
    millis: f64,
}

impl GuestDate {
    fn new(millis: f64) -> Self {
        Self { millis }
    }

    fn construct<'js>(&self, scope: &Scope<'js>) -> Result<Value, Error> {
        Value::from_guest(
            scope,
            Object::from_guest_bound(scope, scope.ctx().globals().into())?
                .get::<Class>("Date")?
                .construct((self.millis,))?
                .to_guest_bound(scope)?,
        )
    }
}

pub struct Stats {
    metadata: Metadata,
}

impl Stats {
    pub fn new(metadata: Metadata) -> Self {
        Self { metadata }
    }

    fn mode(&self) -> u32 {
        let file_type = match self.metadata.file_type() {
            FileType::File => 0o100000,
            FileType::Directory => 0o40000,
            FileType::Symlink => 0o120000,
            FileType::Fifo => 0o10000,
            FileType::Socket => 0o140000,
            FileType::BlockDevice => 0o60000,
            FileType::CharacterDevice => 0o20000,
            FileType::Other => 0,
        };

        file_type | self.metadata.permissions().mode()
    }

    fn millis(time: Option<SystemTime>) -> f64 {
        time.and_then(|time| {
            time.duration_since(SystemTime::UNIX_EPOCH)
                .ok()
        })
        .map(|elapsed| elapsed.as_secs_f64() * 1000.0)
        .unwrap_or_default()
    }

    fn accessed(&self) -> f64 {
        Self::millis(self.metadata.accessed())
    }

    fn modified(&self) -> f64 {
        Self::millis(self.metadata.modified())
    }

    fn changed(&self) -> f64 {
        self.metadata
            .changed()
            .or_else(|| self.metadata.modified())
            .and_then(|time| {
                time.duration_since(SystemTime::UNIX_EPOCH)
                    .ok()
            })
            .map(|elapsed| elapsed.as_secs_f64() * 1000.0)
            .unwrap_or_default()
    }

    fn created(&self) -> f64 {
        Self::millis(self.metadata.created())
    }
}

impl HostClass for Stats {
    const NAME: &'static str = "Stats";

    fn build(spec: &mut ClassSpec<Self>) {
        spec.getter("dev", |stats, _scope| Ok(stats.metadata.dev()));
        spec.getter("ino", |stats, _scope| Ok(stats.metadata.ino()));
        spec.getter("mode", |stats, _scope| Ok(stats.mode()));
        spec.getter("nlink", |stats, _scope| Ok(stats.metadata.nlink()));
        spec.getter("uid", |stats, _scope| Ok(stats.metadata.uid()));
        spec.getter("gid", |stats, _scope| Ok(stats.metadata.gid()));
        spec.getter("rdev", |stats, _scope| Ok(stats.metadata.rdev()));
        spec.getter("size", |stats, _scope| Ok(stats.metadata.len() as f64));
        spec.getter("blksize", |stats, _scope| Ok(stats.metadata.blksize()));
        spec.getter("blocks", |stats, _scope| Ok(stats.metadata.blocks()));
        spec.getter("atimeMs", |stats, _scope| Ok(stats.accessed()));
        spec.getter("mtimeMs", |stats, _scope| Ok(stats.modified()));
        spec.getter("ctimeMs", |stats, _scope| Ok(stats.changed()));
        spec.getter("birthtimeMs", |stats, _scope| Ok(stats.created()));
        spec.getter("atime", |stats, _scope| {
            // Constructed through the guest `Date` global because guestjs has no `Date` marshalling.
            Ok(Scoped::new({
                let date = GuestDate::new(stats.accessed());

                move |scope: &Scope| date.construct(scope)
            }))
        });
        spec.getter("mtime", |stats, _scope| {
            Ok(Scoped::new({
                let date = GuestDate::new(stats.modified());

                move |scope: &Scope| date.construct(scope)
            }))
        });
        spec.getter("ctime", |stats, _scope| {
            Ok(Scoped::new({
                let date = GuestDate::new(stats.changed());

                move |scope: &Scope| date.construct(scope)
            }))
        });
        spec.getter("birthtime", |stats, _scope| {
            Ok(Scoped::new({
                let date = GuestDate::new(stats.created());

                move |scope: &Scope| date.construct(scope)
            }))
        });
        spec.method("isFile", |stats, _scope, _args| {
            Ok(stats.metadata.file_type() == FileType::File)
        });
        spec.method("isDirectory", |stats, _scope, _args| {
            Ok(stats.metadata.file_type() == FileType::Directory)
        });
        spec.method("isDir", |stats, _scope, _args| {
            Ok(stats.metadata.file_type() == FileType::Directory)
        });
        spec.method("isSymbolicLink", |stats, _scope, _args| {
            Ok(stats.metadata.file_type() == FileType::Symlink)
        });
        spec.method("isSymlink", |stats, _scope, _args| {
            Ok(stats.metadata.file_type() == FileType::Symlink)
        });
        spec.method("isFIFO", |stats, _scope, _args| {
            Ok(stats.metadata.file_type() == FileType::Fifo)
        });
        spec.method("isBlockDevice", |stats, _scope, _args| {
            Ok(stats.metadata.file_type() == FileType::BlockDevice)
        });
        spec.method("isCharacterDevice", |stats, _scope, _args| {
            Ok(stats.metadata.file_type() == FileType::CharacterDevice)
        });
        spec.method("isSocket", |stats, _scope, _args| {
            Ok(stats.metadata.file_type() == FileType::Socket)
        });
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};

    use super::Stats;
    use crate::fs::{FileType, Metadata, Permissions};

    #[test]
    fn mode_recombines_the_file_type_and_permission_bits() {
        let stats = Stats::new(Metadata::new(FileType::Directory, 0, Permissions::new(0o755)));

        assert_eq!(stats.mode(), 0o40755);
    }

    #[test]
    fn millis_reports_zero_for_a_missing_timestamp() {
        assert_eq!(Stats::millis(None), 0.0);
    }

    #[test]
    fn changed_falls_back_to_modified() {
        let modified = SystemTime::UNIX_EPOCH + Duration::from_millis(1234);
        let stats = Stats::new(
            Metadata::new(FileType::File, 0, Permissions::new(0o644)).with_modified(modified),
        );

        assert_eq!(stats.changed(), 1234.0);
    }
}
