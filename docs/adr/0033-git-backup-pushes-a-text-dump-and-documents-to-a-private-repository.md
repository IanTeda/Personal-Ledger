# Git Backup pushes a text dump of the Ledger and its Documents to a private repository

Settings › Backup (Desktop 16l/16t, TUI-036) adds a **Git Backup**: a Client commits the Ledger to a remote Git repository on a schedule, so a user has an off-device copy with history that doesn't depend on the Sync Server. The mockups said it "commits the store". We decided instead that each commit holds a **deterministic text dump** of the Ledger (one file per table, rows ordered by `RowID`, a fixed column order and value encoding) plus the **Document files**, tracked through Git LFS. We also decided the push is **refused unless the remote is a private repository**, with no client-side encryption.

A text dump makes the Git history readable and diffable. Each commit changes only the rows that changed, and Git's delta compression keeps the repository small. Committing the raw SQLite file would add a new binary blob on every push, with history nobody can read. Document files (PDFs and images) are binary and can be large, so they go through LFS instead of bloating the repository. They are content-addressed by file, so an unchanged Document is never pushed twice.

Sending the whole Ledger off the device meets one of [ADR-0011](0011-os-level-encryption-not-sqlcipher.md)'s revisit triggers ("database files backed up or synced to less-trusted storage as a supported feature"). We reconsidered and kept plaintext, protected by the repository's privacy. Encrypting before the push would make the remote history undiffable, which defeats the reason for a text dump. It would also tie recovery to a key that is easily lost along with the device the backup exists to replace. The host and anyone with access to the repository are trusted, the same way the user already trusts their own disk.

## Considered Options

- **Raw SQLite file.** Rejected. It is trivial to build and restore, but the repository grows by a full copy on every push and the history can't be read.
- **Raw SQLite file through LFS.** Rejected. It keeps the repository small but still has no readable history, and every push uploads the whole file.
- **Client-side encryption (for example `age`, key in the OS keychain), always or opt-in.** Rejected for now, for the reasons above. If backups to a host the user doesn't control become a supported use, this is the change to revisit, and ADR-0011's SQLCipher path doesn't help here because the dump isn't the database file.

## Consequences

- **The dump covers Ledger data only:** everything that syncs as Change Sets, plus Documents. It never includes Configuration, OS-keychain secrets (access tokens, API keys), the Sync Server's own tables (`change_sets`, `sync_users`), logs or session state.
- **The dump format is a contract.** It needs a versioned writer and a matching importer, because restore means "create a new Ledger from a dump". Changing the format must keep old dumps importable. The importer isn't designed yet.
- **Git Backup is Configuration on one Client**, never synced: the remote, branch, authentication method, schedule and commit-message text. The SSH key path or access token lives in the OS keychain. If two Clients push to the same branch, a non-fast-forward push is reported as a failure and never forced. Use one branch per Client.
- **Privacy is checked, not assumed.** Test (a dry-run push) and every scheduled push check the repository's visibility through the host's API (GitHub, GitLab and similar). Where visibility can't be queried (for example a self-hosted bare repository over SSH), the user confirms once that the remote is private, and that confirmation is stored with the Configuration. A remote that reports public is refused.
- **The remote must support LFS.** Test checks for it and fails with that reason in plain words.
- **Schedule** is one of after changes (after 5 minutes of quiet, at most hourly), hourly, daily or manual. Each commit message is a fixed `yyyy-mm-dd hh:mm ·` prefix followed by user text. The first push commits the whole Ledger and every Document.
