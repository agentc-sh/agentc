// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{path::Path as HostPath, vec::IntoIter};

use async_trait::async_trait;
use futures::stream::{self, Iter};
use include_dir::DirEntry as IncludeDirEntry;

use crate::{
    backend::Backend,
    embedded::{
        file::EmbeddedFile,
        types::{EmbeddedDirectory, EmbeddedSource},
    },
    errors::Error,
    fs::{
        AccessOptions, Capabilities, CreateDirOptions, DirEntry, FileType, Metadata,
        MetadataOptions, OpenOptions, Owner, Permissions, RemoveDirOptions, SetOwnerOptions,
    },
    path::{Component, Path, PathBuf},
};

pub struct EmbeddedFs {
    source: EmbeddedSource,
}

impl EmbeddedFs {
    pub fn file(bytes: &'static [u8]) -> Self {
        EmbeddedFs { source: EmbeddedSource::File { bytes } }
    }

    pub fn directory(directory: EmbeddedDirectory) -> Self {
        EmbeddedFs {
            source: EmbeddedSource::Directory { directory },
        }
    }

    fn metadata(file_type: FileType, len: u64) -> Metadata {
        Metadata::new(
            file_type,
            len,
            Permissions::new(match file_type {
                FileType::Directory => 0o555,
                _ => 0o444,
            }),
        )
    }

    fn relative_path(path: &Path) -> String {
        String::from_utf8_lossy(
            path.as_bytes()
                .strip_prefix(b"/")
                .unwrap_or(path.as_bytes()),
        )
        .into_owned()
    }

    fn file_name(path: &HostPath) -> Result<Component, Error> {
        Ok(Component::new(
            path.file_name()
                .ok_or_else(|| Error::invalid_path("embedded path does not have a file name"))?
                .to_string_lossy()
                .into_owned()
                .into_bytes(),
        ))
    }
}

#[async_trait]
impl Backend for EmbeddedFs {
    type File = EmbeddedFile;
    type DirEntries = Iter<IntoIter<Result<DirEntry, Error>>>;

    fn capabilities(&self) -> Capabilities {
        Capabilities::new().permissions(false)
    }

    async fn open(&self, path: &Path, options: &OpenOptions) -> Result<Self::File, Error> {
        if options.is_write()
            || options.is_append()
            || options.is_truncate()
            || options.is_create()
            || options.is_create_new()
        {
            return Err(Error::permission_denied(path));
        }

        match &self.source {
            EmbeddedSource::File { bytes } if path.is_root() => {
                Ok(EmbeddedFile::new(PathBuf::from(path), *bytes))
            }
            EmbeddedSource::File { .. } => Err(Error::not_found(path)),
            EmbeddedSource::Directory { directory } => {
                let relative_path = Self::relative_path(path);
                let directory = directory.as_inner();

                if let Some(file) = directory.get_file(relative_path.as_str()) {
                    return Ok(EmbeddedFile::new(PathBuf::from(path), file.contents()));
                }

                if directory
                    .get_dir(relative_path.as_str())
                    .is_some()
                    || path.is_root()
                {
                    return Err(Error::is_directory(path));
                }

                Err(Error::not_found(path))
            }
        }
    }

    async fn entries(&self, path: &Path) -> Result<Self::DirEntries, Error> {
        match &self.source {
            EmbeddedSource::File { .. } if path.is_root() => Err(Error::not_directory(path)),
            EmbeddedSource::File { .. } => Err(Error::not_found(path)),
            EmbeddedSource::Directory { directory } => {
                let directory = directory.as_inner();
                let directory = if path.is_root() {
                    directory
                } else {
                    let relative_path = Self::relative_path(path);

                    if directory
                        .get_file(relative_path.as_str())
                        .is_some()
                    {
                        return Err(Error::not_directory(path));
                    }

                    directory
                        .get_dir(relative_path.as_str())
                        .ok_or_else(|| Error::not_found(path))?
                };
                let mut entries = Vec::new();

                for entry in directory.entries() {
                    entries.push(match entry {
                        IncludeDirEntry::Dir(directory) => {
                            let file_name = Self::file_name(directory.path())?;

                            Ok(DirEntry::new(
                                PathBuf::parse(path.as_bytes())?.join(file_name.as_bytes())?,
                                file_name,
                                FileType::Directory,
                                Self::metadata(
                                    FileType::Directory,
                                    directory.entries().len() as u64,
                                ),
                            ))
                        }
                        IncludeDirEntry::File(file) => {
                            let file_name = Self::file_name(file.path())?;

                            Ok(DirEntry::new(
                                PathBuf::parse(path.as_bytes())?.join(file_name.as_bytes())?,
                                file_name,
                                FileType::File,
                                Self::metadata(FileType::File, file.contents().len() as u64),
                            ))
                        }
                    });
                }

                Ok(stream::iter(entries))
            }
        }
    }

