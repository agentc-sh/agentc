// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{collections::BTreeMap, sync::Arc, vec::IntoIter};

use async_trait::async_trait;
use futures::{
    StreamExt,
    stream::{self, Iter},
};
use tokio::sync::RwLock;

use crate::{
    backend::{Backend, ErasedBackend, FileHandle},
    errors::Error,
    fs::{
        AccessOptions, Capabilities, CreateDirOptions, DirEntry, File, FileType, Metadata,
        MetadataOptions, OpenOptions, Owner, Permissions, RemoveDirOptions, SetOwnerOptions,
    },
    overlay::whiteout::Whiteouts,
    path::{Path, PathBuf},
};

pub struct OverlayFs {
    upper: Arc<dyn ErasedBackend>,
    lower: Arc<dyn ErasedBackend>,
    whiteouts: Arc<RwLock<Whiteouts>>,
}

impl OverlayFs {
    pub fn new(upper: impl Backend, lower: impl Backend) -> Self {
        OverlayFs {
            upper: Arc::new(upper),
            lower: Arc::new(lower),
            whiteouts: Arc::new(RwLock::new(Whiteouts::new())),
        }
    }

    async fn is_whiteout(&self, path: &Path) -> bool {
        self.whiteouts
            .read()
            .await
            .contains(path)
    }

    async fn clear_whiteout(&self, path: &Path) {
        self.whiteouts
            .write()
            .await
            .remove(path);
    }

    async fn upper_exists(&self, path: &Path) -> Result<bool, Error> {
        match self
            .upper
            .metadata(path, &MetadataOptions::new().follow_symlinks(false))
            .await
        {
            Ok(_) => Ok(true),
            Err(Error::NotFound(_)) => Ok(false),
            Err(error) => Err(error),
        }
    }

    async fn lower_metadata(&self, path: &Path) -> Result<Option<Metadata>, Error> {
        match self
            .lower
            .metadata(path, &MetadataOptions::new().follow_symlinks(false))
            .await
        {
            Ok(metadata) => Ok(Some(metadata)),
            Err(Error::NotFound(_)) => Ok(None),
            Err(error) => Err(error),
        }
    }

    async fn create_upper_parent(&self, path: &Path) -> Result<(), Error> {
        if let Some(parent) = path.parent() {
            if parent.as_bytes() != b"/" {
                self.upper
                    .create_dir(parent, &CreateDirOptions::new().recursive(true))
                    .await?;
            }
        }

        Ok(())
    }

    async fn copy_lower_file_to_upper(
        &self,
        path: &Path,
        options: &OpenOptions,
    ) -> Result<(), Error> {
        if self.upper_exists(path).await? {
            return Ok(());
        }

        match self.lower_metadata(path).await? {
            Some(metadata) if metadata.file_type() == FileType::File => {
                if options.is_create_new() {
                    return Err(Error::already_exists(path));
                }
            }
            Some(metadata) if metadata.file_type() == FileType::Directory => {
                return Err(Error::is_directory(path));
            }
            Some(_) => {
                return Err(Error::unsupported(
                    "copying lower special files into an overlay upper layer is unsupported",
                ));
            }
            None => return Ok(()),
        }

        let mut lower = File::new(
            self.lower
                .open(path, &OpenOptions::new().read(true))
                .await?,
        );

        self.create_upper_parent(path).await?;
        let mut upper = File::new(
            self.upper
                .open(
                    path,
                    &OpenOptions::new()
                        .write(true)
                        .create(true)
                        .truncate(true),
                )
                .await?,
        );

        upper
            .write_all(lower.read_to_end().await?)
            .await?;

        Ok(())
    }

    async fn upper_directory_is_opaque(&self, path: &Path) -> bool {
        if self
            .whiteouts
            .read()
            .await
            .is_opaque(path)
        {
            return true;
        }

        matches!(
            self.upper
                .metadata(path, &MetadataOptions::new().follow_symlinks(false))
                .await,
            Ok(metadata) if metadata.file_type() == FileType::Directory
        )
    }

