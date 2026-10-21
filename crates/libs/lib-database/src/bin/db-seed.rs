//! `mise run db-seed`: writes the stub seed into the migrated dev database named by the path in
//! argument one. The task drops, creates and migrates it first; this only writes rows.

use std::process::ExitCode;

use chrono::Utc;
use lib_database::Writer;
use sqlx::sqlite::SqliteConnectOptions;

fn main() -> ExitCode {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: db-seed <path-to-the-dev-database>");
        return ExitCode::FAILURE;
    };
    let result = Writer::spawn(SqliteConnectOptions::new().filename(&path))
        .and_then(|(writer, _failures)| lib_database::seed::stub(&writer, Utc::now()));
    match result {
        Ok(()) => {
            println!("Seeded {path} with the stub");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("Failed to seed {path}: {error}");
            ExitCode::FAILURE
        }
    }
}
