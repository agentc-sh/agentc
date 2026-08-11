// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_executor_typescript::guestjs::{errors::Error, host::Args, runtime::Scope, FromGuest};
use serde::Deserialize;

use crate::{
    fs::{AccessOptions, OpenOptions},
    typescript::{
        constants::{
            COPYFILE_EXCL, COPYFILE_FICLONE, COPYFILE_FICLONE_FORCE, O_APPEND, O_CREAT, O_EXCL,
            O_NOFOLLOW, O_TRUNC, R_OK, W_OK, X_OK,
        },
        encoding::Encoding,
    },
};

const OPEN_ACCESS_MASK: u64 = 0b11;
const OPEN_ACCESS_READ: u64 = 0;
const OPEN_ACCESS_WRITE: u64 = 1;
const OPEN_ACCESS_READ_WRITE: u64 = 2;
const SUPPORTED_OPEN_BITS: u64 = OPEN_ACCESS_MASK
    | O_CREAT as u64
    | O_EXCL as u64
    | O_TRUNC as u64
    | O_APPEND as u64
    | O_NOFOLLOW as u64;
const SUPPORTED_COPY_BITS: u32 =
    COPYFILE_EXCL as u32 | COPYFILE_FICLONE as u32 | COPYFILE_FICLONE_FORCE as u32;

pub struct OpenFlags {
    options: OpenOptions,
    creates: bool,
}

impl OpenFlags {
    fn new(options: OpenOptions, creates: bool) -> Self {
        Self { options, creates }
    }

    pub fn parse(flags: &str) -> Result<Self, Error> {
        match flags {
            "r" => Ok(Self::new(OpenOptions::new().read(true), false)),
            "r+" => Ok(Self::new(
                OpenOptions::new()
                    .read(true)
                    .write(true),
                false,
            )),
            "w" => Ok(Self::new(
                OpenOptions::new()
                    .write(true)
                    .truncate(true)
                    .create(true),
                true,
            )),
            "wx" => Ok(Self::new(
                OpenOptions::new()
                    .write(true)
                    .create_new(true),
                true,
            )),
            "w+" => Ok(Self::new(
                OpenOptions::new()
                    .read(true)
                    .write(true)
                    .truncate(true)
                    .create(true),
                true,
            )),
            "wx+" => Ok(Self::new(
                OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create_new(true),
                true,
            )),
            "a" => Ok(Self::new(
                OpenOptions::new()
                    .write(true)
                    .append(true)
                    .create(true),
                true,
            )),
            "ax" => Ok(Self::new(
                OpenOptions::new()
                    .write(true)
                    .append(true)
                    .create_new(true),
                true,
            )),
            "a+" => Ok(Self::new(
                OpenOptions::new()
                    .read(true)
                    .write(true)
                    .append(true)
                    .create(true),
                true,
            )),
            "ax+" => Ok(Self::new(
                OpenOptions::new()
                    .read(true)
                    .write(true)
                    .append(true)
                    .create_new(true),
                true,
            )),
            _ => Err(Error::conversion(format!("agentc:fs: invalid open flags {flags:?}"))),
        }
    }

    pub fn from_bits(bits: i64) -> Result<Self, Error> {
        let bits = bits as u64;
        let access_mode = bits & OPEN_ACCESS_MASK;

        let mut options = match access_mode {
            OPEN_ACCESS_READ => OpenOptions::new().read(true),
            OPEN_ACCESS_WRITE => OpenOptions::new().write(true),
            OPEN_ACCESS_READ_WRITE => OpenOptions::new()
                .read(true)
                .write(true),
            _ => {
                return Err(Error::conversion("agentc:fs: invalid open access mode"));
            }
        };

        if bits & O_EXCL as u64 != 0 && bits & O_CREAT as u64 == 0 {
            return Err(Error::conversion("agentc:fs: O_EXCL requires O_CREAT"));
        }

        let unsupported = bits & !SUPPORTED_OPEN_BITS;
        if unsupported != 0 {
            return Err(Error::conversion(format!(
                "agentc:fs: unsupported open flag bit {:#x}",
                unsupported & unsupported.wrapping_neg(),
            )));
        }

        if bits & O_CREAT as u64 != 0 {
            options = options.create(true);
        }

        if bits & O_EXCL as u64 != 0 {
            options = options.create_new(true);
        }

        if bits & O_TRUNC as u64 != 0 {
            options = options.truncate(true);
        }

        if bits & O_APPEND as u64 != 0 {
            options = options.append(true);
        }

        if bits & O_NOFOLLOW as u64 != 0 {
            options = options.follow_symlinks(false);
        }

        Ok(Self::new(options, bits & O_CREAT as u64 != 0))
    }

