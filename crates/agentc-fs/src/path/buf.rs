// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{
    borrow::Cow,
    fmt::{Debug, Display, Formatter, Result as FmtResult},
    ops::Deref,
    path::{MAIN_SEPARATOR, Path as HostPath, PathBuf as HostPathBuf},
};

use typed_path::{UnixPath, UnixPathBuf};

use crate::{
    errors::Error,
    path::{Component, ComponentsIter},
};

#[repr(transparent)]
#[derive(Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Path {
    inner: [u8],
}

impl Path {
    pub(crate) fn from_bytes(bytes: &[u8]) -> &Self {
        unsafe { &*(bytes as *const [u8] as *const Self) }
    }

    pub fn is_absolute(&self) -> bool {
        UnixPath::new(self.as_bytes()).is_absolute()
    }

    pub fn is_relative(&self) -> bool {
        UnixPath::new(self.as_bytes()).is_relative()
    }

    pub fn parent(&self) -> Option<&Path> {
        UnixPath::new(self.as_bytes())
            .parent()
            .map(|path| Path::from_bytes(path.as_bytes()))
    }

    pub fn file_name(&self) -> Option<Component> {
        UnixPath::new(self.as_bytes())
            .file_name()
            .map(Component::new)
    }

    pub fn file_stem(&self) -> Option<Component> {
        UnixPath::new(self.as_bytes())
            .file_stem()
            .map(Component::new)
    }

    pub fn extension(&self) -> Option<Component> {
        UnixPath::new(self.as_bytes())
            .extension()
            .map(Component::new)
    }

    pub fn components(&self) -> ComponentsIter<'_> {
        ComponentsIter::new(UnixPath::new(self.as_bytes()).components())
    }

    pub fn ancestors(&self) -> AncestorsIter<'_> {
        let mut paths = Vec::new();
        let mut path = Some(self);

        while let Some(current) = path {
            paths.push(current);
            path = current.parent();
        }

        AncestorsIter { paths, index: 0 }
    }

    pub fn display(&self) -> PathDisplay<'_> {
        PathDisplay { path: self }
    }

    pub fn to_string_lossy(&self) -> String {
        String::from_utf8_lossy(self.as_bytes()).into_owned()
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.inner
    }
}

impl Debug for Path {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.debug_tuple("Path")
            .field(&self.to_string_lossy())
            .finish()
    }
}

impl Display for Path {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.to_string_lossy())
    }
}

#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PathBuf {
    inner: Vec<u8>,
}

impl PathBuf {
    pub fn parse(path: impl AsRef<[u8]>) -> Result<Self, Error> {
        if path.as_ref().contains(&0) {
            return Err(Error::invalid_path("path contains a NUL byte"));
        }

        Ok(PathBuf { inner: path.as_ref().to_vec() })
    }

    pub fn root() -> Self {
        PathBuf { inner: b"/".to_vec() }
    }

    pub fn as_path(&self) -> &Path {
        self
    }

    pub fn is_absolute(&self) -> bool {
        self.as_path().is_absolute()
    }

    pub fn is_relative(&self) -> bool {
        self.as_path().is_relative()
    }

    pub fn parent(&self) -> Option<&Path> {
        self.as_path().parent()
    }

    pub fn file_name(&self) -> Option<Component> {
        self.as_path().file_name()
    }

    pub fn file_stem(&self) -> Option<Component> {
        self.as_path().file_stem()
    }

    pub fn extension(&self) -> Option<Component> {
        self.as_path().extension()
    }

    pub fn components(&self) -> ComponentsIter<'_> {
        self.as_path().components()
    }

    pub fn ancestors(&self) -> AncestorsIter<'_> {
        self.as_path().ancestors()
    }

    pub fn join(&self, path: impl IntoPathBuf) -> Result<Self, Error> {
        let mut inner = UnixPathBuf::from(self.as_bytes());
        inner.push(
            path.into_path_buf()?
                .as_path()
                .as_bytes(),
        );

        PathBuf::parse(inner.as_bytes())
    }

    pub fn normalize(&self) -> Result<Self, Error> {
        let mut normalized = Vec::new();
        let absolute = self.is_absolute();

        for component in self.components() {
            match component.as_bytes() {
                b"/" => {}
                b"." => {}
                _ => normalized.push(component),
            }
        }

        let mut bytes = Vec::new();

        if absolute {
            bytes.push(b'/');
        }

        for component in normalized {
            if bytes.len() > usize::from(absolute) {
                bytes.push(b'/');
            }

            bytes.extend_from_slice(component.as_bytes());
        }

        if bytes.is_empty() && absolute {
            bytes.push(b'/');
        }

        PathBuf::parse(bytes)
    }

    pub fn display(&self) -> PathDisplay<'_> {
        self.as_path().display()
    }

    pub fn to_string_lossy(&self) -> String {
        self.as_path().to_string_lossy()
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.inner
    }

    pub fn from_host_path(path: impl AsRef<HostPath>) -> Result<Self, Error> {
        PathBuf::parse(
            path.as_ref()
                .to_string_lossy()
                .replace(MAIN_SEPARATOR, "/"),
        )
    }

    pub fn to_host_path(&self) -> Result<HostPathBuf, Error> {
        Ok(HostPathBuf::from(self.to_string_lossy()))
    }
}

