//! `mise run db-seed`: adds the stub seed, and generated Units on top, to the already-migrated dev database named by the
//! path in argument one. It never drops or migrates the database, and it is idempotent: running it again leaves the same rows.

use std::process::ExitCode;

use chrono::Utc;
use lib_database::Writer;
use sqlx::sqlite::SqliteConnectOptions;

/// How many generated Units go on top of the stub's.
const FAKE_UNITS: u32 = 25;

fn main() -> ExitCode {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: db-seed <path-to-the-dev-database>");
        return ExitCode::FAILURE;
    };
    let result = Writer::spawn(SqliteConnectOptions::new().filename(&path)).and_then(
        |(writer, _failures)| {
            let now = Utc::now();
            lib_database::seed::stub(&writer, now)?;
            lib_database::seed::fake(&writer, now, FAKE_UNITS)
        },
    );
    match result {
        Ok(()) => {
            println!("Seeded {path} with the stub and {FAKE_UNITS} generated Units");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("Failed to seed {path}: {error}");
            ExitCode::FAILURE
        }
    }
}