    async fn collect_entries(
        &self,
        path: &Path,
        entries: &mut BTreeMap<Vec<u8>, DirEntry>,
        upper: bool,
    ) -> Result<(), Error> {
        let mut cursor = if upper {
            match self.upper.entries(path).await {
                Ok(cursor) => cursor,
                Err(Error::NotFound(_)) => return Ok(()),
                Err(error) => return Err(error),
            }
        } else {
            match self.lower.entries(path).await {
                Ok(cursor) => cursor,
                Err(Error::NotFound(_)) => return Ok(()),
                Err(error) => return Err(error),
            }
        };

        while let Some(entry) = cursor.next().await.transpose()? {
            if !self
                .whiteouts
                .read()
                .await
                .child_contains(path, entry.file_name())
            {
                entries.insert(entry.file_name().as_bytes().to_vec(), entry);
            }
        }

        Ok(())
    }
}

#[async_trait]
impl Backend for OverlayFs {
    type File = Box<dyn FileHandle>;
    type DirEntries = Iter<IntoIter<Result<DirEntry, Error>>>;

    fn capabilities(&self) -> Capabilities {
        Capabilities::new()
            .permissions(true)
            .timestamps(true)
    }

    async fn open(&self, path: &Path, options: &OpenOptions) -> Result<Self::File, Error> {
        if self.is_whiteout(path).await {
            if !options.is_write()
                || options.is_append()
                || options.is_truncate()
                || options.is_create()
                || options.is_create_new()
            {
                return Err(Error::not_found(path));
            }

            self.clear_whiteout(path).await;
        }

        if !options.is_write()
            || options.is_append()
            || options.is_truncate()
            || options.is_create()
            || options.is_create_new()
        {
            self.copy_lower_file_to_upper(path, options)
                .await?;

            return self.upper.open(path, options).await;
        }

        match self.upper.open(path, options).await {
            Ok(file) => Ok(file),
            Err(Error::NotFound(_)) => self.lower.open(path, options).await,
            Err(error) => Err(error),
        }
    }

    async fn entries(&self, path: &Path) -> Result<Self::DirEntries, Error> {
        if self.is_whiteout(path).await {
            return Err(Error::not_found(path));
        }

        if Backend::metadata(self, path, &MetadataOptions::new().follow_symlinks(false))
            .await?
            .file_type()
            != FileType::Directory
        {
            return Err(Error::not_directory(path));
        }

        let mut entries = BTreeMap::new();

        if !self
            .upper_directory_is_opaque(path)
            .await
        {
            self.collect_entries(path, &mut entries, false)
                .await?;
        }

        self.collect_entries(path, &mut entries, true)
            .await?;

        Ok(stream::iter(
            entries
                .into_values()
                .map(Ok)
                .collect::<Vec<_>>(),
        ))
    }

    async fn metadata(&self, path: &Path, options: &MetadataOptions) -> Result<Metadata, Error> {
        if self.is_whiteout(path).await {
            return Err(Error::not_found(path));
        }

        match self.upper.metadata(path, options).await {
            Ok(metadata) => Ok(metadata),
            Err(Error::NotFound(_)) => self.lower.metadata(path, options).await,
            Err(error) => Err(error),
        }
    }

    async fn access(&self, path: &Path, options: &AccessOptions) -> Result<(), Error> {
        if self.is_whiteout(path).await {
            return Err(Error::not_found(path));
        }

        match self.upper.access(path, options).await {
            Ok(()) => Ok(()),
            Err(Error::NotFound(_)) => self.lower.access(path, options).await,
            Err(error) => Err(error),
        }
    }

    async fn create_dir(&self, path: &Path, options: &CreateDirOptions) -> Result<bool, Error> {
        let existed = self.upper_exists(path).await?;

        self.clear_whiteout(path).await;
        self.create_upper_parent(path).await?;
        let created = self
            .upper
            .create_dir(path, options)
            .await?;

        if !existed && created {
            self.whiteouts
                .write()
                .await
                .opaque(PathBuf::from(path));
        }

        Ok(created)
    }

