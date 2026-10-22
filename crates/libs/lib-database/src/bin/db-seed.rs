//! `mise run db-seed`: adds the stub seed, and generated Units, Transactions and Bill Plans on top, to the already-migrated dev database named by the
//! path in argument one. It never drops or migrates the database, and it is idempotent: running it again leaves the same rows.

use std::process::ExitCode;

use chrono::Utc;
use lib_database::Writer;
use sqlx::sqlite::SqliteConnectOptions;

/// How many generated Units, and how many generated Transactions, go on top of the stub's.
const FAKE_ROWS: u32 = 25;

fn main() -> ExitCode {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: db-seed <path-to-the-dev-database>");
        return ExitCode::FAILURE;
    };
    let result = Writer::spawn(SqliteConnectOptions::new().filename(&path)).and_then(
        |(writer, _failures)| {
            let now = Utc::now();
            lib_database::seed::stub(&writer, now)?;
            lib_database::seed::fake(&writer, now, FAKE_ROWS)
        },
    );
    match result {
        Ok(()) => {
            println!(
                "Seeded {path} with the stub, {FAKE_ROWS} generated Units and Transactions, and generated Bill Plans"
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("Failed to seed {path}: {error}");
            ExitCode::FAILURE
        }
    }
}
