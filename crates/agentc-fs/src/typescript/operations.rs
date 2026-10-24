// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_executor_typescript::{
    backend::ExecutorBackend,
    guestjs::{
        enter::Enter,
        errors::Error,
        marshal::{ToGuest, args::Args},
    },
};
use bytes::Bytes;

use crate::{
    fs::{Dir, File, FileType, Owner, Permissions, SetOwnerOptions},
    path::PathBuf,
    typescript::{
        dirent::Dirent,
        options::{
            AccessMode, CopyMode, EncodingOrOptions, FileOptions, MkdirOptions, OpenFlags,
            OpenFlagsArg, ReaddirOptions, RmOptions, RmdirOptions,
        },
        stats::Stats,
        types::{FileContents, FileData, ReaddirResult},
    },
};

pub(crate) trait FsOperation<B: ExecutorBackend>: Sized + 'static {
    const NAME: &'static str;

    type Output: ToGuest<B> + Send + 'static;

    fn read<'js>(enter: &Enter<'js, B>, args: &Args<'js, B>) -> Result<Self, Error>;

    fn run(self, dir: Dir) -> impl Future<Output = Result<Self::Output, Error>> + Send;
}

pub(crate) struct Access {
    path: String,
    mode: Option<i32>,
}

impl<B: ExecutorBackend> FsOperation<B> for Access {
    const NAME: &'static str = "access";

    type Output = ();

    fn read<'js>(enter: &Enter<'js, B>, args: &Args<'js, B>) -> Result<Self, Error> {
        Ok(Self {
            path: args.next::<String>(enter)?,
            mode: args.next::<Option<i32>>(enter)?,
        })
    }

    async fn run(self, dir: Dir) -> Result<Self::Output, Error> {
        dir.access(self.path, &AccessMode::options(self.mode.unwrap_or_default()))
            .await?;

        Ok(())
    }
}

pub(crate) struct ReadFile {
    path: String,
    options: FileOptions,
}

impl<B: ExecutorBackend> FsOperation<B> for ReadFile {
    const NAME: &'static str = "readFile";

    type Output = FileContents;

    fn read<'js>(enter: &Enter<'js, B>, args: &Args<'js, B>) -> Result<Self, Error> {
        Ok(Self {
            path: args.next::<String>(enter)?,
            options: args
                .next::<Option<EncodingOrOptions>>(enter)?
                .map(FileOptions::from)
                .unwrap_or_default(),
        })
    }

    async fn run(self, dir: Dir) -> Result<Self::Output, Error> {
        let flags = self.options.open_flags("r")?;
        let mut file = dir
            .open_with_options(self.path, flags.options())
            .await?;

        let bytes = file.read_to_end().await?;

        match self.options.encoding()? {
            Some(encoding) => Ok(FileContents::Text(encoding.decode(&bytes))),
            None => Ok(FileContents::Bytes(Bytes::from(bytes))),
        }
    }
}

pub(crate) struct WriteFile {
    path: String,
    data: FileData,
    options: FileOptions,
}

impl<B: ExecutorBackend> FsOperation<B> for WriteFile {
    const NAME: &'static str = "writeFile";

    type Output = ();

    fn read<'js>(enter: &Enter<'js, B>, args: &Args<'js, B>) -> Result<Self, Error> {
        Ok(Self {
            path: args.next::<String>(enter)?,
            data: args.next::<FileData>(enter)?,
            options: args
                .next::<Option<EncodingOrOptions>>(enter)?
                .map(FileOptions::from)
                .unwrap_or_default(),
        })
    }

    async fn run(self, dir: Dir) -> Result<Self::Output, Error> {
        let flags = self.options.open_flags("w")?;
        let mut file = dir
            .open_with_options(self.path.clone(), flags.options())
            .await?;

        let bytes = self
            .data
            .into_bytes(self.options.encoding()?)?;
        file.write_all(bytes).await?;

        if let Some(mode) = self.options.mode
            && flags.creates()
        {
            dir.set_permissions(self.path, Permissions::new(mode))
                .await?;
        }

        Ok(())
    }
}

pub(crate) struct AppendFile {
    path: String,
    data: FileData,
    options: FileOptions,
}

impl<B: ExecutorBackend> FsOperation<B> for AppendFile {
    const NAME: &'static str = "appendFile";

    type Output = ();

    fn read<'js>(enter: &Enter<'js, B>, args: &Args<'js, B>) -> Result<Self, Error> {
        Ok(Self {
            path: args.next::<String>(enter)?,
            data: args.next::<FileData>(enter)?,
            options: args
                .next::<Option<EncodingOrOptions>>(enter)?
                .map(FileOptions::from)
                .unwrap_or_default(),
        })
    }

    async fn run(self, dir: Dir) -> Result<Self::Output, Error> {
        let flags = self.options.open_flags("a")?;
        let mut file = dir
            .open_with_options(self.path.clone(), flags.options())
            .await?;

        let bytes = self
            .data
            .into_bytes(self.options.encoding()?)?;
        file.write_all(bytes).await?;

        if let Some(mode) = self.options.mode
            && flags.creates()
        {
            dir.set_permissions(self.path, Permissions::new(mode))
                .await?;
        }

        Ok(())
    }
}

