// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{
    error::Error as StdError,
    ffi::{OsStr, OsString},
    io::ErrorKind,
    path::PathBuf as HostPathBuf,
    vec::IntoIter,
};

use async_trait::async_trait;
use futures::stream::{self, Iter};
use tokio::fs::{self, File};

#[cfg(target_os = "linux")]
use rustix::fs::{Mode, OFlags, ResolveFlags, openat2};
#[cfg(not(target_os = "linux"))]
use tokio::fs::OpenOptions as HostOpenOptions;

use crate::{
    backend::Backend,
    errors::Error,
    fs::{
        Capabilities, CreateDirOptions, DirEntry, FileType, Metadata, MetadataOptions,
        OpenOptions, PermissionCapability, Permissions, RemoveDirOptions,
    },
    path::{Component, Path, PathBuf},
};

#[cfg(unix)]
use std::{
    os::unix::{
        ffi::{OsStrExt, OsStringExt},
        fs::{FileTypeExt, PermissionsExt},
    },
};

struct HostRoot {
    path: HostPathBuf,
    follow_symlinks: bool,
    #[cfg(target_os = "linux")]
    file: std::fs::File,
}

struct HostComponent<'a>(&'a Component);

impl From<HostComponent<'_>> for OsString {
    fn from(component: HostComponent<'_>) -> Self {
        #[cfg(unix)]
        {
            return OsString::from_vec(component.0.as_bytes().to_vec());
        }

        #[cfg(not(unix))]
        {
            OsString::from(component.0.to_string_lossy())
        }
    }
}

struct HostFileName<'a>(&'a OsStr);

impl From<HostFileName<'_>> for Component {
    fn from(file_name: HostFileName<'_>) -> Self {
        #[cfg(unix)]
        {
            return Component::new(file_name.0.as_bytes().to_vec());
        }

        #[cfg(not(unix))]
        {
            Component::new(file_name.0.to_string_lossy().as_bytes().to_vec())
        }
    }
}

pub struct HostFs {
    root: HostRoot,
}

impl HostFs {
    pub fn builder() -> HostFsBuilder {
        HostFsBuilder::new()
    }

    fn resolve(&self, path: &Path) -> Result<HostPathBuf, Error> {
        let mut resolved = self.root.path.clone();

        for component in Self::local_components(path)? {
            resolved.push(OsString::from(HostComponent(&component)));
        }

        Ok(resolved)
    }

    fn local_path(&self, path: &Path) -> Result<HostPathBuf, Error> {
        let mut resolved = HostPathBuf::new();

        for component in Self::local_components(path)? {
            resolved.push(OsString::from(HostComponent(&component)));
        }

        if resolved.as_os_str().is_empty() {
            resolved.push(".");
        }

        Ok(resolved)
    }

    fn local_components(path: &Path) -> Result<Vec<Component>, Error> {
        let mut components = Vec::new();

        for component in path.components() {
            match component.as_bytes() {
                b"/" | b"." => {}
                b".." => return Err(Error::path_escapes_authority(path)),
                _ => components.push(component),
            }
        }

        Ok(components)
    }

    fn metadata_from_host(metadata: std::fs::Metadata) -> Metadata {
        Metadata::new(
            Self::file_type_from_host(metadata.file_type()),
            metadata.len(),
            Self::permissions_from_host(metadata.permissions()),
        )
        .with_accessed(metadata.accessed().ok())
        .with_modified(metadata.modified().ok())
        .with_created(metadata.created().ok())
    }

    fn file_type_from_host(file_type: std::fs::FileType) -> FileType {
        if file_type.is_file() {
            return FileType::File;
        }

        if file_type.is_dir() {
            return FileType::Directory;
        }

        if file_type.is_symlink() {
            return FileType::Symlink;
        }

        #[cfg(unix)]
        {
            if file_type.is_fifo() {
                return FileType::Fifo;
            }

            if file_type.is_socket() {
                return FileType::Socket;
            }

            if file_type.is_block_device() {
                return FileType::BlockDevice;
            }

            if file_type.is_char_device() {
                return FileType::CharacterDevice;
            }
        }

        FileType::Other
    }

    fn permissions_from_host(permissions: std::fs::Permissions) -> Permissions {
        #[cfg(unix)]
        {
            return Permissions::new()
                .readonly(permissions.readonly())
                .mode(permissions.mode());
        }

        #[cfg(not(unix))]
        {
            Permissions::new().readonly(permissions.readonly())
        }
    }

