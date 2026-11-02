//! The Desktop's answer to `lib_documents`' [`DocumentFiles`] port: the real file system, through
//! `std::fs`, with `~` read from the platform's home directory.

use std::path::{Path, PathBuf};

use crate::documents::{DocumentFiles, protocols::resolve_typed};

/// The disk the Desktop runs on.
#[derive(Debug, Clone, Copy, Default)]
pub struct DiskFiles;

impl DocumentFiles for DiskFiles {
    fn resolve(&self, typed: &str) -> PathBuf {
        let home = dirs::home_dir();
        let working_dir = std::env::current_dir().ok();
        resolve_typed(typed, home.as_deref(), working_dir.as_deref())
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn size(&self, path: &Path) -> Option<u64> {
        std::fs::metadata(path).ok().map(|meta| meta.len())
    }
}