    pub fn from_args<'js>(
        scope: &Scope<'js>,
        args: &Args<'js>,
        index: usize,
    ) -> Result<Self, Error> {
        if let Ok(Some(flags)) = args.get_opt::<String>(scope, index) {
            return Self::parse(&flags);
        }

        if let Ok(Some(bits)) = args.get_opt::<i64>(scope, index) {
            return Self::from_bits(bits);
        }

        Self::parse("r")
    }

    pub fn options(&self) -> &OpenOptions {
        &self.options
    }

    pub fn creates(&self) -> bool {
        self.creates
    }
}

pub struct AccessMode;

impl AccessMode {
    pub fn options(mode: i32) -> AccessOptions {
        AccessOptions::new()
            .read(mode & R_OK != 0)
            .write(mode & W_OK != 0)
            .execute(mode & X_OK != 0)
    }
}

pub struct CopyMode;

impl CopyMode {
    pub fn exclusive(mode: i32) -> Result<bool, Error> {
        let mode = mode as u32;

        let unsupported = mode & !SUPPORTED_COPY_BITS;

        if unsupported != 0 {
            return Err(Error::conversion(format!(
                "agentc:fs: unsupported copy flag bit {:#x}",
                (unsupported as u64) & (unsupported as u64).wrapping_neg(),
            )));
        }

        if mode & COPYFILE_FICLONE_FORCE as u32 != 0 {
            return Err(Error::unexpected("agentc:fs: COPYFILE_FICLONE_FORCE is not supported"));
        }

        Ok(mode & COPYFILE_EXCL as u32 != 0)
    }
}

#[derive(Debug, Default, Deserialize, FromGuest)]
#[guestjs(crate_path = agentc_executor_typescript::guestjs)]
#[serde(rename_all = "camelCase")]
pub struct FileOptions {
    pub encoding: Option<String>,
    pub mode: Option<u32>,
    pub flag: Option<String>,
}

impl FileOptions {
    pub fn from_args<'js>(
        scope: &Scope<'js>,
        args: &Args<'js>,
        index: usize,
    ) -> Result<Self, Error> {
        if let Ok(Some(encoding)) = args.get_opt::<String>(scope, index) {
            return Ok(Self {
                encoding: Some(encoding),
                ..Default::default()
            });
        }

        Ok(args
            .get_opt::<Self>(scope, index)?
            .unwrap_or_default())
    }

    pub fn encoding(&self) -> Result<Option<Encoding>, Error> {
        self.encoding
            .as_deref()
            .map(Encoding::parse)
            .transpose()
    }

    pub fn open_flags(&self, default: &str) -> Result<OpenFlags, Error> {
        match self.flag.as_deref() {
            Some(flag) => OpenFlags::parse(flag),
            None => OpenFlags::parse(default),
        }
    }
}

#[derive(Debug, Default, Deserialize, FromGuest)]
#[guestjs(crate_path = agentc_executor_typescript::guestjs)]
#[serde(rename_all = "camelCase")]
pub struct ReaddirOptions {
    pub with_file_types: Option<bool>,
    pub recursive: Option<bool>,
}

#[derive(Debug, Default, Deserialize, FromGuest)]
#[guestjs(crate_path = agentc_executor_typescript::guestjs)]
#[serde(rename_all = "camelCase")]
pub struct MkdirOptions {
    pub recursive: Option<bool>,
    pub mode: Option<u32>,
}

#[derive(Debug, Default, Deserialize, FromGuest)]
#[guestjs(crate_path = agentc_executor_typescript::guestjs)]
#[serde(rename_all = "camelCase")]
pub struct RmOptions {
    pub recursive: Option<bool>,
    pub force: Option<bool>,
}

#[derive(Debug, Default, Deserialize, FromGuest)]
#[guestjs(crate_path = agentc_executor_typescript::guestjs)]
#[serde(rename_all = "camelCase")]
pub struct RmdirOptions {
    pub recursive: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::{AccessMode, CopyMode, OpenFlags};

