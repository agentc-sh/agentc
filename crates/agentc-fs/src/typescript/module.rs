// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::borrow::Cow;

use agentc_executor_typescript::{
    guestjs::{
        errors::Error,
        handle::Object,
        host::{Exports, HostClass, HostModule},
        marshal::ToGuestBound,
        runtime::Scope,
    },
    host::HostRuntime,
};
use bytes::Bytes;

use crate::{
    fs::{
        Dir, File, FileType, Owner, Permissions, SetOwnerOptions,
    },
    path::PathBuf,
    typescript::{
        constants::Constants,
        descriptors::Descriptors,
        dirent::Dirent,
        handle::FileHandle,
        options::{
            AccessMode, CopyMode, FileOptions, MkdirOptions, OpenFlags, ReaddirOptions,
            RmOptions, RmdirOptions,
        },
        stats::Stats,
        types::{
            BufferView, FileContents, FileData, ReaddirResult, WriteBuffer,
        },
    },
};

#[derive(Clone)]
pub struct FsModule {
    specifier: Cow<'static, str>,
    dir: Dir,
    runtime: HostRuntime,
}

impl FsModule {
    const DEFAULT_SPECIFIER: &'static str = "agentc:fs";

    pub fn new(dir: Dir, host_runtime: HostRuntime) -> Self {
        Self {
            specifier: Cow::Borrowed(Self::DEFAULT_SPECIFIER),
            dir,
            runtime: host_runtime,
        }
    }

    fn relative_name(base: &[u8], path: &[u8]) -> String {
        String::from_utf8_lossy(
            if base == b"/" {
                &path[1..]
            } else {
                &path[base.len() + 1..]
            },
        )
        .into_owned()
    }

    async fn access(self, path: String, mode: Option<i32>) -> Result<(), Error> {
        self.dir
            .access(path, &AccessMode::options(mode.unwrap_or_default()))
            .await?;

        Ok(())
    }

    async fn read_file(self, path: String, options: FileOptions) -> Result<FileContents, Error> {
        let mut file = self.dir
            .open_with_options(path, options.open_flags("r")?.options())
            .await?;

        let bytes = file.read_to_end().await?;

        match options.encoding()? {
            Some(encoding) => Ok(FileContents::Text(encoding.decode(&bytes))),
            None => Ok(FileContents::Bytes(Bytes::from(bytes))),
        }
    }

    async fn write_file(self, path: String, data: FileData, options: FileOptions) -> Result<(), Error> {
        let flags = options.open_flags("w")?;
        let mut file = self.dir
            .open_with_options(path.clone(), flags.options())
            .await?;

        file.write_all(data.into_bytes(options.encoding()?)?).await?;

        if let Some(mode) = options.mode && flags.creates() {
            self.dir
                .set_permissions(path, Permissions::new(mode))
                .await?;
        }

        Ok(())
    }

    async fn append_file(self, path: String, data: FileData, options: FileOptions) -> Result<(), Error> {
        let flags = options.open_flags("a")?;
        let mut file = self.dir
            .open_with_options(path.clone(), flags.options())
            .await?;

        file.write_all(data.into_bytes(options.encoding()?)?).await?;

        if let Some(mode) = options.mode && flags.creates() {
            self.dir
                .set_permissions(path, Permissions::new(mode))
                .await?;
        }

        Ok(())
    }

    async fn rename(self, from: String, to: String) -> Result<(), Error> {
        self.dir.rename(from, to).await?;

        Ok(())
    }

    async fn readdir(self, path: String, options: ReaddirOptions) -> Result<ReaddirResult, Error> {
        let dir = self.dir.open_dir(&path).await?;
        let mut entries = Vec::new();

        if options.recursive.unwrap_or(false) {
            let mut walk = dir.walk().await?;

            while let Some(entry) = walk.next().await? {
                entries.push(entry);
            }
        } else {
            let mut cursor = dir.entries().await?;

            while let Some(entry) = cursor.next().await? {
                entries.push(entry);
            }
        }

        entries.sort_by(|left, right| left.path().as_bytes().cmp(right.path().as_bytes()));

        if options.with_file_types.unwrap_or(false) {
            return Ok(ReaddirResult::Entries(
                entries
                    .iter()
                    .map(Dirent::new)
                    .collect(),
            ));
        }

        if options.recursive.unwrap_or(false) {
            return Ok(ReaddirResult::Names(
                entries
                    .into_iter()
                    .map(|entry| Self::relative_name(&dir.path().as_bytes().to_vec(), entry.path().as_bytes()))
                    .collect(),
            ));
        }

        Ok(ReaddirResult::Names(
            entries
                .into_iter()
                .map(|entry| entry.file_name().to_string_lossy())
                .collect(),
        ))
    }

    async fn mkdir(self, path: String, options: MkdirOptions) -> Result<String, Error> {
        let dir = if options.recursive.unwrap_or(false) {
            self.dir.create_dir_all(path.clone()).await?
        } else {
            self.dir.create_dir(path.clone()).await?
        };

        if let Some(mode) = options.mode {
            self.dir
                .set_permissions(path, Permissions::new(mode))
                .await?;
        }

        Ok(dir.path().to_string_lossy())
    }

    async fn mkdtemp(self, prefix: String) -> Result<String, Error> {
        Ok(self.dir.create_dir_temp(prefix).await?.path().to_string_lossy())
    }

    async fn rm(self, path: String, options: RmOptions) -> Result<(), Error> {
        let recursive = options.recursive.unwrap_or(false);
        let force = options.force.unwrap_or(false);

        match self.dir.symlink_metadata(&path).await {
            Ok(metadata) if metadata.file_type() == FileType::Directory => {
                let result = if recursive {
                    self.dir.remove_dir_all(path).await
                } else {
                    self.dir.remove_dir(path).await
                };

                if force {
                    Ok(())
                } else {
                    result.map_err(Into::into)
                }
            }
            Ok(_) => {
                let result = self.dir.remove_file(path).await;

                if force {
                    Ok(())
                } else {
                    result.map_err(Into::into)
                }
            }
            Err(_) if force => Ok(()),
            Err(error) => Err(error.into()),
        }
    }

    async fn rmdir(self, path: String, options: RmdirOptions) -> Result<(), Error> {
        if options.recursive.unwrap_or(false) {
            self.dir.remove_dir_all(path).await?;
        } else {
            self.dir.remove_dir(path).await?;
        }

        Ok(())
    }

    async fn unlink(self, path: String) -> Result<(), Error> {
        self.dir.remove_file(path).await?;

        Ok(())
    }

    async fn stat(self, path: String) -> Result<Stats, Error> {
        Ok(Stats::new(self.dir.metadata(path).await?))
    }

    async fn lstat(self, path: String) -> Result<Stats, Error> {
        Ok(Stats::new(self.dir.symlink_metadata(path).await?))
    }

    async fn chmod(self, path: String, mode: u32) -> Result<(), Error> {
        self.dir
            .set_permissions(path, Permissions::new(mode))
            .await?;

        Ok(())
    }