pub(crate) struct Rename {
    from: String,
    to: String,
}

impl<B: ExecutorBackend> FsOperation<B> for Rename {
    const NAME: &'static str = "rename";

    type Output = ();

    fn read<'js>(enter: &Enter<'js, B>, args: &Args<'js, B>) -> Result<Self, Error> {
        Ok(Self {
            from: args.next::<String>(enter)?,
            to: args.next::<String>(enter)?,
        })
    }

    async fn run(self, dir: Dir) -> Result<Self::Output, Error> {
        dir.rename(self.from, self.to).await?;

        Ok(())
    }
}

pub(crate) struct Readdir {
    path: String,
    options: ReaddirOptions,
}

impl Readdir {
    fn relative_name(base: &[u8], path: &[u8]) -> String {
        String::from_utf8_lossy(if base == b"/" {
            &path[1..]
        } else {
            &path[base.len() + 1..]
        })
        .into_owned()
    }
}

impl<B: ExecutorBackend> FsOperation<B> for Readdir {
    const NAME: &'static str = "readdir";

    type Output = ReaddirResult;

    fn read<'js>(enter: &Enter<'js, B>, args: &Args<'js, B>) -> Result<Self, Error> {
        Ok(Self {
            path: args.next::<String>(enter)?,
            options: args
                .next::<Option<ReaddirOptions>>(enter)?
                .unwrap_or_default(),
        })
    }

    async fn run(self, dir: Dir) -> Result<Self::Output, Error> {
        let dir = dir.open_dir(&self.path).await?;
        let mut entries = Vec::new();

        if self.options.recursive.unwrap_or(false) {
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

        entries.sort_by(|left, right| {
            left.path()
                .as_bytes()
                .cmp(right.path().as_bytes())
        });

        if self
            .options
            .with_file_types
            .unwrap_or(false)
        {
            return Ok(ReaddirResult::Entries(
                entries
                    .iter()
                    .map(Dirent::new)
                    .collect(),
            ));
        }

        if self.options.recursive.unwrap_or(false) {
            return Ok(ReaddirResult::Names(
                entries
                    .into_iter()
                    .map(|entry| {
                        Self::relative_name(
                            &dir.path().as_bytes().to_vec(),
                            entry.path().as_bytes(),
                        )
                    })
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
}

pub(crate) struct Mkdir {
    path: String,
    options: MkdirOptions,
}

impl<B: ExecutorBackend> FsOperation<B> for Mkdir {
    const NAME: &'static str = "mkdir";

    type Output = String;

    fn read<'js>(enter: &Enter<'js, B>, args: &Args<'js, B>) -> Result<Self, Error> {
        Ok(Self {
            path: args.next::<String>(enter)?,
            options: args
                .next::<Option<MkdirOptions>>(enter)?
                .unwrap_or_default(),
        })
    }

    async fn run(self, dir: Dir) -> Result<Self::Output, Error> {
        let (created_dir, created) = if self.options.recursive.unwrap_or(false) {
            dir.create_dir_all(self.path).await?
        } else {
            dir.create_dir(self.path).await?
        };

        if let Some(mode) = self.options.mode
            && created
        {
            dir.set_permissions(created_dir.path(), Permissions::new(mode))
                .await?;
        }

        Ok(created_dir.path().to_string_lossy())
    }
}

pub(crate) struct Mkdtemp {
    prefix: String,
}

impl<B: ExecutorBackend> FsOperation<B> for Mkdtemp {
    const NAME: &'static str = "mkdtemp";

    type Output = String;

    fn read<'js>(enter: &Enter<'js, B>, args: &Args<'js, B>) -> Result<Self, Error> {
        Ok(Self { prefix: args.next::<String>(enter)? })
    }

    async fn run(self, dir: Dir) -> Result<Self::Output, Error> {
        Ok(dir
            .create_dir_temp(self.prefix)
            .await?
            .path()
            .to_string_lossy())
    }
}

pub(crate) struct Rm {
    path: String,
    options: RmOptions,
}

impl<B: ExecutorBackend> FsOperation<B> for Rm {
    const NAME: &'static str = "rm";

    type Output = ();

    fn read<'js>(enter: &Enter<'js, B>, args: &Args<'js, B>) -> Result<Self, Error> {
        Ok(Self {
            path: args.next::<String>(enter)?,
            options: args
                .next::<Option<RmOptions>>(enter)?
                .unwrap_or_default(),
        })
    }

    async fn run(self, dir: Dir) -> Result<Self::Output, Error> {
        let recursive = self.options.recursive.unwrap_or(false);
        let force = self.options.force.unwrap_or(false);

        match dir.symlink_metadata(&self.path).await {
            Ok(metadata) if metadata.file_type() == FileType::Directory => {
                let result = if recursive {
                    dir.remove_dir_all(self.path).await
                } else {
                    dir.remove_dir(self.path).await
                };

                if force {
                    Ok(())
                } else {
                    result.map_err(Into::into)
                }
            }
            Ok(_) => {
                let result = dir.remove_file(self.path).await;

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
}

pub(crate) struct Rmdir {
    path: String,
    options: RmdirOptions,
}

impl<B: ExecutorBackend> FsOperation<B> for Rmdir {
    const NAME: &'static str = "rmdir";

    type Output = ();

    fn read<'js>(enter: &Enter<'js, B>, args: &Args<'js, B>) -> Result<Self, Error> {
        Ok(Self {
            path: args.next::<String>(enter)?,
            options: args
                .next::<Option<RmdirOptions>>(enter)?
                .unwrap_or_default(),
        })
    }

    async fn run(self, dir: Dir) -> Result<Self::Output, Error> {
        if self.options.recursive.unwrap_or(false) {
            dir.remove_dir_all(self.path).await?;
        } else {
            dir.remove_dir(self.path).await?;
        }

        Ok(())
    }
}

pub(crate) struct Unlink {
    path: String,
}

impl<B: ExecutorBackend> FsOperation<B> for Unlink {
    const NAME: &'static str = "unlink";

    type Output = ();

    fn read<'js>(enter: &Enter<'js, B>, args: &Args<'js, B>) -> Result<Self, Error> {
        Ok(Self { path: args.next::<String>(enter)? })
    }

    async fn run(self, dir: Dir) -> Result<Self::Output, Error> {
        dir.remove_file(self.path).await?;

        Ok(())
    }
}

pub(crate) struct Stat {
    path: String,
}

impl<B: ExecutorBackend> FsOperation<B> for Stat {
    const NAME: &'static str = "stat";

    type Output = Stats<B>;

    fn read<'js>(enter: &Enter<'js, B>, args: &Args<'js, B>) -> Result<Self, Error> {
        Ok(Self { path: args.next::<String>(enter)? })
    }

    async fn run(self, dir: Dir) -> Result<Self::Output, Error> {
        Ok(Stats::new(dir.metadata(self.path).await?))
    }
}

pub(crate) struct Lstat {
    path: String,
}

impl<B: ExecutorBackend> FsOperation<B> for Lstat {
    const NAME: &'static str = "lstat";

    type Output = Stats<B>;

    fn read<'js>(enter: &Enter<'js, B>, args: &Args<'js, B>) -> Result<Self, Error> {
        Ok(Self { path: args.next::<String>(enter)? })
    }

    async fn run(self, dir: Dir) -> Result<Self::Output, Error> {
        Ok(Stats::new(dir.symlink_metadata(self.path).await?))
    }
}