    fn map_io_error(path: &Path, message: &str, error: std::io::Error) -> Error {
        match error.kind() {
            ErrorKind::NotFound => Error::not_found(path),
            ErrorKind::AlreadyExists => Error::already_exists(path),
            ErrorKind::PermissionDenied => Error::permission_denied(path),
            ErrorKind::InvalidInput => Error::invalid_path(error.to_string()),
            _ => Error::unexpected(
                message,
                Some(Box::new(error) as Box<dyn StdError + Send + Sync>),
            ),
        }
    }

    #[cfg(target_os = "linux")]
    fn map_rustix_error(path: &Path, message: &str, error: rustix::io::Errno) -> Error {
        match error.kind() {
            ErrorKind::NotFound => Error::not_found(path),
            ErrorKind::AlreadyExists => Error::already_exists(path),
            ErrorKind::PermissionDenied => Error::permission_denied(path),
            ErrorKind::InvalidInput => Error::invalid_path(error.to_string()),
            _ => Error::unexpected(
                message,
                Some(Box::new(error) as Box<dyn StdError + Send + Sync>),
            ),
        }
    }
}

#[async_trait]
impl Backend for HostFs {
    type File = File;
    type DirEntries = Iter<IntoIter<Result<DirEntry, Error>>>;

    fn capabilities(&self) -> Capabilities {
        #[cfg(unix)]
        {
            return Capabilities::new()
                .symlink(true)
                .atomic_rename(true)
                .permissions(PermissionCapability::PosixMode)
                .timestamps(true);
        }

        #[cfg(not(unix))]
        {
            Capabilities::new()
                .symlink(true)
                .atomic_rename(true)
                .permissions(PermissionCapability::Readonly)
                .timestamps(true)
        }
    }

    async fn open(&self, path: &Path, options: &OpenOptions) -> Result<Self::File, Error> {
        if !self.root.follow_symlinks && options.follows_symlinks() {
            match fs::symlink_metadata(self.resolve(path)?).await {
                Ok(metadata) if metadata.file_type().is_symlink() => {
                    return Err(Error::permission_denied(path));
                }
                Ok(_) => {}
                Err(error) if error.kind() == ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(Self::map_io_error(
                        path,
                        "failed to inspect host path",
                        error,
                    ));
                }
            }
        }

        #[cfg(target_os = "linux")]
        {
            let mut flags = OFlags::CLOEXEC;

            flags |= match (options.is_read(), options.is_write() || options.is_append()) {
                (true, true) => OFlags::RDWR,
                (false, true) => OFlags::WRONLY,
                _ => OFlags::RDONLY,
            };

            if options.is_append() {
                flags |= OFlags::APPEND;
            }

            if options.is_truncate() {
                flags |= OFlags::TRUNC;
            }

            if options.is_create() || options.is_create_new() {
                flags |= OFlags::CREATE;
            }

            if options.is_create_new() {
                flags |= OFlags::EXCL;
            }

            if !self.root.follow_symlinks || !options.follows_symlinks() {
                flags |= OFlags::NOFOLLOW;
            }

            let mut resolve = ResolveFlags::IN_ROOT | ResolveFlags::NO_MAGICLINKS;

            if !self.root.follow_symlinks || !options.follows_symlinks() {
                resolve |= ResolveFlags::NO_SYMLINKS;
            }

            return Ok(
                File::from_std(std::fs::File::from(
                    openat2(
                        &self.root.file,
                        self.local_path(path)?,
                        flags,
                        if options.is_create() || options.is_create_new() {
                            Mode::from_raw_mode(0o666)
                        } else {
                            Mode::empty()
                        },
                        resolve,
                    )
                    .map_err(|error| {
                        Self::map_rustix_error(path, "failed to open rooted host file", error)
                    })?,
                ))
            );
        }