impl Debug for PathBuf {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        Debug::fmt(self.as_path(), f)
    }
}

impl Display for PathBuf {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        Display::fmt(self.as_path(), f)
    }
}

impl Deref for PathBuf {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        Path::from_bytes(&self.inner)
    }
}

impl AsRef<Path> for PathBuf {
    fn as_ref(&self) -> &Path {
        self.as_path()
    }
}

impl From<PathBuf> for Vec<u8> {
    fn from(path: PathBuf) -> Self {
        path.inner
    }
}

impl From<&Path> for PathBuf {
    fn from(path: &Path) -> Self {
        PathBuf { inner: path.as_bytes().to_vec() }
    }
}

pub trait IntoPathBuf {
    fn into_path_buf(self) -> Result<PathBuf, Error>;
}

impl IntoPathBuf for &str {
    fn into_path_buf(self) -> Result<PathBuf, Error> {
        PathBuf::parse(self)
    }
}

impl IntoPathBuf for String {
    fn into_path_buf(self) -> Result<PathBuf, Error> {
        PathBuf::parse(self)
    }
}

impl IntoPathBuf for &[u8] {
    fn into_path_buf(self) -> Result<PathBuf, Error> {
        PathBuf::parse(self)
    }
}

impl IntoPathBuf for Vec<u8> {
    fn into_path_buf(self) -> Result<PathBuf, Error> {
        PathBuf::parse(self)
    }
}

impl IntoPathBuf for PathBuf {
    fn into_path_buf(self) -> Result<PathBuf, Error> {
        Ok(self)
    }
}

impl IntoPathBuf for &PathBuf {
    fn into_path_buf(self) -> Result<PathBuf, Error> {
        Ok(self.clone())
    }
}

impl IntoPathBuf for &Path {
    fn into_path_buf(self) -> Result<PathBuf, Error> {
        Ok(PathBuf::from(self))
    }
}

impl<'a> From<&'a Path> for Cow<'a, [u8]> {
    fn from(path: &'a Path) -> Self {
        Cow::Borrowed(path.as_bytes())
    }
}

pub struct AncestorsIter<'a> {
    paths: Vec<&'a Path>,
    index: usize,
}

impl Iterator for AncestorsIter<'_> {
    type Item = PathBuf;

    fn next(&mut self) -> Option<Self::Item> {
        match self.paths.get(self.index) {
            Some(path) => {
                self.index += 1;

                Some(PathBuf { inner: path.as_bytes().to_vec() })
            }
            None => None,
        }
    }
}

pub struct PathDisplay<'a> {
    path: &'a Path,
}

impl Display for PathDisplay<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        Display::fmt(self.path, f)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path as HostPath;

    use crate::path::{IntoPathBuf, PathBuf};

    #[test]
    fn parse_tracks_absolute_and_relative_paths() {
        assert!(
            PathBuf::parse("/workspace/file.txt")
                .unwrap()
                .is_absolute()
        );
        assert!(
            PathBuf::parse("workspace/file.txt")
                .unwrap()
                .is_relative()
        );
    }

    #[test]
    fn parse_rejects_nul_bytes() {
        assert!(PathBuf::parse(b"/workspace/\0/file.txt").is_err());
    }

    #[test]
    fn normalize_removes_repeated_separators_and_current_dir() {
        assert_eq!(
            PathBuf::parse("/workspace//./file.txt")
                .unwrap()
                .normalize()
                .unwrap()
                .to_string_lossy(),
            "/workspace/file.txt"
        );
    }

    #[test]
    fn parent_and_file_name_inspect_path() {
        let path = PathBuf::parse("/workspace/file.txt").unwrap();

        assert_eq!(path.parent().unwrap().to_string_lossy(), "/workspace");
        assert_eq!(
            path.file_name()
                .unwrap()
                .to_string_lossy(),
            "file.txt"
        );
    }

    #[test]
    fn join_combines_relative_child_paths() {
        assert_eq!(
            PathBuf::parse("/workspace")
                .unwrap()
                .join("file.txt")
                .unwrap()
                .to_string_lossy(),
            "/workspace/file.txt"
        );
    }

    #[test]
    fn host_conversion_is_explicit() {
        assert_eq!(
            PathBuf::from_host_path(HostPath::new("workspace/file.txt"))
                .unwrap()
                .to_string_lossy(),
            "workspace/file.txt"
        );
    }

    #[test]
    fn parent_components_are_preserved_for_authority_resolution() {
        assert_eq!(
            PathBuf::parse("../workspace/file.txt")
                .unwrap()
                .components()
                .map(|component| component.to_string_lossy())
                .collect::<Vec<_>>(),
            vec!["..", "workspace", "file.txt"]
        );
    }

    #[test]
    fn into_path_buf_accepts_common_inputs() {
        assert_eq!(
            "workspace/file.txt"
                .into_path_buf()
                .unwrap()
                .to_string_lossy(),
            "workspace/file.txt"
        );
    }
}