    async fn remove_file(&self, path: &Path) -> Result<(), Error> {
        let removed_upper = match self.upper.remove_file(path).await {
            Ok(()) => true,
            Err(Error::NotFound(_)) => false,
            Err(error) => return Err(error),
        };

        let removed_lower = self
            .lower_metadata(path)
            .await?
            .is_some();

        if removed_lower {
            self.whiteouts
                .write()
                .await
                .insert(PathBuf::from(path));
        }

        if removed_upper || removed_lower {
            Ok(())
        } else {
            Err(Error::not_found(path))
        }
    }

    async fn remove_dir(&self, path: &Path, options: &RemoveDirOptions) -> Result<(), Error> {
        let removed_upper = match self
            .upper
            .remove_dir(path, options)
            .await
        {
            Ok(()) => true,
            Err(Error::NotFound(_)) => false,
            Err(error) => return Err(error),
        };

        let removed_lower = self
            .lower_metadata(path)
            .await?
            .is_some();

        if removed_lower {
            self.whiteouts
                .write()
                .await
                .insert(PathBuf::from(path));
        }

        if removed_upper || removed_lower {
            Ok(())
        } else {
            Err(Error::not_found(path))
        }
    }

    async fn truncate(&self, path: &Path, len: u64) -> Result<(), Error> {
        self.copy_lower_file_to_upper(path, &OpenOptions::new().write(true))
            .await?;

        self.upper.truncate(path, len).await
    }

    async fn rename(&self, from: &Path, to: &Path) -> Result<(), Error> {
        if self.is_whiteout(from).await {
            return Err(Error::not_found(from));
        }

        self.copy_lower_file_to_upper(from, &OpenOptions::new().write(true))
            .await?;
        self.clear_whiteout(to).await;
        self.create_upper_parent(to).await?;
        self.upper.rename(from, to).await
    }

    async fn symlink(&self, target: &Path, link: &Path) -> Result<(), Error> {
        self.clear_whiteout(link).await;
        self.create_upper_parent(link).await?;
        self.upper.symlink(target, link).await
    }

    async fn read_link(&self, path: &Path) -> Result<PathBuf, Error> {
        if self.is_whiteout(path).await {
            return Err(Error::not_found(path));
        }

        match self.upper.read_link(path).await {
            Ok(path) => Ok(path),
            Err(Error::NotFound(_)) => self.lower.read_link(path).await,
            Err(error) => Err(error),
        }
    }

    async fn set_permissions(&self, path: &Path, permissions: Permissions) -> Result<(), Error> {
        self.copy_lower_file_to_upper(path, &OpenOptions::new().write(true))
            .await?;
        self.upper
            .set_permissions(path, permissions)
            .await
    }

    async fn set_owner(
        &self,
        path: &Path,
        owner: Owner,
        options: &SetOwnerOptions,
    ) -> Result<(), Error> {
        self.copy_lower_file_to_upper(path, &OpenOptions::new().write(true))
            .await?;

        self.upper
            .set_owner(path, owner, options)
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        backend::Backend,
        errors::Error,
        fs::{CreateDirOptions, File, Fs, OpenOptions},
        memory::MemoryFs,
        overlay::OverlayFs,
        path::PathBuf,
    };

    struct MemorySource;

    impl MemorySource {
        async fn with_file(path: &str, content: &[u8]) -> MemoryFs {
            let fs = MemoryFs::new();
            let path = PathBuf::parse(path).unwrap();

            if let Some(parent) = path
                .parent()
                .filter(|parent| parent.as_bytes() != b"/")
            {
                Backend::create_dir(&fs, parent, &CreateDirOptions::new().recursive(true))
                    .await
                    .unwrap();
            }

            let mut file = File::new(Box::new(
                Backend::open(
                    &fs,
                    path.as_path(),
                    &OpenOptions::new()
                        .write(true)
                        .create(true),
                )
                .await
                .unwrap(),
            ));

            file.write_all(content).await.unwrap();
            fs
        }
    }

