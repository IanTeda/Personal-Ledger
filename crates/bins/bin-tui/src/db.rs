//! Shared connection to the TUI's own local embedded-SQLite Ledger store: one agreed
//! database file and one Client migration set (`migrations/client`), so every caller that
//! reads or writes real data opens its own pool but agrees on where the file lives and
//! which migrations apply.
//!
//! Currently unreferenced. Its only callers were the retired `app`/`screen` stack (ADR-0013);
//! the `view/` layer that replaced it renders in-memory mock data, and wiring persistence
//! back in is a later phase. Kept because it is neutral plumbing, not part of the retired
//! screen architecture — the first `View` that needs real data starts here.

use std::path::PathBuf;

/// The Client's own SQLite file, in the OS temp directory so `cargo run` never leaves a
/// stray file in the repo working directory (also `.gitignore`d via `*.sqlite`).
fn database_url() -> String {
    let path: PathBuf = std::env::temp_dir().join("personal-ledger-tui-feasibility-demo.sqlite");
    format!("sqlite://{}?mode=rwc", path.display())
}

/// Connect to the Client's local Ledger store and apply the Client migration set
/// (`migrations/client`), returning a ready-to-use pool.
pub async fn connect() -> crate::Result<sqlx::SqlitePool> {
    connect_to(database_url()).await
}

/// Same as [`connect`], against an explicit database URL — split out so tests can point it
/// at an isolated, throwaway SQLite file instead of the shared one.
pub async fn connect_to(url: String) -> crate::Result<sqlx::SqlitePool> {
    let connection = lib_database::DatabaseConnection::new(url).await?;
    let pool = connection.into_pool();

    sqlx::migrate!("../../libs/lib-database/migrations/client")
        .run(&pool)
        .await
        .map_err(lib_database::Error::from)?;

    Ok(pool)
}
