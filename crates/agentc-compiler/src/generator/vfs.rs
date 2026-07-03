// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

/// A simple in-memory representation of a generations file tree.
#[derive(Default, Debug, Clone)]
pub struct VirtualFileSystem {
    files: HashMap<PathBuf, String>,
}

impl VirtualFileSystem {
    /// Create a new, empty virtual file system.
    pub fn new() -> Self {
        Self { files: HashMap::new() }
    }

    /// Insert a file with the given path and content into the virtual file system.
    pub fn insert(&mut self, path: impl Into<PathBuf>, content: impl Into<String>) {
        self.files
            .insert(path.into(), content.into());
    }

    /// Get the content of a file with the given path, if it exists in the virtual file system.
    pub fn get(&self, path: impl AsRef<Path>) -> Option<&str> {
        self.files
            .get(path.as_ref())
            .map(String::as_str)
    }

    /// Remove a file with the given path from the virtual file system, returning its content if it existed.
    pub fn remove(&mut self, path: impl AsRef<Path>) -> Option<String> {
        self.files.remove(path.as_ref())
    }

    /// Check if a file with the given path exists in the virtual file system.
    pub fn contains(&self, path: impl AsRef<Path>) -> bool {
        self.files.contains_key(path.as_ref())
    }

    /// Iterate over all files in the virtual file system, yielding their paths and content.
    pub fn iter(&self) -> impl Iterator<Item = (&Path, &str)> {
        self.files
            .iter()
            .map(|(path, content)| (path.as_path(), content.as_str()))
    }

    /// Get the number of files in the virtual file system.
    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// Check if the virtual file system is empty.
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Merge another virtual file system into this one, overwriting any existing files with the same paths.
    pub fn merge(&mut self, other: Self) {
        self.files.extend(other.files);
    }

    /// Write the files to a target directory on disk, creating any necessary parent directories.
    pub async fn write_to(&self, target_dir: &Path) -> Result<(), std::io::Error> {
        for (path, content) in self.iter() {
            let full_path = target_dir.join(path);

            if let Some(parent) = full_path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }

            tokio::fs::write(full_path, content).await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_get_roundtrip() {
        let mut vfs = VirtualFileSystem::new();
        vfs.insert("src/main.rs", "fn main() {}");
        assert_eq!(vfs.get("src/main.rs"), Some("fn main() {}"));
    }

    #[test]
    fn get_nonexistent_returns_none() {
        let vfs = VirtualFileSystem::new();
        assert_eq!(vfs.get("missing.rs"), None);
    }

    #[test]
    fn contains_returns_true_for_inserted_file() {
        let mut vfs = VirtualFileSystem::new();
        vfs.insert("src/lib.rs", "");
        assert!(vfs.contains("src/lib.rs"));
    }

    #[test]
    fn contains_returns_false_for_missing_file() {
        let vfs = VirtualFileSystem::new();
        assert!(!vfs.contains("src/lib.rs"));
    }

    #[test]
    fn remove_returns_content_and_deletes_entry() {
        let mut vfs = VirtualFileSystem::new();
        vfs.insert("src/lib.rs", "content");
        let removed = vfs.remove("src/lib.rs");
        assert_eq!(removed, Some("content".into()));
        assert!(!vfs.contains("src/lib.rs"));
    }

    #[test]
    fn remove_nonexistent_returns_none() {
        let mut vfs = VirtualFileSystem::new();
        assert_eq!(vfs.remove("ghost.rs"), None);
    }

    #[test]
    fn len_reflects_insertions_and_removals() {
        let mut vfs = VirtualFileSystem::new();
        assert_eq!(vfs.len(), 0);
        vfs.insert("a.rs", "");
        vfs.insert("b.rs", "");
        assert_eq!(vfs.len(), 2);
        vfs.remove("a.rs");
        assert_eq!(vfs.len(), 1);
    }

    #[test]
    fn is_empty_on_new_vfs() {
        assert!(VirtualFileSystem::new().is_empty());
    }

    #[test]
    fn is_empty_false_after_insert() {
        let mut vfs = VirtualFileSystem::new();
        vfs.insert("x.rs", "");
        assert!(!vfs.is_empty());
    }

    #[test]
    fn merge_combines_files() {
        let mut base = VirtualFileSystem::new();
        base.insert("a.rs", "a");

        let mut other = VirtualFileSystem::new();
        other.insert("b.rs", "b");

        base.merge(other);
        assert_eq!(base.get("a.rs"), Some("a"));
        assert_eq!(base.get("b.rs"), Some("b"));
        assert_eq!(base.len(), 2);
    }

    #[test]
    fn merge_overwrites_existing_path() {
        let mut base = VirtualFileSystem::new();
        base.insert("a.rs", "original");

        let mut other = VirtualFileSystem::new();
        other.insert("a.rs", "overwritten");

        base.merge(other);
        assert_eq!(base.get("a.rs"), Some("overwritten"));
    }

    #[test]
    fn iter_yields_all_entries() {
        let mut vfs = VirtualFileSystem::new();
        vfs.insert("a.rs", "a");
        vfs.insert("b.rs", "b");

        let mut paths: Vec<String> = vfs
            .iter()
            .map(|(p, _)| p.to_string_lossy().to_string())
            .collect();
        paths.sort();
        assert_eq!(paths, vec!["a.rs", "b.rs"]);
    }
}
