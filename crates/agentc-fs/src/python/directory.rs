// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{future::Future, marker::PhantomData};

use agentc_executor_python::{
    backend::ExecutorBackend,
    guestpy::{
        errors::Error,
        host::{exception::Raise, iter::HostStream, state::ModuleState},
        host_class,
    },
};
use bytes::Bytes;
use futures::{stream, TryStreamExt};

use crate::{
    fs::{AccessOptions, Dir, OpenOptions, Owner, Permissions, SetOwnerOptions},
    python::{
        entry::Entry,
        file::File,
        module::FsModule,
        path::StrPath,
        stat::Stat,
    },
};

pub struct Directory<B: ExecutorBackend> {
    dir: Dir,
    backend: PhantomData<B>,
}

#[host_class(backend = B, crate_path = agentc_executor_python::guestpy)]
impl<B: ExecutorBackend> Directory<B> {
    #[guestpy(static_method)]
    fn root(#[guestpy(context)] module: ModuleState<FsModule>) -> Result<Self, Error> {
        Ok(Self::from(module.dir().clone()))
    }

    #[guestpy(get)]
    fn path(&self) -> Result<String, Error> {
        Ok(self.dir.path().to_string_lossy())
    }

    #[guestpy(get)]
    fn authority_root(&self) -> Result<String, Error> {
        Ok(self.dir.authority_root().to_string_lossy())
    }

    #[guestpy(async_method)]
    fn open(
        &self,
        #[guestpy(positional)] path: StrPath,
        #[guestpy(kw)] read: Option<bool>,
        #[guestpy(kw)] write: Option<bool>,
        #[guestpy(kw)] append: Option<bool>,
        #[guestpy(kw)] truncate: Option<bool>,
        #[guestpy(kw)] create: Option<bool>,
        #[guestpy(kw)] create_new: Option<bool>,
        #[guestpy(kw)] follow_symlinks: Option<bool>,
    ) -> Result<impl Future<Output = Result<File<B>, Error>> + use<B>, Error> {
        let dir = self.dir.clone();

        Ok(async move {
            Ok(
                File::<B>::from(
                    dir.open_with_options(
                        path.into_inner(),
                        &OpenOptions::new()
                            .read(read.unwrap_or(true))
                            .write(write.unwrap_or(false))
                            .append(append.unwrap_or(false))
                            .truncate(truncate.unwrap_or(false))
                            .create(create.unwrap_or(false))
                            .create_new(create_new.unwrap_or(false))
                            .follow_symlinks(follow_symlinks.unwrap_or(true)),
                    )
                    .await
                    .map_err(Raise::<B>::from)?
                )
            )
        })
    }

    #[guestpy(async_method)]
    fn read_bytes(
        &self,
        #[guestpy(positional)] path: StrPath,
    ) -> Result<impl Future<Output = Result<Bytes, Error>> + use<B>, Error> {
        let dir = self.dir.clone();

        Ok(async move {
            Ok(
                Bytes::from(
                    dir.open_file(path.into_inner())
                        .await
                        .map_err(Raise::<B>::from)?
                        .read_to_end()
                        .await
                        .map_err(Raise::<B>::from)?
                )
            )
        })
    }

    #[guestpy(async_method)]
    fn read_text(
        &self,
        #[guestpy(positional)] path: StrPath,
    ) -> Result<impl Future<Output = Result<String, Error>> + use<B>, Error> {
        let dir = self.dir.clone();

        Ok(async move {
            Ok(
                dir.open_file(path.into_inner())
                    .await
                    .map_err(Raise::<B>::from)?
                    .read_to_string()
                    .await
                    .map_err(Raise::<B>::from)?
            )
        })
    }

    #[guestpy(async_method)]
    fn write_bytes(
        &self,
        #[guestpy(positional)] path: StrPath,
        #[guestpy(positional)] data: Bytes,
    ) -> Result<impl Future<Output = Result<(), Error>> + use<B>, Error> {
        let dir = self.dir.clone();

        Ok(async move {
            let mut file = dir
                .open_with_options(
                    path.into_inner(),
                    &OpenOptions::new()
                        .write(true)
                        .create(true)
                        .truncate(true),
                )
                .await
                .map_err(Raise::<B>::from)?;

            file.write_all(data)
                .await
                .map_err(Raise::<B>::from)?;

            file.flush()
                .await
                .map_err(Raise::<B>::from)?;

            Ok(())
        })
    }

    #[guestpy(async_method)]
    fn write_text(
        &self,
        #[guestpy(positional)] path: StrPath,
        #[guestpy(positional)] data: String,
    ) -> Result<impl Future<Output = Result<(), Error>> + use<B>, Error> {
        let dir = self.dir.clone();

        Ok(async move {
            let mut file = dir
                .open_with_options(
                    path.into_inner(),
                    &OpenOptions::new()
                        .write(true)
                        .create(true)
                        .truncate(true),
                )
                .await
                .map_err(Raise::<B>::from)?;

            file.write_all(data)
                .await
                .map_err(Raise::<B>::from)?;

            file.flush()
                .await
                .map_err(Raise::<B>::from)?;

            Ok(())
        })
    }

