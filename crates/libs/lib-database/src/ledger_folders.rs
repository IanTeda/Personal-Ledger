//! The disk behind `lib_shell`'s [`LedgerFolders`] port: the folders the Desktop's file explorer
//! and the TUI's Data folder picker walk. It lives here, not in `lib-shell`, because `lib-shell`
//! is I/O-free (a feature crate holds no `std::fs`), and not in each bin, because both Clients
//! need the same adapter. `lib-database` is the one crate both already depend on that owns the
//! Ledger file's I/O (`storage`, `LedgerCreationAdapter`) and already depends on `lib-shell`, so
//! it adds no dependency edge.

use std::path::Path;

use lib_shell::{EntryKind, FolderEntry, FolderUnreadable, LedgerFolders};

/// The explorer's folders, read from the disk. An entry whose metadata can't be read (a race
/// with its deletion) is left out.
#[derive(Debug, Clone, Copy, Default)]
pub struct FsLedgerFolders;

impl LedgerFolders for FsLedgerFolders {
    fn read_folder(&self, folder: &Path) -> Result<Vec<FolderEntry>, FolderUnreadable> {
        let read_dir =
            std::fs::read_dir(folder).map_err(|error| FolderUnreadable(error.to_string()))?;
        Ok(read_dir
            .flatten()
            .filter_map(|entry| {
                let path = entry.path();
                let metadata = entry.metadata().ok()?;
                let is_folder = metadata.is_dir();
                Some(FolderEntry {
                    name: entry.file_name().to_string_lossy().into_owned(),
                    kind: EntryKind::of(&path, is_folder),
                    size: (!is_folder).then_some(metadata.len()),
                    modified: metadata.modified().ok(),
                    path,
                })
            })
            .collect())
    }

    /// A broken link is there too: `symlink_metadata` doesn't follow it.
    fn exists(&self, path: &Path) -> bool {
        std::fs::symlink_metadata(path).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_disk_adapter_reads_a_real_folder_and_says_what_is_there() {
        let dir = tempfile::tempdir().expect("tempdir should be creatable");
        std::fs::create_dir(dir.path().join("archive")).expect("a folder");
        std::fs::write(dir.path().join("teda.pldb"), b"stub").expect("a file");
        std::fs::write(dir.path().join("notes.csv"), b"stub").expect("a file");

        let mut entries = FsLedgerFolders
            .read_folder(dir.path())
            .expect("the folder reads");
        entries.sort_by(|a, b| a.name.cmp(&b.name));

        let read: Vec<_> = entries
            .iter()
            .map(|entry| (entry.name.as_str(), entry.kind, entry.size))
            .collect();
        assert_eq!(
            read,
            [
                ("archive", EntryKind::Folder, None),
                ("notes.csv", EntryKind::Other, Some(4)),
                ("teda.pldb", EntryKind::Ledger, Some(4)),
            ]
        );
        assert!(entries.iter().all(|entry| entry.modified.is_some()));
        assert!(FsLedgerFolders.exists(&dir.path().join("teda.pldb")));
        assert!(!FsLedgerFolders.exists(&dir.path().join("teda.pldb-wal")));
        assert!(
            FsLedgerFolders
                .read_folder(&dir.path().join("missing"))
                .is_err()
        );
    }

    #[test]
    fn a_broken_link_exists() {
        #[cfg(unix)]
        {
            let dir = tempfile::tempdir().expect("tempdir should be creatable");
            let link = dir.path().join("broken");
            std::os::unix::fs::symlink(dir.path().join("gone"), &link).expect("a link");
            assert!(FsLedgerFolders.exists(&link), "a broken link is there too");
        }
    }
}