    async fn metadata(&self, path: &Path, _options: &MetadataOptions) -> Result<Metadata, Error> {
        match &self.source {
            EmbeddedSource::File { bytes } if path.is_root() => {
                Ok(Self::metadata(FileType::File, bytes.len() as u64))
            }
            EmbeddedSource::File { .. } => Err(Error::not_found(path)),
            EmbeddedSource::Directory { directory } => {
                let directory = directory.as_inner();
                if path.is_root() {
                    return Ok(Self::metadata(
                        FileType::Directory,
                        directory.entries().len() as u64,
                    ));
                }

                let relative_path = Self::relative_path(path);

                if let Some(file) = directory.get_file(relative_path.as_str()) {
                    return Ok(Self::metadata(FileType::File, file.contents().len() as u64));
                }

                if let Some(directory) = directory.get_dir(relative_path.as_str()) {
                    return Ok(Self::metadata(
                        FileType::Directory,
                        directory.entries().len() as u64,
                    ));
                }

                Err(Error::not_found(path))
            }
        }
    }

    async fn access(&self, path: &Path, options: &AccessOptions) -> Result<(), Error> {
        options.evaluate(
            path,
            &Backend::metadata(
                self,
                path,
                &MetadataOptions::new().follow_symlinks(options.follows_symlinks()),
            )
            .await?,
        )
    }

    async fn create_dir(&self, path: &Path, _options: &CreateDirOptions) -> Result<bool, Error> {
        Err(Error::permission_denied(path))
    }

    async fn remove_file(&self, path: &Path) -> Result<(), Error> {
        Err(Error::permission_denied(path))
    }

    async fn remove_dir(&self, path: &Path, _options: &RemoveDirOptions) -> Result<(), Error> {
        Err(Error::permission_denied(path))
    }

    async fn truncate(&self, path: &Path, _len: u64) -> Result<(), Error> {
        Err(Error::permission_denied(path))
    }

    async fn rename(&self, from: &Path, _to: &Path) -> Result<(), Error> {
        Err(Error::permission_denied(from))
    }

    async fn symlink(&self, _target: &Path, _link: &Path) -> Result<(), Error> {
        Err(Error::unsupported("embedded filesystem does not support symlinks"))
    }

    async fn read_link(&self, _path: &Path) -> Result<PathBuf, Error> {
        Err(Error::unsupported("embedded filesystem does not support symlinks"))
    }

    async fn set_permissions(&self, path: &Path, _permissions: Permissions) -> Result<(), Error> {
        Err(Error::permission_denied(path))
    }

    async fn set_owner(
        &self,
        path: &Path,
        _owner: Owner,
        _options: &SetOwnerOptions,
    ) -> Result<(), Error> {
        Err(Error::permission_denied(path))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        embedded_dir, embedded_file,
        errors::Error,
        fs::{AccessOptions, FileType, Fs},
    };

    #[tokio::test]
    async fn reads_embedded_file_source() {
        let mut file = Fs::new(embedded_file!("../lib.rs"))
            .root()
            .open_file("/")
            .await
            .unwrap();

        assert!(
            file.read_to_string()
                .await
                .unwrap()
                .contains("agentc_fs")
        );
    }

    #[tokio::test]
    async fn lists_embedded_directory_source() {
        let mut entries = Fs::new(embedded_dir!("$CARGO_MANIFEST_DIR/src"))
            .root()
            .entries()
            .await
            .unwrap();
        let mut names = Vec::new();

        while let Some(entry) = entries.next().await.unwrap() {
            names.push(entry.file_name().to_string_lossy());
        }

        names.sort();

        assert!(names.contains(&"lib.rs".to_string()));
        assert!(names.contains(&"fs".to_string()));
    }

    #[tokio::test]
    async fn returns_metadata_for_embedded_directory_entries() {
        let root = Fs::new(embedded_dir!("$CARGO_MANIFEST_DIR/src")).root();

        assert_eq!(
            root.metadata("lib.rs")
                .await
                .unwrap()
                .file_type(),
            FileType::File
        );
        assert_eq!(
            root.metadata("fs")
                .await
                .unwrap()
                .file_type(),
            FileType::Directory
        );
    }

    #[tokio::test]
    async fn rejects_embedded_mutation() {
        assert!(matches!(
            Fs::new(embedded_dir!("$CARGO_MANIFEST_DIR/src"))
                .root()
                .options()
                .write(true)
                .open("lib.rs")
                .await,
            Err(Error::PermissionDenied(path)) if path.to_string_lossy() == "/lib.rs"
        ));
    }

    #[tokio::test]
    async fn returns_not_found_for_missing_embedded_path() {
        assert!(matches!(
            Fs::new(embedded_dir!("$CARGO_MANIFEST_DIR/src"))
                .root()
                .open_file("missing.md")
                .await,
            Err(Error::NotFound(path)) if path.to_string_lossy() == "/missing.md"
        ));
    }

    #[tokio::test]
    async fn embedded_entries_are_readonly_modes() {
        let root = Fs::new(embedded_dir!("$CARGO_MANIFEST_DIR/src")).root();

        assert_eq!(
            root.metadata("lib.rs")
                .await
                .unwrap()
                .permissions()
                .mode(),
            0o444
        );
        assert_eq!(
            root.metadata("fs")
                .await
                .unwrap()
                .permissions()
                .mode(),
            0o555
        );
    }

    #[tokio::test]
    async fn embedded_access_denies_write() {
        assert!(matches!(
            Fs::new(embedded_dir!("$CARGO_MANIFEST_DIR/src"))
                .root()
                .access("lib.rs", &AccessOptions::new().write(true))
                .await,
            Err(Error::PermissionDenied(path)) if path.to_string_lossy() == "/lib.rs"
        ));
    }
}