        #[cfg(not(target_os = "linux"))]
        {
            HostOpenOptions::new()
                .read(options.is_read())
                .write(options.is_write())
                .append(options.is_append())
                .truncate(options.is_truncate())
                .create(options.is_create())
                .create_new(options.is_create_new())
                .open(self.resolve(path)?)
                .await
                .map_err(|error| Self::map_io_error(path, "failed to open host file", error))
        }
    }

    async fn entries(&self, path: &Path) -> Result<Self::DirEntries, Error> {
        let mut reader = fs::read_dir(self.resolve(path)?)
            .await
            .map_err(|error| Self::map_io_error(path, "failed to read host directory", error))?;
        let mut entries = Vec::new();

        while let Some(entry) = reader
            .next_entry()
            .await
            .map_err(|error| Self::map_io_error(path, "failed to read host directory entry", error))?
        {
            let file_name = Component::from(HostFileName(&entry.file_name()));
            let child_path = PathBuf::from(path).join(file_name.as_bytes())?;
            let metadata = entry
                .metadata()
                .await
                .map_err(|error| {
                    Self::map_io_error(
                        child_path.as_path(),
                        "failed to read host directory entry metadata",
                        error,
                    )
                })?;

            entries.push(Ok(DirEntry::new(
                child_path,
                file_name,
                Self::file_type_from_host(metadata.file_type()),
                Self::metadata_from_host(metadata),
            )));
        }

        Ok(stream::iter(entries))
    }

    async fn metadata(&self, path: &Path, options: &MetadataOptions) -> Result<Metadata, Error> {
        Ok(Self::metadata_from_host(
            if options.follows_symlinks() {
                fs::metadata(self.resolve(path)?).await
            } else {
                fs::symlink_metadata(self.resolve(path)?).await
            }
            .map_err(|error| Self::map_io_error(path, "failed to read host metadata", error))?,
        ))
    }

    async fn create_dir(&self, path: &Path, options: &CreateDirOptions) -> Result<(), Error> {
        if options.is_recursive() {
            fs::create_dir_all(self.resolve(path)?).await
        } else {
            fs::create_dir(self.resolve(path)?).await
        }
        .map_err(|error| Self::map_io_error(path, "failed to create host directory", error))
    }

    async fn remove_file(&self, path: &Path) -> Result<(), Error> {
        fs::remove_file(self.resolve(path)?)
            .await
            .map_err(|error| Self::map_io_error(path, "failed to remove host file", error))
    }

    async fn remove_dir(&self, path: &Path, options: &RemoveDirOptions) -> Result<(), Error> {
        if options.is_recursive() {
            fs::remove_dir_all(self.resolve(path)?).await
        } else {
            fs::remove_dir(self.resolve(path)?).await
        }
        .map_err(|error| Self::map_io_error(path, "failed to remove host directory", error))
    }

    async fn rename(&self, from: &Path, to: &Path) -> Result<(), Error> {
        fs::rename(self.resolve(from)?, self.resolve(to)?)
            .await
            .map_err(|error| Self::map_io_error(from, "failed to rename host path", error))
    }

    async fn symlink(&self, target: &Path, link: &Path) -> Result<(), Error> {
        #[cfg(unix)]
        {
            tokio::fs::symlink(self.resolve(target)?, self.resolve(link)?)
                .await
                .map_err(|error| Self::map_io_error(link, "failed to create host symlink", error))
        }

        #[cfg(windows)]
        {
            let target_path = self.resolve(target)?;
            let link_path = self.resolve(link)?;

            if fs::metadata(&target_path)
                .await
                .map_err(|error| Self::map_io_error(target, "failed to inspect host symlink target", error))?
                .file_type()
                .is_dir()
            {
                fs::symlink_dir(target_path, link_path).await
            } else {
                fs::symlink_file(target_path, link_path).await
            }
            .map_err(|error| Self::map_io_error(link, "failed to create host symlink", error))
        }

        #[cfg(not(any(unix, windows)))]
        {
            Err(Error::unsupported("host symlinks are unsupported on this platform"))
        }
    }

    async fn read_link(&self, path: &Path) -> Result<PathBuf, Error> {
        PathBuf::from_host_path(
            fs::read_link(self.resolve(path)?)
                .await
                .map_err(|error| Self::map_io_error(path, "failed to read host symlink", error))?,
        )
    }

    async fn set_permissions(&self, path: &Path, permissions: Permissions) -> Result<(), Error> {
        let mut host_permissions = fs::metadata(self.resolve(path)?)
            .await
            .map_err(|error| Self::map_io_error(path, "failed to read host metadata", error))?
            .permissions();

        host_permissions.set_readonly(permissions.is_readonly());

        #[cfg(unix)]
        if let Some(mode) = permissions.posix_mode() {
            host_permissions.set_mode(mode);
        }

        fs::set_permissions(self.resolve(path)?, host_permissions)
            .await
            .map_err(|error| Self::map_io_error(path, "failed to set host permissions", error))
    }
}

pub struct HostFsBuilder {
    root: Option<HostPathBuf>,
    follow_symlinks: bool,
}

impl HostFsBuilder {
    pub fn new() -> Self {
        HostFsBuilder { root: None, follow_symlinks: false }
    }

    pub fn root(mut self, root: impl Into<HostPathBuf>) -> Self {
        self.root = Some(root.into());
        self
    }

    pub fn follow_symlinks(mut self, follow: bool) -> Self {
        self.follow_symlinks = follow;
        self
    }