    fn open_flags(flags: OpenFlags) -> (bool, bool, bool, bool, bool, bool) {
        (
            flags.options().is_read(),
            flags.options().is_write(),
            flags.options().is_append(),
            flags.options().is_truncate(),
            flags.options().is_create(),
            flags.options().is_create_new(),
        )
    }

    #[test]
    fn open_flags_parse_every_node_string() {
        assert_eq!(
            open_flags(OpenFlags::parse("r").unwrap()),
            (true, false, false, false, false, false)
        );
        assert_eq!(
            open_flags(OpenFlags::parse("r+").unwrap()),
            (true, true, false, false, false, false)
        );
        assert_eq!(
            open_flags(OpenFlags::parse("w").unwrap()),
            (false, true, false, true, true, false)
        );
        assert_eq!(
            open_flags(OpenFlags::parse("wx").unwrap()),
            (false, true, false, false, false, true)
        );
        assert_eq!(
            open_flags(OpenFlags::parse("w+").unwrap()),
            (true, true, false, true, true, false)
        );
        assert_eq!(
            open_flags(OpenFlags::parse("wx+").unwrap()),
            (true, true, false, false, false, true)
        );
        assert_eq!(
            open_flags(OpenFlags::parse("a").unwrap()),
            (false, true, true, false, true, false)
        );
        assert_eq!(
            open_flags(OpenFlags::parse("ax").unwrap()),
            (false, true, true, false, false, true)
        );
        assert_eq!(
            open_flags(OpenFlags::parse("a+").unwrap()),
            (true, true, true, false, true, false)
        );
        assert_eq!(
            open_flags(OpenFlags::parse("ax+").unwrap()),
            (true, true, true, false, false, true)
        );
    }

    #[test]
    fn open_flags_reject_sync_mode_strings() {
        assert!(matches!(
            OpenFlags::parse("rs"),
            Err(agentc_executor_typescript::guestjs::errors::Error::Conversion {
                message,
                ..
            }) if message == "agentc:fs: invalid open flags \"rs\""
        ));
    }

    #[test]
    fn open_flags_from_bits_maps_the_access_mode() {
        assert_eq!(
            open_flags(OpenFlags::from_bits(0).unwrap()),
            (true, false, false, false, false, false)
        );
        assert_eq!(
            open_flags(OpenFlags::from_bits(1).unwrap()),
            (false, true, false, false, false, false)
        );
        assert_eq!(
            open_flags(OpenFlags::from_bits(2).unwrap()),
            (true, true, false, false, false, false)
        );
    }

    #[test]
    fn open_flags_from_bits_requires_o_creat_for_o_excl() {
        assert!(matches!(
            OpenFlags::from_bits(128),
            Err(agentc_executor_typescript::guestjs::errors::Error::Conversion {
                message,
                ..
            }) if message == "agentc:fs: O_EXCL requires O_CREAT"
        ));
    }

    #[test]
    fn open_flags_from_bits_rejects_an_unsupported_bit() {
        assert!(matches!(
            OpenFlags::from_bits(4),
            Err(agentc_executor_typescript::guestjs::errors::Error::Conversion {
                message,
                ..
            }) if message == "agentc:fs: unsupported open flag bit 0x4"
        ));
    }

    #[test]
    fn access_mode_maps_each_bit() {
        let mode = AccessMode::options(4 | 2 | 1);

        assert!(mode.is_read());
        assert!(mode.is_write());
        assert!(mode.is_execute());
    }

    #[test]
    fn access_mode_zero_is_an_existence_check() {
        let mode = AccessMode::options(0);

        assert!(!mode.is_read());
        assert!(!mode.is_write());
        assert!(!mode.is_execute());
    }

    #[test]
    fn copy_mode_accepts_ficlone_and_rejects_ficlone_force() {
        assert!(!CopyMode::exclusive(2).unwrap());
        assert!(CopyMode::exclusive(1).unwrap());
        assert!(matches!(
            CopyMode::exclusive(4),
            Err(agentc_executor_typescript::guestjs::errors::Error::Unexpected {
                message,
                ..
            }) if message == "agentc:fs: COPYFILE_FICLONE_FORCE is not supported"
        ));
    }
}
