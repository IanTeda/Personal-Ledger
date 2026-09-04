//! The embedded-SQLite feasibility demo (FC-TUI-005): connects the TUI to a real
//! `lib-database`/`lib-core`-backed SQLite store (not dummy data), seeding one category if
//! the store is empty and then listing every category — a real write and a real read through
//! the same in-process path a client uses per FR.39. Runs the connect/migrate/seed/read
//! sequence as a background task from `init()`, reporting back through the async action
//! channel ADR-0003 (`docs/adr/0003-hybrid-tea-component-tui-architecture.md`) committed to.

use std::path::PathBuf;

use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Color, Style},
    widgets::{Block, Cell, Paragraph, Row, Table},
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{action::Action, screen::Screen};

/// The demo's own SQLite file, in the OS temp directory so `cargo run` never leaves a stray
/// file in the repo working directory (it's also `.gitignore`d regardless, via `*.sqlite`).
fn demo_database_url() -> String {
    let path: PathBuf = std::env::temp_dir().join("personal-ledger-tui-feasibility-demo.sqlite");
    format!("sqlite://{}?mode=rwc", path.display())
}

/// What the screen currently knows about the real SQLite data.
enum Status {
    Loading,
    Loaded(Vec<lib_database::Categories>),
    Failed(String),
}

/// Demonstrates the TUI operating end-to-end against a real embedded SQLite store.
pub struct CategoriesScreen {
    status: Status,
}

impl CategoriesScreen {
    /// Starts in the `Loading` state; the real data arrives asynchronously via `init()`.
    pub fn new() -> Self {
        Self {
            status: Status::Loading,
        }
    }

    /// Connects to the demo database, applies `lib-database`'s own migrations, seeds one
    /// category if the store is empty (a real write), then reads every category back
    /// (a real read) — proving the embedded-SQLite path works end-to-end through a real
    /// client, not a mock.
    async fn load() -> lib_database::DatabaseResult<Vec<lib_database::Categories>> {
        Self::load_from(demo_database_url()).await
    }

    /// Same as [`Self::load`], against an explicit database URL — split out so tests can
    /// point it at an isolated, throwaway SQLite file instead of the shared demo one.
    async fn load_from(url: String) -> lib_database::DatabaseResult<Vec<lib_database::Categories>> {
        let config = lib_database::DatabaseConfig {
            url,
            ..lib_database::DatabaseConfig::default()
        };
        let connection = lib_database::DatabaseConnection::new(config).await?;
        let pool = connection.pool();

        sqlx::migrate!("../../libs/lib-database/migrations")
            .run(pool)
            .await?;

        if lib_database::Categories::find_all(pool).await?.is_empty() {
            let seed = lib_database::Categories {
                id: lib_core::RowID::new(),
                code: "DEM.SEE.D01".to_string(),
                name: "Demo Seed Category".to_string(),
                description: Some(
                    "Inserted by the TUI's embedded-SQLite feasibility demo (FC-TUI-005)"
                        .to_string(),
                ),
                url_slug: Some(lib_core::UrlSlug::from("demo-seed-category")),
                category_type: lib_core::CategoryTypes::Expense,
                color: Some(lib_core::HexColor::from_rgb(0x4a, 0x9e, 0xd6)),
                icon: None,
                is_active: true,
                created_on: chrono::Utc::now(),
                updated_on: chrono::Utc::now(),
            };
            seed.insert(pool).await?;
        }

        lib_database::Categories::find_all(pool).await
    }
}

impl Screen for CategoriesScreen {
    fn init(&mut self, action_tx: UnboundedSender<Action>) {
        tokio::spawn(async move {
            let action = match Self::load().await {
                Ok(categories) => Action::CategoriesLoaded(categories),
                Err(err) => Action::CategoriesLoadFailed(err.to_string()),
            };
            // The app may have already exited; a dropped receiver is not an error here.
            let _ = action_tx.send(action);
        });
    }