pub(crate) struct Chmod {
    path: String,
    mode: u32,
}

impl<B: ExecutorBackend> FsOperation<B> for Chmod {
    const NAME: &'static str = "chmod";

    type Output = ();

    fn read<'js>(enter: &Enter<'js, B>, args: &Args<'js, B>) -> Result<Self, Error> {
        Ok(Self {
            path: args.next::<String>(enter)?,
            mode: args.next::<u32>(enter)?,
        })
    }

    async fn run(self, dir: Dir) -> Result<Self::Output, Error> {
        dir.set_permissions(self.path, Permissions::new(self.mode))
            .await?;

        Ok(())
    }
}

pub(crate) struct Chown {
    path: String,
    uid: u32,
    gid: u32,
}

impl<B: ExecutorBackend> FsOperation<B> for Chown {
    const NAME: &'static str = "chown";

    type Output = ();

    fn read<'js>(enter: &Enter<'js, B>, args: &Args<'js, B>) -> Result<Self, Error> {
        Ok(Self {
            path: args.next::<String>(enter)?,
            uid: args.next::<u32>(enter)?,
            gid: args.next::<u32>(enter)?,
        })
    }

    async fn run(self, dir: Dir) -> Result<Self::Output, Error> {
        dir.set_owner(
            self.path,
            Owner::new()
                .user(self.uid)
                .group(self.gid),
        )
        .await?;

        Ok(())
    }
}

pub(crate) struct Lchown {
    path: String,
    uid: u32,
    gid: u32,
}

impl<B: ExecutorBackend> FsOperation<B> for Lchown {
    const NAME: &'static str = "lchown";

