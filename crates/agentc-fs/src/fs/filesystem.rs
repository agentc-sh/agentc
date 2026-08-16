// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::sync::Arc;

use crate::{
    backend::Backend,
    errors::Error,
    fs::{
        builder::FsBuilder,
        dir::Dir,
        namespace::{Mount, MountKind, Namespace},
    },
    memory::MemoryFs,
    path::{IntoPathBuf, PathBuf},
};

#[derive(Clone)]
pub struct Fs {
    pub(crate) namespace: Arc<Namespace>,
}

impl Fs {
    pub fn builder() -> FsBuilder {
        FsBuilder::new()
    }

    pub fn new(backend: impl Backend) -> Self {
        Fs::from_namespace(Namespace::single_root(Arc::new(backend)))
    }

    pub fn memory() -> Self {
        Fs::new(MemoryFs::new())
    }

    pub(crate) fn from_namespace(namespace: Namespace) -> Self {
        Fs { namespace: Arc::new(namespace) }
    }

    pub fn root(&self) -> Dir {
        Dir::new(self.clone(), PathBuf::root(), PathBuf::root())
    }

    pub fn mount(&self, path: impl IntoPathBuf, backend: impl Backend) -> Result<(), Error> {
        self.namespace.insert(Mount {
            path: FsBuilder::mount_path(path)?,
            kind: MountKind::Directory,
            backend: Arc::new(backend),
            dev: 0,
        });

        Ok(())
    }

    pub fn mount_fs(&self, path: impl IntoPathBuf, other: Fs) -> Result<(), Error> {
        self.namespace.insert(Mount {
            path: FsBuilder::mount_path(path)?,
            kind: MountKind::Directory,
            backend: other.namespace,
            dev: 0,
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        backend::Backend,
        fs::{File, OpenOptions, filesystem::Fs},
        memory::MemoryFs,
        path::PathBuf,
    };

    struct MemorySource;

    impl MemorySource {
        async fn with_file(path: &str, content: &[u8]) -> MemoryFs {
            let fs = MemoryFs::new();
            let path = PathBuf::parse(path).unwrap();
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
    async fn a_dir_captured_before_a_mount_sees_the_mounted_content() {
        let fs = Fs::builder()
            .mount("/", MemoryFs::new())
            .build()
            .unwrap();
        let root = fs.root();

        fs.clone()
            .mount_fs(
                "/skills",
                Fs::builder()
                    .mount("/", MemorySource::with_file("/reference.md", b"grafted").await)
                    .build()
                    .unwrap(),
            )
            .unwrap();

        let mut file = root
            .open_file("/skills/reference.md")
            .await
            .unwrap();

        assert_eq!(file.read_to_string().await.unwrap(), "grafted");
    }

    #[tokio::test]
    async fn a_runtime_mount_accepts_writes() {
        let fs = Fs::builder()
            .mount("/", MemoryFs::new())
            .build()
            .unwrap();

        fs.mount("/data", MemoryFs::new())
            .unwrap();

        let root = fs.root();
        let mut file = root
            .options()
            .write(true)
            .create(true)
            .open("/data/notes.txt")
            .await
            .unwrap();

        file.write_all(b"runtime")
            .await
            .unwrap();
        file.flush().await.unwrap();

        let mut file = root
            .open_file("/data/notes.txt")
            .await
            .unwrap();

        assert_eq!(file.read_to_string().await.unwrap(), "runtime");
    }

    #[tokio::test]
    async fn a_builder_mount_fs_composes_an_existing_filesystem() {
        let mut file = Fs::builder()
            .mount("/", MemoryFs::new())
            .mount_fs(
                "/data",
                Fs::builder()
                    .mount("/", MemorySource::with_file("/notes.txt", b"built").await)
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap()
            .root()
            .open_file("/data/notes.txt")
            .await
            .unwrap();

        assert_eq!(file.read_to_string().await.unwrap(), "built");
    }
}