    fn update(&mut self, action: &Action) {
        match action {
            Action::CategoriesLoaded(categories) => {
                self.status = Status::Loaded(categories.clone());
            }
            Action::CategoriesLoadFailed(message) => {
                self.status = Status::Failed(message.clone());
            }
            _ => {}
        }
    }

    fn title(&self) -> &'static str {
        "Live Categories (SQLite)"
    }

    fn view(&self, frame: &mut Frame, area: Rect) {
        let title = " Live categories demo — FC-TUI-005 (real SQLite data) ";
        match &self.status {
            Status::Loading => {
                let paragraph = Paragraph::new("Connecting to embedded SQLite store...")
                    .block(Block::bordered().title(title));
                frame.render_widget(paragraph, area);
            }
            Status::Failed(message) => {
                let paragraph = Paragraph::new(format!("Failed to load: {message}"))
                    .style(Style::default().fg(Color::Red))
                    .block(Block::bordered().title(title));
                frame.render_widget(paragraph, area);
            }
            Status::Loaded(categories) => {
                let header = Row::new(["Code", "Name", "Type", "Active"])
                    .style(Style::default().fg(Color::Yellow));
                let rows = categories.iter().map(|category| {
                    Row::new([
                        Cell::from(category.code.clone()),
                        Cell::from(category.name.clone()),
                        Cell::from(category.category_type.as_str()),
                        Cell::from(if category.is_active { "yes" } else { "no" }),
                    ])
                });
                let widths = [
                    Constraint::Length(14),
                    Constraint::Length(24),
                    Constraint::Length(10),
                    Constraint::Length(8),
                ];
                let table = Table::new(rows, widths)
                    .header(header)
                    .column_spacing(1)
                    .block(Block::bordered().title(format!(
                        " Live categories demo — FC-TUI-005 (real SQLite data, {} row{}) ",
                        categories.len(),
                        if categories.len() == 1 { "" } else { "s" }
                    )));
                frame.render_widget(table, area);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;

    fn render(status: Status) {
        let screen = CategoriesScreen { status };
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");

        terminal
            .draw(|frame| screen.view(frame, frame.area()))
            .expect("rendering the categories screen should not error");
    }

    #[test]
    fn renders_loading_without_panicking() {
        render(Status::Loading);
    }

    #[test]
    fn renders_failed_without_panicking() {
        render(Status::Failed("connection refused".to_string()));
    }

    #[test]
    fn renders_loaded_without_panicking() {
        render(Status::Loaded(vec![lib_database::Categories {
            id: lib_core::RowID::new(),
            code: "TES.TCO.DE1".to_string(),
            name: "Test Category".to_string(),
            description: None,
            url_slug: None,
            category_type: lib_core::CategoryTypes::Expense,
            color: None,
            icon: None,
            is_active: true,
            created_on: chrono::Utc::now(),
            updated_on: chrono::Utc::now(),
        }]));
    }

    /// Exercises the real embedded-SQLite path end-to-end against an isolated, throwaway
    /// database file: connect, migrate (a write), seed-if-empty (a write), and read back —
    /// proving FC-TUI-005 against actual `lib-database`/`lib-core` code, not a mock. Runs
    /// `load_from` twice against the same file to confirm the seed step is idempotent.
    #[tokio::test]
    async fn load_from_seeds_once_and_reads_back() {
        let path = std::env::temp_dir().join(format!(
            "personal-ledger-tui-test-{}.sqlite",
            lib_core::RowID::new()
        ));
        let url = format!("sqlite://{}?mode=rwc", path.display());

        let first = CategoriesScreen::load_from(url.clone())
            .await
            .expect("first load should connect, migrate, seed, and read successfully");
        assert_eq!(
            first.len(),
            1,
            "a fresh database should be seeded with one category"
        );
        assert_eq!(first[0].code, "DEM.SEE.D01");

        let second = CategoriesScreen::load_from(url)
            .await
            .expect("second load should connect and read successfully");
        assert_eq!(
            second.len(),
            1,
            "loading an already-seeded database should not re-seed"
        );
        assert_eq!(second[0].id, first[0].id);

        let _ = std::fs::remove_file(&path);
    }
}
