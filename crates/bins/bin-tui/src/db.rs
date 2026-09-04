//! Shared connection to the TUI's own local embedded-SQLite Ledger store — every screen
//! that reads or writes real data (starting with the feasibility cycle's Categories demo,
//! now every Concept-cycle entity screen built on this skeleton) goes through the same
//! database file and the same Client migration set (`docs/adr/0003-...`'s async-from-the-
//! start model: each screen still opens its own connection per `init()`, but they all agree
//! on where the file lives and which migrations apply).

use std::path::PathBuf;

/// The Client's own SQLite file, in the OS temp directory so `cargo run` never leaves a
/// stray file in the repo working directory (also `.gitignore`d via `*.sqlite`).
fn database_url() -> String {
    let path: PathBuf = std::env::temp_dir().join("personal-ledger-tui-feasibility-demo.sqlite");
    format!("sqlite://{}?mode=rwc", path.display())
}

/// Connect to the Client's local Ledger store and apply the Client migration set
/// (`migrations/client`), returning a ready-to-use pool.
pub async fn connect() -> lib_database::DatabaseResult<sqlx::SqlitePool> {
    connect_to(database_url()).await
}

/// Same as [`connect`], against an explicit database URL — split out so tests can point it
/// at an isolated, throwaway SQLite file instead of the shared one.
pub async fn connect_to(url: String) -> lib_database::DatabaseResult<sqlx::SqlitePool> {
    let config = lib_database::DatabaseConfig {
        url,
        ..lib_database::DatabaseConfig::default()
    };
    let connection = lib_database::DatabaseConnection::new(config).await?;
    let pool = connection.into_pool();

    sqlx::migrate!("../../libs/lib-database/migrations/client")
        .run(&pool)
        .await?;

    Ok(pool)
}