    pub fn build(self) -> Result<HostFs, Error> {
        let root = self
            .root
            .ok_or_else(|| Error::invalid_path("host root is required"))?;

        if !root.exists() {
            return Err(Error::not_found(PathBuf::root()));
        }

        if !root.is_dir() {
            return Err(Error::not_directory(PathBuf::root()));
        }

        #[cfg(target_os = "linux")]
        let file = std::fs::File::open(&root)
            .map_err(|error| {
                HostFs::map_io_error(
                    PathBuf::root().as_path(),
                    "failed to open host root",
                    error,
                )
            })?;

        Ok(HostFs {
            root: HostRoot {
                path: root,
                follow_symlinks: self.follow_symlinks,
                #[cfg(target_os = "linux")]
                file,
            },
        })
    }
}

impl Default for HostFsBuilder {
    fn default() -> Self {
        HostFsBuilder::new()
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs as std_fs,
        path::{Path as HostPath, PathBuf as HostPathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use crate::{
        backend::Backend,
        errors::Error,
        fs::{FileType, Fs, MetadataOptions},
        host::HostFs,
        path::PathBuf,
    };

    struct TempRoot {
        path: HostPathBuf,
    }

    impl TempRoot {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "agentc-fs-host-{}",
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));

            std_fs::create_dir_all(&path).unwrap();

            TempRoot { path }
        }

        fn path(&self) -> &HostPath {
            &self.path
        }
    }

    impl Drop for TempRoot {
        fn drop(&mut self) {
            let _ = std_fs::remove_dir_all(&self.path);
        }
    }

    #[tokio::test]
    async fn reads_from_host_root() {
        let root = TempRoot::new();

        std_fs::write(root.path.join("readme.txt"), b"host").unwrap();

        let mut file = Fs::new(
            HostFs::builder()
                .root(root.path())
                .build()
                .unwrap(),
        )
        .root()
        .open_file("/readme.txt")
        .await
        .unwrap();

        assert_eq!(file.read_to_string().await.unwrap(), "host");
    }

    #[tokio::test]
    async fn writes_to_host_root() {
        let root = TempRoot::new();

        let mut file = Fs::new(
            HostFs::builder()
                .root(root.path())
                .build()
                .unwrap(),
        )
        .root()
            .options()
            .write(true)
            .create(true)
            .open("/notes.txt")
            .await
            .unwrap();

        file.write_all(b"created").await.unwrap();
        file.flush().await.unwrap();

        assert_eq!(std_fs::read_to_string(root.path.join("notes.txt")).unwrap(), "created");
    }

    #[tokio::test]
    async fn lists_host_directory_entries() {
        let root = TempRoot::new();

        std_fs::write(root.path.join("a.txt"), b"a").unwrap();
        std_fs::create_dir(root.path.join("nested")).unwrap();

        let mut entries = Fs::new(
            HostFs::builder()
                .root(root.path())
                .build()
                .unwrap(),
        )
        .root()
        .entries()
        .await
        .unwrap();
        let mut names = Vec::new();

        while let Some(entry) = entries.next().await.unwrap() {
            names.push((entry.file_name().to_string_lossy(), entry.file_type()));
        }

        names.sort_by(|left, right| left.0.cmp(&right.0));

        assert_eq!(
            names,
            vec![
                ("a.txt".to_string(), FileType::File),
                ("nested".to_string(), FileType::Directory),
            ]
        );
    }

    #[tokio::test]
    async fn rejects_parent_escape() {
        let root = TempRoot::new();

        assert!(matches!(
            Backend::metadata(
                &HostFs::builder()
                    .root(root.path())
                    .build()
                    .unwrap(),
                PathBuf::parse("/../outside.txt")
                    .unwrap()
                    .as_path(),
                &MetadataOptions::new(),
            )
            .await,
            Err(Error::PathEscapesAuthority(path)) if path.to_string_lossy() == "/../outside.txt"
        ));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn rejects_symlink_when_following_is_disabled() {
        use std::os::unix::fs::symlink;

        let root = TempRoot::new();

        std_fs::write(root.path.join("target.txt"), b"target").unwrap();
        symlink(root.path.join("target.txt"), root.path.join("link.txt")).unwrap();

        assert!(matches!(
            Fs::new(
                HostFs::builder()
                    .root(root.path())
                    .build()
                    .unwrap(),
            )
            .root()
            .open_file("/link.txt")
            .await,
            Err(Error::PermissionDenied(path)) if path.to_string_lossy() == "/link.txt"
        ));
    }
}