    #[tokio::test]
    async fn reads_lower_file() {
        let mut file = Fs::new(OverlayFs::new(
            MemoryFs::new(),
            MemorySource::with_file("/notes.txt", b"lower").await,
        ))
        .root()
        .open_file("/notes.txt")
        .await
        .unwrap();

        assert_eq!(file.read_to_string().await.unwrap(), "lower");
    }

    #[tokio::test]
    async fn write_copies_lower_file_to_upper() {
        let root = Fs::new(OverlayFs::new(
            MemoryFs::new(),
            MemorySource::with_file("/notes.txt", b"lower").await,
        ))
        .root();

        root.options()
            .write(true)
            .truncate(true)
            .open("/notes.txt")
            .await
            .unwrap()
            .write_all(b"upper")
            .await
            .unwrap();

        assert_eq!(
            root.open_file("/notes.txt")
                .await
                .unwrap()
                .read_to_string()
                .await
                .unwrap(),
            "upper"
        );
    }

    #[tokio::test]
    async fn upper_overrides_lower() {
        let mut file = Fs::new(OverlayFs::new(
            MemorySource::with_file("/notes.txt", b"upper").await,
            MemorySource::with_file("/notes.txt", b"lower").await,
        ))
        .root()
        .open_file("/notes.txt")
        .await
        .unwrap();

        assert_eq!(file.read_to_string().await.unwrap(), "upper");
    }

    #[tokio::test]
    async fn remove_lower_creates_whiteout() {
        let root = Fs::new(OverlayFs::new(
            MemoryFs::new(),
            MemorySource::with_file("/notes.txt", b"lower").await,
        ))
        .root();

        root.remove_file("/notes.txt")
            .await
            .unwrap();

        assert!(matches!(
            root.open_file("/notes.txt").await,
            Err(Error::NotFound(path)) if path.to_string_lossy() == "/notes.txt"
        ));
    }

    #[tokio::test]
    async fn whiteout_hides_lower_from_entries() {
        let root = Fs::new(OverlayFs::new(
            MemorySource::with_file("/visible.txt", b"upper").await,
            MemorySource::with_file("/hidden.txt", b"lower").await,
        ))
        .root();

        root.remove_file("/hidden.txt")
            .await
            .unwrap();

        let mut entries = root.entries().await.unwrap();
        let mut names = Vec::new();

        while let Some(entry) = entries.next().await.unwrap() {
            names.push(entry.file_name().to_string_lossy());
        }

        assert_eq!(names, vec!["visible.txt"]);
    }

    #[tokio::test]
    async fn opaque_upper_directory_hides_lower_directory() {
        let mut entries = Fs::new(OverlayFs::new(
            MemorySource::with_file("/workspace/upper.txt", b"upper").await,
            MemorySource::with_file("/workspace/lower.txt", b"lower").await,
        ))
        .root()
        .open_dir("/workspace")
        .await
        .unwrap()
        .entries()
        .await
        .unwrap();
        let mut names = Vec::new();

        while let Some(entry) = entries.next().await.unwrap() {
            names.push(entry.file_name().to_string_lossy());
        }

        assert_eq!(names, vec!["upper.txt"]);
    }

    #[tokio::test]
    async fn overlay_truncate_copies_the_file_up_first() {
        let root = Fs::new(OverlayFs::new(
            MemoryFs::new(),
            MemorySource::with_file("/notes.txt", b"lower content").await,
        ))
        .root();

        root.truncate("/notes.txt", 5)
            .await
            .unwrap();

        assert_eq!(
            root.open_file("/notes.txt")
                .await
                .unwrap()
                .read_to_string()
                .await
                .unwrap(),
            "lower"
        );
    }
}