    #[guestpy(async_method)]
    fn truncate(
        &self,
        #[guestpy(positional)] path: StrPath,
        #[guestpy(positional)] length: u64,
    ) -> Result<impl Future<Output = Result<(), Error>> + use<B>, Error> {
        let dir = self.dir.clone();

        Ok(async move {
            Ok(
                dir.truncate(path.into_inner(), length)
                    .await
                    .map_err(Raise::<B>::from)?
            )
        })
    }

    #[guestpy(async_method)]
    fn open_dir(
        &self,
        #[guestpy(positional)] path: StrPath,
    ) -> Result<impl Future<Output = Result<Self, Error>> + use<B>, Error> {
        let dir = self.dir.clone();

        Ok(async move {
            Ok(Self::from(
                dir.open_dir(path.into_inner())
                    .await
                    .map_err(Raise::<B>::from)?,
            ))
        })
    }

    #[guestpy(async_method)]
    fn mkdir(
        &self,
        #[guestpy(positional)] path: StrPath,
        #[guestpy(kw)] parents: Option<bool>,
        #[guestpy(kw)] exist_ok: Option<bool>,
    ) -> Result<impl Future<Output = Result<Self, Error>> + use<B>, Error> {
        let dir = self.dir.clone();

        Ok(async move {
            let path = path.into_inner();

            Ok(
                Self::from(
                    match (parents.unwrap_or(false), exist_ok.unwrap_or(false)) {
                        (false, false) => dir.create_dir(path).await.map(|(dir, _)| dir),
                        (false, true) => match dir.create_dir(&path).await {
                            Err(error) if error.is_already_exists() => dir.open_dir(path).await,
                            result => result.map(|(dir, _)| dir),
                        },
                        (true, false) => match dir.create_dir(&path).await {
                            Err(error) if error.is_not_found() => {
                                dir.create_dir_all(path).await.map(|(dir, _)| dir)
                            }
                            result => result.map(|(dir, _)| dir),
                        },
                        (true, true) => dir.create_dir_all(path).await.map(|(dir, _)| dir),
                    }
                    .map_err(Raise::<B>::from)?
                )
            )
        })
    }

    #[guestpy(async_method)]
    fn mkdtemp(
        &self,
        #[guestpy(kw)] prefix: StrPath,
    ) -> Result<impl Future<Output = Result<Self, Error>> + use<B>, Error> {
        let dir = self.dir.clone();

        Ok(async move {
            Ok(Self::from(
                dir.create_dir_temp(prefix.into_inner())
                    .await
                    .map_err(Raise::<B>::from)?,
            ))
        })
    }

    #[guestpy(method)]
    fn scandir(&self) -> Result<HostStream<Entry<B>>, Error> {
        let dir = self.dir.clone();

        Ok(HostStream::new(
            stream::once(async move { dir.entries().await })
                .try_flatten()
                .map_ok(Entry::<B>::from)
                .map_err(|error| Raise::<B>::from(error).into()),
        ))
    }

    #[guestpy(method)]
    fn walk(&self) -> Result<HostStream<Entry<B>>, Error> {
        let dir = self.dir.clone();

        Ok(HostStream::new(
            stream::once(async move { dir.walk().await })
                .try_flatten()
                .map_ok(Entry::<B>::from)
                .map_err(|error| Raise::<B>::from(error).into()),
        ))
    }

    #[guestpy(async_method)]
    fn entry(
        &self,
        #[guestpy(positional)] path: StrPath,
    ) -> Result<impl Future<Output = Result<Entry<B>, Error>> + use<B>, Error> {
        let dir = self.dir.clone();

        Ok(async move {
            Ok(Entry::<B>::from(
                dir.entry(path.into_inner())
                    .await
                    .map_err(Raise::<B>::from)?,
            ))
        })
    }

    #[guestpy(async_method)]
    fn stat(
        &self,
        #[guestpy(positional)] path: StrPath,
        #[guestpy(kw)] follow_symlinks: Option<bool>,
    ) -> Result<impl Future<Output = Result<Stat<B>, Error>> + use<B>, Error> {
        let dir = self.dir.clone();

        Ok(async move {
            Ok(Stat::<B>::from(
                match follow_symlinks.unwrap_or(true) {
                    true => dir.metadata(path.into_inner()).await,
                    false => dir.symlink_metadata(path.into_inner()).await,
                }
                .map_err(Raise::<B>::from)?,
            ))
        })
    }

    #[guestpy(async_method)]
    fn exists(
        &self,
        #[guestpy(positional)] path: StrPath,
        #[guestpy(kw)] follow_symlinks: Option<bool>,
    ) -> Result<impl Future<Output = Result<bool, Error>> + use<B>, Error> {
        let dir = self.dir.clone();

        Ok(async move {
            match match follow_symlinks.unwrap_or(true) {
                true => dir.metadata(path.into_inner()).await,
                false => dir.symlink_metadata(path.into_inner()).await,
            } {
                Ok(_) => Ok(true),
                Err(error) if error.is_not_found() => Ok(false),
                Err(error) => Err(Raise::<B>::from(error).into()),
            }
        })
    }