    async fn chown(self, path: String, uid: u32, gid: u32) -> Result<(), Error> {
        self.dir
            .set_owner(path, Owner::new().user(uid).group(gid))
            .await?;

        Ok(())
    }

    async fn lchown(self, path: String, uid: u32, gid: u32) -> Result<(), Error> {
        self.dir
            .set_owner_with_options(
                path,
                Owner::new().user(uid).group(gid),
                &SetOwnerOptions::new().follow_symlinks(false),
            )
            .await?;

        Ok(())
    }

    async fn symlink(self, target: String, path: String) -> Result<(), Error> {
        self.dir.symlink(target, path).await?;

        Ok(())
    }

    async fn readlink(self, path: String) -> Result<String, Error> {
        Ok(self.dir.read_link(path).await?.to_string_lossy())
    }

    async fn truncate(self, path: String, len: Option<u64>) -> Result<(), Error> {
        self.dir.truncate(path, len.unwrap_or_default()).await?;

        Ok(())
    }

    async fn copy_file(self, from: String, to: String, mode: Option<i32>) -> Result<(), Error> {
        let exclusive = CopyMode::exclusive(mode.unwrap_or_default())?;
        let mut source = self
            .dir
            .options()
            .read(true)
            .open(from)
            .await?;
        let bytes = source.read_to_end().await?;
        let mut destination = if exclusive {
            self.dir
                .options()
                .write(true)
                .create(true)
                .create_new(true)
                .open(to)
                .await?
        } else {
            self.dir
                .options()
                .write(true)
                .create(true)
                .truncate(true)
                .open(to)
                .await?
        };

        destination.write_all(bytes).await?;

        Ok(())
    }

    async fn open(self, path: String, flags: OpenFlags, mode: Option<u32>) -> Result<(PathBuf, File), Error> {
        let resolved = self.dir.resolve(&path)?;
        let file = self.dir
            .open_with_options(path, flags.options())
            .await?;

        if let Some(mode) = mode {
            if flags.creates() {
                self.dir
                    .set_permissions(resolved.clone(), Permissions::new(mode))
                    .await?;
            }
        }

        Ok((resolved, file))
    }

    pub fn with_specifier(mut self, specifier: impl Into<Cow<'static, str>>) -> Self {
        self.specifier = specifier.into();
        self
    }

    pub fn specifier(&self) -> &str {
        &self.specifier
    }
}

impl HostModule for FsModule {
    fn name(&self) -> &str {
        self.specifier()
    }