    type Output = ();

    fn read<'js>(enter: &Enter<'js, B>, args: &Args<'js, B>) -> Result<Self, Error> {
        Ok(Self {
            path: args.next::<String>(enter)?,
            uid: args.next::<u32>(enter)?,
            gid: args.next::<u32>(enter)?,
        })
    }

    async fn run(self, dir: Dir) -> Result<Self::Output, Error> {
        dir.set_owner_with_options(
            self.path,
            Owner::new()
                .user(self.uid)
                .group(self.gid),
            &SetOwnerOptions::new().follow_symlinks(false),
        )
        .await?;

        Ok(())
    }
}

pub(crate) struct Symlink {
    target: String,
    path: String,
}

impl<B: ExecutorBackend> FsOperation<B> for Symlink {
    const NAME: &'static str = "symlink";

    type Output = ();

    fn read<'js>(enter: &Enter<'js, B>, args: &Args<'js, B>) -> Result<Self, Error> {
        Ok(Self {
            target: args.next::<String>(enter)?,
            path: args.next::<String>(enter)?,
        })
    }

    async fn run(self, dir: Dir) -> Result<Self::Output, Error> {
        dir.symlink(self.target, self.path)
            .await?;

        Ok(())
    }
}

pub(crate) struct Readlink {
    path: String,
}

impl<B: ExecutorBackend> FsOperation<B> for Readlink {
    const NAME: &'static str = "readlink";

    type Output = String;

    fn read<'js>(enter: &Enter<'js, B>, args: &Args<'js, B>) -> Result<Self, Error> {
        Ok(Self { path: args.next::<String>(enter)? })
    }

    async fn run(self, dir: Dir) -> Result<Self::Output, Error> {
        Ok(dir
            .read_link(self.path)
            .await?
            .to_string_lossy())
    }
}

pub(crate) struct Truncate {
    path: String,
    len: Option<u64>,
}

impl<B: ExecutorBackend> FsOperation<B> for Truncate {
    const NAME: &'static str = "truncate";

    type Output = ();

    fn read<'js>(enter: &Enter<'js, B>, args: &Args<'js, B>) -> Result<Self, Error> {
        Ok(Self {
            path: args.next::<String>(enter)?,
            len: args.next::<Option<u64>>(enter)?,
        })
    }

    async fn run(self, dir: Dir) -> Result<Self::Output, Error> {
        dir.truncate(self.path, self.len.unwrap_or_default())
            .await?;

        Ok(())
    }
}

pub(crate) struct CopyFile {
    from: String,
    to: String,
    mode: Option<i32>,
}

impl<B: ExecutorBackend> FsOperation<B> for CopyFile {
    const NAME: &'static str = "copyFile";

    type Output = ();

    fn read<'js>(enter: &Enter<'js, B>, args: &Args<'js, B>) -> Result<Self, Error> {
        Ok(Self {
            from: args.next::<String>(enter)?,
            to: args.next::<String>(enter)?,
            mode: args.next::<Option<i32>>(enter)?,
        })
    }

    async fn run(self, dir: Dir) -> Result<Self::Output, Error> {
        let exclusive = CopyMode::exclusive(self.mode.unwrap_or_default())?;
        let mut source = dir
            .options()
            .read(true)
            .open(self.from)
            .await?;
        let bytes = source.read_to_end().await?;
        let mut destination = if exclusive {
            dir.options()
                .write(true)
                .create(true)
                .create_new(true)
                .open(self.to)
                .await?
        } else {
            dir.options()
                .write(true)
                .create(true)
                .truncate(true)
                .open(self.to)
                .await?
        };

        destination.write_all(bytes).await?;

        Ok(())
    }
}

pub(crate) struct Open {
    path: String,
    flags: OpenFlags,
    mode: Option<u32>,
}

impl Open {
    pub(crate) fn read<'js, B: ExecutorBackend>(
        enter: &Enter<'js, B>,
        args: &Args<'js, B>,
    ) -> Result<Self, Error> {
        Ok(Self {
            path: args.next::<String>(enter)?,
            flags: match args.next::<Option<OpenFlagsArg>>(enter)? {
                Some(flags) => OpenFlags::try_from(flags)?,
                None => OpenFlags::parse("r")?,
            },
            mode: args.next::<Option<u32>>(enter)?,
        })
    }

    pub(crate) async fn run(self, dir: Dir) -> Result<(PathBuf, File), Error> {
        let resolved = dir.resolve(&self.path)?;
        let file = dir
            .open_with_options(self.path, self.flags.options())
            .await?;

        if let Some(mode) = self.mode {
            if self.flags.creates() {
                dir.set_permissions(resolved.clone(), Permissions::new(mode))
                    .await?;
            }
        }

        Ok((resolved, file))
    }
}
