//! In-memory ports for tests: [`InMemoryFiles`], a file system of the files it is given.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::protocols::{DocumentFiles, resolve_typed};

/// A file system holding only the files it is given, with a fixed home and working directory.
#[derive(Debug, Clone)]
pub struct InMemoryFiles {
    pub home: PathBuf,
    pub working_dir: PathBuf,
    /// Each file on "disk", with its size in bytes.
    pub files: HashMap<PathBuf, u64>,
}

impl Default for InMemoryFiles {
    fn default() -> Self {
        Self {
            home: PathBuf::from("/home/user"),
            working_dir: PathBuf::from("/work"),
            files: HashMap::new(),
        }
    }
}

impl InMemoryFiles {
    /// Adds a file of `bytes` at `path`.
    pub fn with(mut self, path: impl Into<PathBuf>, bytes: u64) -> Self {
        self.files.insert(path.into(), bytes);
        self
    }
}

impl DocumentFiles for InMemoryFiles {
    fn resolve(&self, typed: &str) -> PathBuf {
        resolve_typed(typed, Some(&self.home), Some(&self.working_dir))
    }

    fn exists(&self, path: &Path) -> bool {
        self.files.contains_key(path)
    }

    fn size(&self, path: &Path) -> Option<u64> {
        self.files.get(path).copied()
    }
}