    #[guestpy(async_method)]
    fn access(
        &self,
        #[guestpy(positional)] path: StrPath,
        #[guestpy(kw)] read: Option<bool>,
        #[guestpy(kw)] write: Option<bool>,
        #[guestpy(kw)] execute: Option<bool>,
        #[guestpy(kw)] follow_symlinks: Option<bool>,
    ) -> Result<impl Future<Output = Result<bool, Error>> + use<B>, Error> {
        let dir = self.dir.clone();

        Ok(async move {
            match dir
                .access(
                    path.into_inner(),
                    &AccessOptions::new()
                        .read(read.unwrap_or(false))
                        .write(write.unwrap_or(false))
                        .execute(execute.unwrap_or(false))
                        .follow_symlinks(follow_symlinks.unwrap_or(true)),
                )
                .await
            {
                Ok(()) => Ok(true),
                Err(error) if error.is_permission_denied() => Ok(false),
                Err(error) => Err(Raise::<B>::from(error).into()),
            }
        })
    }

    #[guestpy(async_method)]
    fn chmod(
        &self,
        #[guestpy(positional)] path: StrPath,
        #[guestpy(positional)] mode: u32,
    ) -> Result<impl Future<Output = Result<(), Error>> + use<B>, Error> {
        let dir = self.dir.clone();

        Ok(async move {
            Ok(
                dir.set_permissions(path.into_inner(), Permissions::new(mode))
                    .await
                    .map_err(Raise::<B>::from)?
            )
        })
    }

    #[guestpy(async_method)]
    fn chown(
        &self,
        #[guestpy(positional)] path: StrPath,
        #[guestpy(kw)] uid: Option<u32>,
        #[guestpy(kw)] gid: Option<u32>,
        #[guestpy(kw)] follow_symlinks: Option<bool>,
    ) -> Result<impl Future<Output = Result<(), Error>> + use<B>, Error> {
        let dir = self.dir.clone();

        Ok(async move {
            Ok(
                dir.set_owner_with_options(
                    path.into_inner(),
                    Owner::new().user(uid).group(gid),
                    &SetOwnerOptions::new()
                        .follow_symlinks(follow_symlinks.unwrap_or(true)),
                )
                .await
                .map_err(Raise::<B>::from)?
            )
        })
    }

    #[guestpy(async_method)]
    fn symlink(
        &self,
        #[guestpy(positional)] target: StrPath,
        #[guestpy(positional)] link: StrPath,
    ) -> Result<impl Future<Output = Result<(), Error>> + use<B>, Error> {
        let dir = self.dir.clone();

        Ok(async move {
            Ok(
                dir.symlink(target.into_inner(), link.into_inner())
                    .await
                    .map_err(Raise::<B>::from)?
            )
        })
    }

    #[guestpy(async_method)]
    fn readlink(
        &self,
        #[guestpy(positional)] path: StrPath,
    ) -> Result<impl Future<Output = Result<String, Error>> + use<B>, Error> {
        let dir = self.dir.clone();

        Ok(async move {
            Ok(
                dir.read_link(path.into_inner())
                    .await
                    .map_err(Raise::<B>::from)?
                    .to_string_lossy()
            )
        })
    }

    #[guestpy(async_method)]
    fn rename(
        &self,
        #[guestpy(positional)] source: StrPath,
        #[guestpy(positional)] destination: StrPath,
    ) -> Result<impl Future<Output = Result<(), Error>> + use<B>, Error> {
        let dir = self.dir.clone();

        Ok(async move {
            Ok(
                dir.rename(source.into_inner(), destination.into_inner())
                    .await
                    .map_err(Raise::<B>::from)?
            )
        })
    }

    #[guestpy(async_method)]
    fn remove(
        &self,
        #[guestpy(positional)] path: StrPath,
    ) -> Result<impl Future<Output = Result<(), Error>> + use<B>, Error> {
        let dir = self.dir.clone();

        Ok(async move {
            Ok(
                dir.remove_file(path.into_inner())
                    .await
                    .map_err(Raise::<B>::from)?
            )
        })
    }

    #[guestpy(async_method)]
    fn rmdir(
        &self,
        #[guestpy(positional)] path: StrPath,
    ) -> Result<impl Future<Output = Result<(), Error>> + use<B>, Error> {
        let dir = self.dir.clone();

        Ok(async move {
            Ok(
                dir.remove_dir(path.into_inner())
                    .await
                    .map_err(Raise::<B>::from)?
            )
        })
    }

    #[guestpy(async_method)]
    fn rmtree(
        &self,
        #[guestpy(positional)] path: StrPath,
    ) -> Result<impl Future<Output = Result<(), Error>> + use<B>, Error> {
        let dir = self.dir.clone();

        Ok(async move {
            Ok(
                dir.remove_dir_all(path.into_inner())
                    .await
                    .map_err(Raise::<B>::from)?
            )
        })
    }
}

impl<B: ExecutorBackend> From<Dir> for Directory<B> {
    fn from(dir: Dir) -> Self {
        Self {
            dir,
            backend: PhantomData,
        }
    }
}