    fn initialize<'js>(&self, scope: &Scope<'js>) -> Result<(), Error> {
        let module = scope.host_module(self.specifier())?;
        let globals = scope.ctx().globals();

        globals.set(
            Stats::NAME,
            module
                .class(Stats::NAME)?
                .to_guest_bound(scope)?,
        )?;
        globals.set(
            Dirent::NAME,
            module
                .class(Dirent::NAME)?
                .to_guest_bound(scope)?,
        )?;
        globals.set(
            FileHandle::NAME,
            module
                .class(FileHandle::NAME)?
                .to_guest_bound(scope)?,
        )?;

        Ok(())
    }

    fn build(&self, exports: &mut Exports) {
        let descriptors = Descriptors::new();

        exports.class::<Stats>();
        exports.class::<Dirent>();
        exports.class::<FileHandle>();
        exports.object("constants", Constants::build);

        exports.async_function("access", {
            let module = self.clone();

            move |scope, args| {
                let module = module.clone();
                let path = args.get_owned::<String>(scope, 0)?;
                let mode = args.get_opt::<i32>(scope, 1)?;

                Ok(async move { module.access(path, mode).await })
            }
        });
        exports.async_function("readFile", {
            let module = self.clone();

            move |scope, args| {
                let module = module.clone();
                let path = args.get_owned::<String>(scope, 0)?;
                let options = FileOptions::from_args(scope, &args, 1)?;

                Ok(async move { module.read_file(path, options).await })
            }
        });
        exports.async_function("writeFile", {
            let module = self.clone();

            move |scope, args| {
                let module = module.clone();
                let path = args.get_owned::<String>(scope, 0)?;
                let data = FileData::from_args(scope, &args, 1)?;
                let options = FileOptions::from_args(scope, &args, 2)?;

                Ok(async move { module.write_file(path, data, options).await })
            }
        });

        exports.async_function("appendFile", {
            let module = self.clone();

            move |scope, args| {
                let module = module.clone();
                let path = args.get_owned::<String>(scope, 0)?;
                let data = FileData::from_args(scope, &args, 1)?;
                let options = FileOptions::from_args(scope, &args, 2)?;

                Ok(async move { module.append_file(path, data, options).await })
            }
        });

        exports.async_function("rename", {
            let module = self.clone();

            move |scope, args| {
                let module = module.clone();
                let from = args.get_owned::<String>(scope, 0)?;
                let to = args.get_owned::<String>(scope, 1)?;

                Ok(async move { module.rename(from, to).await })
            }
        });

        exports.async_function("readdir", {
            let module = self.clone();

            move |scope, args| {
                let module = module.clone();
                let path = args.get_owned::<String>(scope, 0)?;
                let options = args.get_opt::<ReaddirOptions>(scope, 1)?.unwrap_or_default();

                Ok(async move { module.readdir(path, options).await })
            }
        });

        exports.async_function("mkdir", {
            let module = self.clone();

            move |scope, args| {
                let module = module.clone();
                let path = args.get_owned::<String>(scope, 0)?;
                let options = args.get_opt::<MkdirOptions>(scope, 1)?.unwrap_or_default();

                Ok(async move { module.mkdir(path, options).await })
            }
        });

        exports.async_function("mkdtemp", {
            let module = self.clone();

            move |scope, args| {
                let module = module.clone();
                let prefix = args.get_owned::<String>(scope, 0)?;

                Ok(async move { module.mkdtemp(prefix).await })
            }
        });

        exports.async_function("rm", {
            let module = self.clone();

            move |scope, args| {
                let module = module.clone();
                let path = args.get_owned::<String>(scope, 0)?;
                let options = args.get_opt::<RmOptions>(scope, 1)?.unwrap_or_default();

                Ok(async move { module.rm(path, options).await })
            }
        });

        exports.async_function("rmdir", {
            let module = self.clone();

            move |scope, args| {
                let module = module.clone();
                let path = args.get_owned::<String>(scope, 0)?;
                let options = args.get_opt::<RmdirOptions>(scope, 1)?.unwrap_or_default();

                Ok(async move { module.rmdir(path, options).await })
            }
        });

        exports.async_function("unlink", {
            let module = self.clone();

            move |scope, args| {
                let module = module.clone();
                let path = args.get_owned::<String>(scope, 0)?;

                Ok(async move { module.unlink(path).await })
            }
        });

        exports.async_function("stat", {
            let module = self.clone();

            move |scope, args| {
                let module = module.clone();
                let path = args.get_owned::<String>(scope, 0)?;

                Ok(async move { module.stat(path).await })
            }
        });

        exports.async_function("lstat", {
            let module = self.clone();

            move |scope, args| {
                let module = module.clone();
                let path = args.get_owned::<String>(scope, 0)?;

                Ok(async move { module.lstat(path).await })
            }
        });

        exports.async_function("chmod", {
            let module = self.clone();

            move |scope, args| {
                let module = module.clone();
                let path = args.get_owned::<String>(scope, 0)?;
                let mode = args.get::<u32>(scope, 1)?;

                Ok(async move { module.chmod(path, mode).await })
            }
        });

        exports.async_function("chown", {
            let module = self.clone();

            move |scope, args| {
                let module = module.clone();
                let path = args.get_owned::<String>(scope, 0)?;
                let uid = args.get::<u32>(scope, 1)?;
                let gid = args.get::<u32>(scope, 2)?;

                Ok(async move { module.chown(path, uid, gid).await })
            }
        });

        exports.async_function("lchown", {
            let module = self.clone();

            move |scope, args| {
                let module = module.clone();
                let path = args.get_owned::<String>(scope, 0)?;
                let uid = args.get::<u32>(scope, 1)?;
                let gid = args.get::<u32>(scope, 2)?;

                Ok(async move { module.lchown(path, uid, gid).await })
            }
        });

        exports.async_function("symlink", {
            let module = self.clone();

            move |scope, args| {
                let module = module.clone();
                let target = args.get_owned::<String>(scope, 0)?;
                let path = args.get_owned::<String>(scope, 1)?;

                Ok(async move { module.symlink(target, path).await })
            }
        });

        exports.async_function("readlink", {
            let module = self.clone();

            move |scope, args| {
                let module = module.clone();
                let path = args.get_owned::<String>(scope, 0)?;

                Ok(async move { module.readlink(path).await })
            }
        });

        exports.async_function("truncate", {
            let module = self.clone();

            move |scope, args| {
                let module = module.clone();
                let path = args.get_owned::<String>(scope, 0)?;
                let len = args.get_opt::<u64>(scope, 1)?;

                Ok(async move { module.truncate(path, len).await })
            }
        });

        exports.async_function("copyFile", {
            let module = self.clone();

            move |scope, args| {
                let module = module.clone();
                let from = args.get_owned::<String>(scope, 0)?;
                let to = args.get_owned::<String>(scope, 1)?;
                let mode = args.get_opt::<i32>(scope, 2)?;

                Ok(async move { module.copy_file(from, to, mode).await })
            }
        });
        exports.async_function("open", {
            let module = self.clone();
            let descriptors = descriptors.clone();

            move |scope, args| {
                let module = module.clone();
                let descriptors = descriptors.clone();
                let path = args.get_owned::<String>(scope, 0)?;
                let flags = OpenFlags::from_args(scope, &args, 1)?;
                let mode = args.get_opt::<u32>(scope, 2)?;

                Ok(async move {
                    let (resolved, file) = module.clone().open(path, flags, mode).await?;
                    let fd = descriptors.insert(resolved.clone(), file);

                    Ok(FileHandle::new(fd, resolved, module.dir.clone(), descriptors))
                })
            }
        });

        exports.function("accessSync", {
            let module = self.clone();

            move |scope, args| {
                // `block_on` reports a bridge failure; the inner result is the operation's own.
                let path = args.get_owned::<String>(scope, 0)?;
                let mode = args.get_opt::<i32>(scope, 1)?;

                module
                    .runtime
                    .block_on(module.clone().access(path, mode))
                    .map_err(|error| Error::unexpected(format!("agentc:fs: {error}")))?
            }
        });
        exports.function("readFileSync", {
            let module = self.clone();

            move |scope, args| {
                let path = args.get_owned::<String>(scope, 0)?;
                let options = FileOptions::from_args(scope, &args, 1)?;

                module
                    .runtime
                    .block_on(module.clone().read_file(path, options))
                    .map_err(|error| Error::unexpected(format!("agentc:fs: {error}")))?
            }
        });
        exports.function("writeFileSync", {
            let module = self.clone();

            move |scope, args| {
                let path = args.get_owned::<String>(scope, 0)?;
                let data = FileData::from_args(scope, &args, 1)?;
                let options = FileOptions::from_args(scope, &args, 2)?;

                module
                    .runtime
                    .block_on(module.clone().write_file(path, data, options))
                    .map_err(|error| Error::unexpected(format!("agentc:fs: {error}")))?
            }
        });

        exports.function("appendFileSync", {
            let module = self.clone();

            move |scope, args| {
                let path = args.get_owned::<String>(scope, 0)?;
                let data = FileData::from_args(scope, &args, 1)?;
                let options = FileOptions::from_args(scope, &args, 2)?;

                module
                    .runtime
                    .block_on(module.clone().append_file(path, data, options))
                    .map_err(|error| Error::unexpected(format!("agentc:fs: {error}")))?
            }
        });

        exports.function("renameSync", {
            let module = self.clone();

            move |scope, args| {
                let from = args.get_owned::<String>(scope, 0)?;
                let to = args.get_owned::<String>(scope, 1)?;

                module
                    .runtime
                    .block_on(module.clone().rename(from, to))
                    .map_err(|error| Error::unexpected(format!("agentc:fs: {error}")))?
            }
        });

        exports.function("readdirSync", {
            let module = self.clone();

            move |scope, args| {
                let path = args.get_owned::<String>(scope, 0)?;
                let options = args.get_opt::<ReaddirOptions>(scope, 1)?.unwrap_or_default();

                module
                    .runtime
                    .block_on(module.clone().readdir(path, options))
                    .map_err(|error| Error::unexpected(format!("agentc:fs: {error}")))?
            }
        });

        exports.function("mkdirSync", {
            let module = self.clone();

            move |scope, args| {
                let path = args.get_owned::<String>(scope, 0)?;
                let options = args.get_opt::<MkdirOptions>(scope, 1)?.unwrap_or_default();

                module
                    .runtime
                    .block_on(module.clone().mkdir(path, options))
                    .map_err(|error| Error::unexpected(format!("agentc:fs: {error}")))?
            }
        });

        exports.function("mkdtempSync", {
            let module = self.clone();

            move |scope, args| {
                let prefix = args.get_owned::<String>(scope, 0)?;

                module
                    .runtime
                    .block_on(module.clone().mkdtemp(prefix))
                    .map_err(|error| Error::unexpected(format!("agentc:fs: {error}")))?
            }
        });

        exports.function("rmSync", {
            let module = self.clone();

            move |scope, args| {
                let path = args.get_owned::<String>(scope, 0)?;
                let options = args.get_opt::<RmOptions>(scope, 1)?.unwrap_or_default();

                module
                    .runtime
                    .block_on(module.clone().rm(path, options))
                    .map_err(|error| Error::unexpected(format!("agentc:fs: {error}")))?
            }
        });

        exports.function("rmdirSync", {
            let module = self.clone();

            move |scope, args| {
                let path = args.get_owned::<String>(scope, 0)?;
                let options = args.get_opt::<RmdirOptions>(scope, 1)?.unwrap_or_default();

                module
                    .runtime
                    .block_on(module.clone().rmdir(path, options))
                    .map_err(|error| Error::unexpected(format!("agentc:fs: {error}")))?
            }
        });

        exports.function("unlinkSync", {
            let module = self.clone();

            move |scope, args| {
                let path = args.get_owned::<String>(scope, 0)?;

                module
                    .runtime
                    .block_on(module.clone().unlink(path))
                    .map_err(|error| Error::unexpected(format!("agentc:fs: {error}")))?
            }
        });

        exports.function("statSync", {
            let module = self.clone();

            move |scope, args| {
                let path = args.get_owned::<String>(scope, 0)?;

                module
                    .runtime
                    .block_on(module.clone().stat(path))
                    .map_err(|error| Error::unexpected(format!("agentc:fs: {error}")))?
            }
        });

        exports.function("lstatSync", {
            let module = self.clone();

            move |scope, args| {
                let path = args.get_owned::<String>(scope, 0)?;

                module
                    .runtime
                    .block_on(module.clone().lstat(path))
                    .map_err(|error| Error::unexpected(format!("agentc:fs: {error}")))?
            }
        });

        exports.function("chmodSync", {
            let module = self.clone();

            move |scope, args| {
                let path = args.get_owned::<String>(scope, 0)?;
                let mode = args.get::<u32>(scope, 1)?;

                module
                    .runtime
                    .block_on(module.clone().chmod(path, mode))
                    .map_err(|error| Error::unexpected(format!("agentc:fs: {error}")))?
            }
        });

        exports.function("chownSync", {
            let module = self.clone();

            move |scope, args| {
                let path = args.get_owned::<String>(scope, 0)?;
                let uid = args.get::<u32>(scope, 1)?;
                let gid = args.get::<u32>(scope, 2)?;

                module
                    .runtime
                    .block_on(module.clone().chown(path, uid, gid))
                    .map_err(|error| Error::unexpected(format!("agentc:fs: {error}")))?
            }
        });

        exports.function("lchownSync", {
            let module = self.clone();

            move |scope, args| {
                let path = args.get_owned::<String>(scope, 0)?;
                let uid = args.get::<u32>(scope, 1)?;
                let gid = args.get::<u32>(scope, 2)?;

                module
                    .runtime
                    .block_on(module.clone().lchown(path, uid, gid))
                    .map_err(|error| Error::unexpected(format!("agentc:fs: {error}")))?
            }
        });

        exports.function("symlinkSync", {
            let module = self.clone();

            move |scope, args| {
                let target = args.get_owned::<String>(scope, 0)?;
                let path = args.get_owned::<String>(scope, 1)?;

                module
                    .runtime
                    .block_on(module.clone().symlink(target, path))
                    .map_err(|error| Error::unexpected(format!("agentc:fs: {error}")))?
            }
        });

        exports.function("readlinkSync", {
            let module = self.clone();

            move |scope, args| {
                let path = args.get_owned::<String>(scope, 0)?;

                module
                    .runtime
                    .block_on(module.clone().readlink(path))
                    .map_err(|error| Error::unexpected(format!("agentc:fs: {error}")))?
            }
        });

        exports.function("truncateSync", {
            let module = self.clone();

            move |scope, args| {
                let path = args.get_owned::<String>(scope, 0)?;
                let len = args.get_opt::<u64>(scope, 1)?;

                module
                    .runtime
                    .block_on(module.clone().truncate(path, len))
                    .map_err(|error| Error::unexpected(format!("agentc:fs: {error}")))?
            }
        });

        exports.function("copyFileSync", {
            let module = self.clone();

            move |scope, args| {
                let from = args.get_owned::<String>(scope, 0)?;
                let to = args.get_owned::<String>(scope, 1)?;
                let mode = args.get_opt::<i32>(scope, 2)?;

                module
                    .runtime
                    .block_on(module.clone().copy_file(from, to, mode))
                    .map_err(|error| Error::unexpected(format!("agentc:fs: {error}")))?
            }
        });
        exports.function("openSync", {
            let module = self.clone();
            let descriptors = descriptors.clone();

            move |scope, args| {
                let path = args.get_owned::<String>(scope, 0)?;
                let flags = OpenFlags::from_args(scope, &args, 1)?;
                let mode = args.get_opt::<u32>(scope, 2)?;

                let (resolved, file) = module
                    .runtime
                    .block_on(module.clone().open(path, flags, mode))
                    .map_err(|error| Error::unexpected(format!("agentc:fs: {error}")))??;

                Ok(descriptors.insert(resolved, file))
            }
        });
        exports.function("closeSync", {
            let descriptors = descriptors.clone();

            move |scope, args| {
                descriptors.close(args.get::<u32>(scope, 0)?)?;

                Ok(())
            }
        });
        exports.function("readSync", {
            let module = self.clone();
            let descriptors = descriptors.clone();

            move |scope, args| {
                let fd = args.get::<u32>(scope, 0)?;
                let buffer = args.get::<Object>(scope, 1)?;
                let offset = args.get_opt::<u32>(scope, 2)?.unwrap_or_default() as usize;
                let length = args
                    .get_opt::<u32>(scope, 3)?
                    .map(|length| length as usize)
                    .unwrap_or(BufferView::checked_length(BufferView::len(&buffer)?, offset)?);
                let position = args.get_opt::<u64>(scope, 4)?;
                let mut session = descriptors.take(fd)?;
                let (session, result) = module
                    .runtime
                    .block_on(async move {
                        let result = session.read(length, position).await;

                        (session, result)
                    })
                    .map_err(|error| Error::unexpected(format!("agentc:fs: {error}")))?;

                descriptors.restore(fd, session);

                let bytes = result?;

                BufferView::write(&buffer, offset, &bytes)?;

                Ok(bytes.len() as u32)
            }
        });
        exports.function("writeSync", {
            let module = self.clone();
            let descriptors = descriptors.clone();

            move |scope, args| {
                let fd = args.get::<u32>(scope, 0)?;
                let request = WriteBuffer::from_args(scope, &args, 2)?;
                let mut session = descriptors.take(fd)?;
                let (session, result) = module
                    .runtime
                    .block_on(async move {
                        let result = session.write(request.bytes, request.position).await;

                        (session, result)
                    })
                    .map_err(|error| Error::unexpected(format!("agentc:fs: {error}")))?;

                descriptors.restore(fd, session);

                Ok(result? as u32)
            }
        });
        exports.function("fstatSync", {
            let module = self.clone();
            let descriptors = descriptors.clone();

            move |scope, args| {
                let path = descriptors.path(args.get::<u32>(scope, 0)?)?;
                let dir = module.dir.clone();

                module
                    .runtime
                    .block_on(async move { dir.metadata(path).await })
                    .map(|result| result.map(Stats::new).map_err(Into::into))
                    .map_err(|error| Error::unexpected(format!("agentc:fs: {error}")))?
            }
        });
        exports.function("ftruncateSync", {
            let module = self.clone();
            let descriptors = descriptors.clone();

            move |scope, args| {
                let fd = args.get::<u32>(scope, 0)?;
                let len = args.get_opt::<u64>(scope, 1)?.unwrap_or_default();
                let mut session = descriptors.take(fd)?;
                let (session, result) = module
                    .runtime
                    .block_on(async move {
                        let result = session.truncate(len).await;

                        (session, result)
                    })
                    .map_err(|error| Error::unexpected(format!("agentc:fs: {error}")))?;

                descriptors.restore(fd, session);

                result.map_err(Into::into)
            }
        });
        exports.function("fsyncSync", {
            let module = self.clone();
            let descriptors = descriptors.clone();

            move |scope, args| {
                let fd = args.get::<u32>(scope, 0)?;
                let mut session = descriptors.take(fd)?;
                let (session, result) = module
                    .runtime
                    .block_on(async move {
                        let result = session.sync_all().await;

                        (session, result)
                    })
                    .map_err(|error| Error::unexpected(format!("agentc:fs: {error}")))?;

                descriptors.restore(fd, session);

                result.map_err(Into::into)
            }
        });
        exports.function("fdatasyncSync", {
            let module = self.clone();
            let descriptors = descriptors.clone();

            move |scope, args| {
                let fd = args.get::<u32>(scope, 0)?;
                let mut session = descriptors.take(fd)?;
                let (session, result) = module
                    .runtime
                    .block_on(async move {
                        let result = session.sync_data().await;

                        (session, result)
                    })
                    .map_err(|error| Error::unexpected(format!("agentc:fs: {error}")))?;

                descriptors.restore(fd, session);

                result.map_err(Into::into)
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use agentc_executor_typescript::{
        executor::Executor,
        guestjs::{
            handle::Promise,
            marshal::{FromGuest, ToGuestArgs},
        },
        host::HostRuntime,
    };

    use crate::{fs::{Fs, Dir}, typescript::module::FsModule};

    const FS_SOURCE: &str = r#"
import {
    appendFile,
    chmod,
    closeSync,
    constants,
    copyFile,
    fdatasyncSync,
    fstatSync,
    ftruncateSync,
    fsyncSync,
    lstat,
    mkdir,
    mkdtemp,
    open,
    openSync,
    readFile,
    readFileSync,
    readSync,
    readdir,
    readlink,
    rename,
    rm,
    stat,
    statSync,
    symlink,
    truncate,
    writeSync,
    writeFile,
    Dirent as ImportedDirent,
    FileHandle as ImportedFileHandle,
    Stats as ImportedStats,
} from "agentc:fs";

export async function writeReadBytes(path) {
    await writeFile(path, Uint8Array.from([1, 2, 3]));

    return Array.from(await readFile(path)).join(",");
}

export async function writeReadUtf8(path) {
    await writeFile(path, "hello", "utf8");

    return await readFile(path, "utf8");
}

export async function appendText(path) {
    await writeFile(path, "hello", "utf8");
    await appendFile(path, " world", "utf8");

    return await readFile(path, "utf8");
}

export async function readEncoded(path) {
    await writeFile(path, Uint8Array.from([104, 101, 108, 108, 111]));

    return `${await readFile(path, "hex")}:${await readFile(path, "base64")}`;
}

export async function makeNested(path) {
    return await mkdir(path, { recursive: true });
}

export async function makeTemps(prefix) {
    const first = await mkdtemp(prefix);
    const second = await mkdtemp(prefix);

    return `${first !== second}:${first.startsWith(prefix)}:${second.startsWith(prefix)}`;
}

export async function sortedNames(path) {
    return (await readdir(path)).join(",");
}

export async function direntSummary(path) {
    return (await readdir(path, { withFileTypes: true }))
        .map(entry => `${entry.name}:${entry.parentPath}:${entry.isFile()}:${entry.isDirectory()}`)
        .join("|");
}

export async function recursiveNames(path) {
    return (await readdir(path, { recursive: true })).join(",");
}

export async function statKinds(path) {
    return `${(await stat(path)).isFile()}:${(await lstat(path)).isSymbolicLink()}`;
}

export async function statDates(path) {
    const stats = await stat(path);

    return stats.mtime instanceof Date
        && stats.mtime.getTime() === Math.trunc(stats.mtimeMs);
}

export async function statModeBits(path) {
    return ((await stat(path)).mode & constants.S_IFMT) === constants.S_IFREG;
}

export async function chmodMode(path) {
    await chmod(path, 0o600);

    return (await stat(path)).mode & 0o7777;
}

export async function rmForce(path) {
    await rm(path, { force: true });

    return "ok";
}

export async function rmRecursive(path) {
    await rm(path, { recursive: true });

    try {
        await stat(path);

        return "present";
    } catch {
        return "gone";
    }
}

export async function renameRead(from, to) {
    await rename(from, to);

    return await readFile(to, "utf8");
}

export async function copyExclusive(from, to) {
    try {
        await copyFile(from, to, constants.COPYFILE_EXCL);

        return "copied";
    } catch (error) {
        return error.message;
    }
}

export async function truncateRead(path, length) {
    await truncate(path, length);

    return await readFile(path, "utf8");
}

export async function linkTarget(path) {
    return await readlink(path);
}

export async function writeText(path, text) {
    await writeFile(path, text, "utf8");

    return "ok";
}

export async function missingMessage(path) {
    try {
        await readFile(path, "utf8");

        return "ok";
    } catch (error) {
        return error.message;
    }
}

export async function syncSurface(path) {
    const text = readFileSync(path, "utf8");
    const stats = statSync(path);

    return typeof text === "string"
        && !("then" in Object(text))
        && !("then" in stats)
        && stats.isFile();
}

export async function openReadInto(path) {
    const handle = await open(path, "r");
    const buffer = new Uint8Array(5);
    const result = await handle.read(buffer, 0, 5, 0);

    await handle.close();

    return `${result.bytesRead}:${Array.from(buffer).join(",")}`;
}

export async function identity(path) {
    const handle = await open(path, "r");
    const buffer = new Uint8Array(5);
    const result = await handle.read(buffer, 0, 5, 0);

    await handle.close();

    return result.buffer === buffer;
}

export async function readWithoutBuffer(path) {
    const handle = await open(path, "r");
    const result = await handle.read();

    await handle.close();

    return `${result.bytesRead}:${Array.from(result.buffer).join(",")}`;
}

export async function readAtPosition(path) {
    const handle = await open(path, "r+");
    const positioned = await handle.read(new Uint8Array(2), 0, 2, 1);
    const current = await handle.read(new Uint8Array(2), 0, 2);

    await handle.close();

    return `${Array.from(positioned.buffer).join(",")}:${Array.from(current.buffer).join(",")}`;
}

export async function writeThroughHandle(path) {
    const handle = await open(path, "a+");

    await handle.write(" world");
    await handle.close();

    return await readFile(path, "utf8");
}

export async function syncReadInto(path) {
    const fd = openSync(path, "r");
    const buffer = new Uint8Array(5);
    const bytesRead = readSync(fd, buffer, 0, 5, 0);

    closeSync(fd);

    return `${bytesRead}:${Array.from(buffer).join(",")}`;
}

export async function syncWriteFrom(path) {
    const fd = openSync(path, "w+");
    const buffer = Uint8Array.from([104, 101, 108, 108, 111]);
    const bytesWritten = writeSync(fd, buffer, 0, 5, 0);

    closeSync(fd);

    return `${bytesWritten}:${await readFile(path, "utf8")}`;
}

export async function readSubarray(path) {
    const handle = await open(path, "r");
    const buffer = new Uint8Array(16);

    await handle.read(buffer.subarray(4, 8), 0, 4, 0);
    await handle.close();

    return Array.from(buffer).join(",");
}

export async function syncFstatMatches(path) {
    const fd = openSync(path, "r");
    const same = fstatSync(fd).mode === (await stat(path)).mode;

    closeSync(fd);

    return same;
}

export async function closeTwice(path) {
    const fd = openSync(path, "r");

    closeSync(fd);

    try {
        closeSync(fd);

        return "ok";
    } catch (error) {
        return error.message;
    }
}

export async function busyDescriptor(path) {
    const handle = await open(path, "r");
    const first = handle.read(new Uint8Array(5), 0, 5, 0);

    try {
        await handle.read(new Uint8Array(5), 0, 5, 0);

        await first;
        await handle.close();

        return "ok";
    } catch (error) {
        await first;
        await handle.close();

        return error.message;
    }
}

export async function syncDescriptorOps(path) {
    const fd = openSync(path, "r+");
    const before = fstatSync(fd).size;

    ftruncateSync(fd, 2);
    fsyncSync(fd);
    fdatasyncSync(fd);

    const after = fstatSync(fd).size;

    closeSync(fd);

    return `${before}:${after}`;
}

export async function globalsIdentity() {
    return [
        globalThis.Stats === ImportedStats,
        globalThis.Dirent === ImportedDirent,
        globalThis.FileHandle === ImportedFileHandle,
    ].join(":");
}
"#;

    async fn executor(dir: Dir) -> Executor {
        let module = FsModule::new(dir, HostRuntime::current().expect("host runtime"));

        Executor::builder("fs.ts", FS_SOURCE)
            .workers(1)
            .standard_environment()
            .configure(move |guest| guest.bind(module.clone()))
            .build()
            .await
            .expect("executor builds")
    }

    async fn call<T, A>(executor: &Executor, export: &'static str, args: A) -> T
    where
        A: ToGuestArgs + Send + 'static,
        T: FromGuest<Owned = T> + Send + 'static,
    {
        executor
            .execute(move |context| {
                Box::pin(async move {
                    context
                        .module()
                        .function(export)
                        .await?
                        .call::<_, Promise<T>>(args)
                        .await?
                        .await
                })
            })
            .await
            .expect("guest call succeeds")
    }

    #[tokio::test]
    async fn writes_and_reads_a_file_as_bytes() {
        let fs = Fs::memory();
        let executor = executor(fs.root()).await;

        assert_eq!(
            call::<String, _>(&executor, "writeReadBytes", ("/bytes.bin".to_owned(),)).await,
            "1,2,3",
        );

        executor.shutdown().await.expect("executor shuts down");
    }

    #[tokio::test]
    async fn writes_and_reads_a_file_as_utf8() {
        let fs = Fs::memory();
        let executor = executor(fs.root()).await;

        assert_eq!(
            call::<String, _>(&executor, "writeReadUtf8", ("/notes.txt".to_owned(),)).await,
            "hello",
        );

        executor.shutdown().await.expect("executor shuts down");
    }

    #[tokio::test]
    async fn append_file_extends_an_existing_file() {
        let fs = Fs::memory();
        let executor = executor(fs.root()).await;

        assert_eq!(
            call::<String, _>(&executor, "appendText", ("/notes.txt".to_owned(),)).await,
            "hello world",
        );

        executor.shutdown().await.expect("executor shuts down");
    }

    #[tokio::test]
    async fn read_file_with_hex_and_base64_encodings() {
        let fs = Fs::memory();
        let executor = executor(fs.root()).await;

        assert_eq!(
            call::<String, _>(&executor, "readEncoded", ("/encoded.bin".to_owned(),)).await,
            "68656c6c6f:aGVsbG8=",
        );

        executor.shutdown().await.expect("executor shuts down");
    }

    #[tokio::test]
    async fn mkdir_recursive_creates_a_nested_path() {
        let fs = Fs::memory();
        let executor = executor(fs.root()).await;

        assert_eq!(
            call::<String, _>(&executor, "makeNested", ("/work/nested/path".to_owned(),)).await,
            "/work/nested/path",
        );

        executor.shutdown().await.expect("executor shuts down");
    }

    #[tokio::test]
    async fn mkdtemp_creates_a_unique_directory() {
        let fs = Fs::memory();
        let executor = executor(fs.root()).await;

        assert_eq!(
            call::<String, _>(&executor, "makeTemps", ("/tmp-".to_owned(),)).await,
            "true:true:true",
        );

        executor.shutdown().await.expect("executor shuts down");
    }

    #[tokio::test]
    async fn readdir_returns_sorted_names() {
        let fs = Fs::memory();
        let root = fs.root();

        root.create_dir("/dir").await.unwrap();

        let executor = executor(root.clone()).await;

        root.options().write(true).create(true).open("/dir/b.txt").await.unwrap();
        root.options().write(true).create(true).open("/dir/a.txt").await.unwrap();
        root.options().write(true).create(true).open("/dir/c.txt").await.unwrap();

        assert_eq!(
            call::<String, _>(&executor, "sortedNames", ("/dir".to_owned(),)).await,
            "a.txt,b.txt,c.txt",
        );

        executor.shutdown().await.expect("executor shuts down");
    }

    #[tokio::test]
    async fn readdir_with_file_types_returns_dirent_objects() {
        let fs = Fs::memory();
        let root = fs.root();
        let executor = executor(root.clone()).await;

        root.create_dir("/dir").await.unwrap();
        root.options().write(true).create(true).open("/dir/file.txt").await.unwrap();
        root.create_dir("/dir/folder").await.unwrap();

        assert_eq!(
            call::<String, _>(&executor, "direntSummary", ("/dir".to_owned(),)).await,
            "file.txt:/dir:true:false|folder:/dir:false:true",
        );

        executor.shutdown().await.expect("executor shuts down");
    }

    #[tokio::test]
    async fn readdir_recursive_returns_relative_names() {
        let fs = Fs::memory();
        let root = fs.root();
        let executor = executor(root.clone()).await;

        root.create_dir_all("/dir/sub").await.unwrap();
        root.options().write(true).create(true).open("/dir/a.txt").await.unwrap();
        root.options().write(true).create(true).open("/dir/sub/b.txt").await.unwrap();

        assert_eq!(
            call::<String, _>(&executor, "recursiveNames", ("/dir".to_owned(),)).await,
            "a.txt,sub,sub/b.txt",
        );

        executor.shutdown().await.expect("executor shuts down");
    }

    #[tokio::test]
    async fn stat_follows_a_symlink_and_lstat_does_not() {
        let fs = Fs::memory();
        let root = fs.root();
        let executor = executor(root.clone()).await;

        root.options().write(true).create(true).open("/target.txt").await.unwrap();
        root.symlink("/target.txt", "/link.txt").await.unwrap();

        assert_eq!(
            call::<String, _>(&executor, "statKinds", ("/link.txt".to_owned(),)).await,
            "true:true",
        );

        executor.shutdown().await.expect("executor shuts down");
    }

    #[tokio::test]
    async fn stat_reports_dates_as_date_objects() {
        let fs = Fs::memory();
        let root = fs.root();
        let executor = executor(root.clone()).await;

        root.options().write(true).create(true).open("/file.txt").await.unwrap();

        assert!(call::<bool, _>(&executor, "statDates", ("/file.txt".to_owned(),)).await);

        executor.shutdown().await.expect("executor shuts down");
    }

    #[tokio::test]
    async fn stat_mode_carries_the_file_type_bits() {
        let fs = Fs::memory();
        let root = fs.root();
        let executor = executor(root.clone()).await;

        root.options().write(true).create(true).open("/file.txt").await.unwrap();

        assert!(call::<bool, _>(&executor, "statModeBits", ("/file.txt".to_owned(),)).await);

        executor.shutdown().await.expect("executor shuts down");
    }

    #[tokio::test]
    async fn chmod_changes_the_reported_mode() {
        let fs = Fs::memory();
        let root = fs.root();
        let executor = executor(root.clone()).await;

        root.options().write(true).create(true).open("/file.txt").await.unwrap();

        assert_eq!(
            call::<u32, _>(&executor, "chmodMode", ("/file.txt".to_owned(),)).await,
            0o600_u32,
        );

        executor.shutdown().await.expect("executor shuts down");
    }

    #[tokio::test]
    async fn rm_force_ignores_a_missing_path() {
        let fs = Fs::memory();
        let executor = executor(fs.root()).await;

        assert_eq!(
            call::<String, _>(&executor, "rmForce", ("/missing.txt".to_owned(),)).await,
            "ok",
        );

        executor.shutdown().await.expect("executor shuts down");
    }

    #[tokio::test]
    async fn rm_recursive_removes_a_populated_directory() {
        let fs = Fs::memory();
        let root = fs.root();
        let executor = executor(root.clone()).await;

        root.create_dir_all("/dir/sub").await.unwrap();
        root.options().write(true).create(true).open("/dir/sub/file.txt").await.unwrap();

        assert_eq!(
            call::<String, _>(&executor, "rmRecursive", ("/dir".to_owned(),)).await,
            "gone",
        );

        executor.shutdown().await.expect("executor shuts down");
    }

    #[tokio::test]
    async fn rename_moves_a_file() {
        let fs = Fs::memory();
        let root = fs.root();
        let executor = executor(root.clone()).await;

        root.options().write(true).create(true).open("/from.txt").await.unwrap().write_all(b"hello").await.unwrap();

        assert_eq!(
            call::<String, _>(&executor, "renameRead", ("/from.txt".to_owned(), "/to.txt".to_owned())).await,
            "hello",
        );

        executor.shutdown().await.expect("executor shuts down");
    }

    #[tokio::test]
    async fn copy_file_exclusive_refuses_an_existing_destination() {
        let fs = Fs::memory();
        let root = fs.root();
        let executor = executor(root.clone()).await;

        root.options().write(true).create(true).open("/from.txt").await.unwrap().write_all(b"a").await.unwrap();
        root.options().write(true).create(true).open("/to.txt").await.unwrap().write_all(b"b").await.unwrap();

        assert!(
            call::<String, _>(&executor, "copyExclusive", ("/from.txt".to_owned(), "/to.txt".to_owned())).await
                .contains("agentc:fs:"),
        );

        executor.shutdown().await.expect("executor shuts down");
    }

    #[tokio::test]
    async fn truncate_shortens_a_file() {
        let fs = Fs::memory();
        let root = fs.root();
        let executor = executor(root.clone()).await;

        root.options().write(true).create(true).open("/file.txt").await.unwrap().write_all(b"hello").await.unwrap();

        assert_eq!(
            call::<String, _>(&executor, "truncateRead", ("/file.txt".to_owned(), 2_u64)).await,
            "he",
        );

        executor.shutdown().await.expect("executor shuts down");
    }

    #[tokio::test]
    async fn readlink_returns_the_stored_target() {
        let fs = Fs::memory();
        let root = fs.root();
        let executor = executor(root.clone()).await;

        root.options().write(true).create(true).open("/target.txt").await.unwrap();
        root.symlink("/target.txt", "/link.txt").await.unwrap();

        assert_eq!(
            call::<String, _>(&executor, "linkTarget", ("/link.txt".to_owned(),)).await,
            "/target.txt",
        );

        executor.shutdown().await.expect("executor shuts down");
    }

    #[tokio::test]
    async fn relative_paths_resolve_against_the_bound_directory() {
        let fs = Fs::memory();
        let root = fs.root();

        root.create_dir("/work").await.unwrap();

        let executor = executor(root.open_dir("/work").await.unwrap()).await;

        assert_eq!(
            call::<String, _>(&executor, "writeText", ("data.json".to_owned(), "hello".to_owned())).await,
            "ok",
        );
        assert_eq!(
            root.open_file("/work/data.json").await.unwrap().read_to_string().await.unwrap(),
            "hello",
        );

        executor.shutdown().await.expect("executor shuts down");
    }

    #[tokio::test]
    async fn absolute_paths_resolve_from_the_root() {
        let fs = Fs::memory();
        let root = fs.root();

        root.create_dir("/work").await.unwrap();

        let executor = executor(root.open_dir("/work").await.unwrap()).await;

        assert_eq!(
            call::<String, _>(&executor, "writeText", ("/other.txt".to_owned(), "hello".to_owned())).await,
            "ok",
        );
        assert_eq!(
            root.open_file("/other.txt").await.unwrap().read_to_string().await.unwrap(),
            "hello",
        );

        executor.shutdown().await.expect("executor shuts down");
    }

    #[tokio::test]
    async fn a_missing_path_surfaces_as_a_guest_exception() {
        let fs = Fs::memory();
        let executor = executor(fs.root()).await;

        assert!(
            call::<String, _>(&executor, "missingMessage", ("/missing.txt".to_owned(),)).await
                .contains("agentc:fs:"),
        );

        executor.shutdown().await.expect("executor shuts down");
    }

    #[tokio::test]
    async fn the_synchronous_functions_return_values_rather_than_promises() {
        let fs = Fs::memory();
        let root = fs.root();
        let executor = executor(root.clone()).await;

        root.options().write(true).create(true).open("/file.txt").await.unwrap().write_all(b"hello").await.unwrap();

        assert!(call::<bool, _>(&executor, "syncSurface", ("/file.txt".to_owned(),)).await);

        executor.shutdown().await.expect("executor shuts down");
    }

    #[tokio::test]
    async fn open_and_read_fills_the_callers_buffer() {
        let fs = Fs::memory();
        let root = fs.root();
        let executor = executor(root.clone()).await;

        root.options().write(true).create(true).open("/file.txt").await.unwrap().write_all(b"hello").await.unwrap();

        assert_eq!(
            call::<String, _>(&executor, "openReadInto", ("/file.txt".to_owned(),)).await,
            "5:104,101,108,108,111",
        );

        executor.shutdown().await.expect("executor shuts down");
    }

    #[tokio::test]
    async fn open_and_read_returns_the_same_buffer_object() {
        let fs = Fs::memory();
        let root = fs.root();
        let executor = executor(root.clone()).await;

        root.options().write(true).create(true).open("/file.txt").await.unwrap().write_all(b"hello").await.unwrap();

        assert!(call::<bool, _>(&executor, "identity", ("/file.txt".to_owned(),)).await);

        executor.shutdown().await.expect("executor shuts down");
    }

    #[tokio::test]
    async fn read_without_a_buffer_allocates_one() {
        let fs = Fs::memory();
        let root = fs.root();
        let executor = executor(root.clone()).await;

        root.options().write(true).create(true).open("/file.txt").await.unwrap().write_all(b"hello").await.unwrap();

        assert_eq!(
            call::<String, _>(&executor, "readWithoutBuffer", ("/file.txt".to_owned(),)).await,
            "5:104,101,108,108,111",
        );

        executor.shutdown().await.expect("executor shuts down");
    }

    #[tokio::test]
    async fn read_with_a_position_does_not_move_the_cursor() {
        let fs = Fs::memory();
        let root = fs.root();
        let executor = executor(root.clone()).await;

        root.options().write(true).create(true).open("/file.txt").await.unwrap().write_all(b"hello").await.unwrap();

        assert_eq!(
            call::<String, _>(&executor, "readAtPosition", ("/file.txt".to_owned(),)).await,
            "101,108:104,101",
        );

        executor.shutdown().await.expect("executor shuts down");
    }

    #[tokio::test]
    async fn write_through_a_handle_appends_at_the_cursor() {
        let fs = Fs::memory();
        let root = fs.root();
        let executor = executor(root.clone()).await;

        root.options().write(true).create(true).open("/file.txt").await.unwrap().write_all(b"hello").await.unwrap();

        assert_eq!(
            call::<String, _>(&executor, "writeThroughHandle", ("/file.txt".to_owned(),)).await,
            "hello world",
        );

        executor.shutdown().await.expect("executor shuts down");
    }

    #[tokio::test]
    async fn read_sync_fills_the_callers_buffer() {
        let fs = Fs::memory();
        let root = fs.root();
        let executor = executor(root.clone()).await;

        root.options().write(true).create(true).open("/file.txt").await.unwrap().write_all(b"hello").await.unwrap();

        assert_eq!(
            call::<String, _>(&executor, "syncReadInto", ("/file.txt".to_owned(),)).await,
            "5:104,101,108,108,111",
        );

        executor.shutdown().await.expect("executor shuts down");
    }

    #[tokio::test]
    async fn write_sync_consumes_the_callers_buffer() {
        let fs = Fs::memory();
        let root = fs.root();
        let executor = executor(root.clone()).await;

        assert_eq!(
            call::<String, _>(&executor, "syncWriteFrom", ("/file.txt".to_owned(),)).await,
            "5:hello",
        );

        executor.shutdown().await.expect("executor shuts down");
    }

    #[tokio::test]
    async fn a_subarray_view_is_written_at_its_own_offset() {
        let fs = Fs::memory();
        let root = fs.root();
        let executor = executor(root.clone()).await;

        root.options().write(true).create(true).open("/file.txt").await.unwrap().write_all(b"rust").await.unwrap();

        assert_eq!(
            call::<String, _>(&executor, "readSubarray", ("/file.txt".to_owned(),)).await,
            "0,0,0,0,114,117,115,116,0,0,0,0,0,0,0,0",
        );

        executor.shutdown().await.expect("executor shuts down");
    }

    #[tokio::test]
    async fn fstat_sync_matches_stat_on_the_same_path() {
        let fs = Fs::memory();
        let root = fs.root();
        let executor = executor(root.clone()).await;

        root.options().write(true).create(true).open("/file.txt").await.unwrap().write_all(b"hello").await.unwrap();

        assert!(call::<bool, _>(&executor, "syncFstatMatches", ("/file.txt".to_owned(),)).await);

        executor.shutdown().await.expect("executor shuts down");
    }

    #[tokio::test]
    async fn closing_twice_reports_a_bad_descriptor() {
        let fs = Fs::memory();
        let root = fs.root();
        let executor = executor(root.clone()).await;

        root.options().write(true).create(true).open("/file.txt").await.unwrap();

        assert!(
            call::<String, _>(&executor, "closeTwice", ("/file.txt".to_owned(),)).await
                .contains("bad file descriptor")
        );

        executor.shutdown().await.expect("executor shuts down");
    }

    #[tokio::test]
    async fn a_second_operation_on_a_busy_descriptor_reports_it() {
        let fs = Fs::memory();
        let root = fs.root();
        let executor = executor(root.clone()).await;

        root.options().write(true).create(true).open("/file.txt").await.unwrap().write_all(b"hello").await.unwrap();

        assert!(
            call::<String, _>(&executor, "busyDescriptor", ("/file.txt".to_owned(),)).await
                .contains("is busy")
        );

        executor.shutdown().await.expect("executor shuts down");
    }

    #[tokio::test]
    async fn sync_descriptor_operations_work() {
        let fs = Fs::memory();
        let root = fs.root();
        let executor = executor(root.clone()).await;

        root.options().write(true).create(true).open("/file.txt").await.unwrap().write_all(b"hello").await.unwrap();

        assert_eq!(
            call::<String, _>(&executor, "syncDescriptorOps", ("/file.txt".to_owned(),)).await,
            "5:2",
        );

        executor.shutdown().await.expect("executor shuts down");
    }

    #[tokio::test]
    async fn the_classes_are_installed_as_globals() {
        let fs = Fs::memory();
        let executor = executor(fs.root()).await;

        assert_eq!(
            call::<String, _>(&executor, "globalsIdentity", ()).await,
            "true:true:true",
        );

        executor.shutdown().await.expect("executor shuts down");
    }
}
