//! Personal Ledger desktop app -- feasibility-cycle demo.
//!
//! Demonstrates chart rendering in `GPUI` (ADR-0007) against dummy data, per
//! FC-DESKTOP-002, using `gpui-component`'s chart widgets. `gpui-component` ships
//! `LineChart`, `PieChart` (with a real donut via `inner_radius`), `CandlestickChart`,
//! `BarChart`, and a `table` widget natively -- covering every FC-DESKTOP-002/003 chart
//! type and the table demo, so this feasibility cycle standardises on it rather than
//! hand-rolled `Canvas` drawing or a general-purpose plotting-library adapter (see issue
//! #28's discussion for the comparison against `gpui-d3rs` and `ruviz-gpui`).
//!
//! Multiple screens are switched via `gpui-component`'s own `TabBar`, following the
//! precedent set by the TUI map: the second screen (doughnut) is where tab navigation
//! was introduced there too.
//!
//! The divergent (diverging bar) chart is the one type `gpui-component` doesn't cover --
//! its `BarChart` always anchors bars to the bottom of the plot area rather than a
//! zero-value baseline, so negative values don't diverge from a centre line. No crate for
//! this chart type was found for any framework surveyed (see
//! `docs/research/desktop-gui-frameworks.md`), so it's hand-rolled directly on `gpui`'s own
//! `canvas()`/`paint_quad`, the same fallback the TUI cycle used for its own divergent
//! chart (ADR-0002).
//!
//! The final screen, Live Categories, demonstrates the app operating against a real
//! embedded SQLite store (`lib-database`/`lib-domain`), not dummy data -- FC-DESKTOP's own
//! real-data ticket (#37), reusing the connect/migrate/seed/read pattern the TUI's own
//! live-data demo established (`crates/bins/bin-tui/src/screen/categories.rs`, FC-TUI-005).
//! `lib-database` is built on `sqlx`'s Tokio runtime feature, but `GPUI`'s own executor is
//! `smol`-based (see `gpui`'s `Cargo.toml`), so `main` runs under `#[tokio::main]` and hands
//! a `tokio::runtime::Handle` down to the load task -- `Handle::spawn` runs the actual
//! database work on a real Tokio worker thread, and the result crosses back to `GPUI`'s own
//! executor over a `tokio::sync::oneshot` channel (a plain future, needing no Tokio runtime
//! context itself to await).

use gpui::{
    App, Application, Bounds, Context, Entity, Pixels, SharedString, Window, WindowBounds,
    WindowOptions, canvas, div, fill, point, prelude::*, px, size,
};
use gpui_component::{
    ActiveTheme,
    chart::{CandlestickChart, LineChart, PieChart},
    tab::{Tab, TabBar},
    table::{Column, Table, TableDelegate, TableState},
};

/// A single dummy monthly-spend data point, standing in for a real transaction-total
/// report (FR.35).
struct SpendPoint {
    month: SharedString,
    amount: f64,
}

/// Dummy monthly spend series, for the line chart demo.
fn dummy_spend() -> Vec<SpendPoint> {
    [
        ("Jan", 120.0),
        ("Feb", 95.0),
        ("Mar", 140.0),
        ("Apr", 110.0),
        ("May", 180.0),
        ("Jun", 160.0),
        ("Jul", 200.0),
        ("Aug", 175.0),
        ("Sep", 220.0),
        ("Oct", 205.0),
        ("Nov", 240.0),
        ("Dec", 230.0),
    ]
    .into_iter()
    .map(|(month, amount)| SpendPoint {
        month: month.into(),
        amount,
    })
    .collect()
}

/// A single dummy category-spend data point, standing in for a real per-Category
/// report (FR.35).
struct CategorySpend {
    category: SharedString,
    amount: f64,
}

/// Dummy category spend breakdown, for the doughnut chart demo.
fn dummy_categories() -> Vec<CategorySpend> {
    [
        ("Rent", 1500.0),
        ("Groceries", 420.0),
        ("Utilities", 210.0),
        ("Transport", 180.0),
        ("Entertainment", 90.0),
    ]
    .into_iter()
    .map(|(category, amount)| CategorySpend {
        category: category.into(),
        amount,
    })
    .collect()
}

/// A single dummy daily OHLC price point, standing in for a real investment's price
/// history (see the "Personal Investors" future consideration in
/// `docs/product-requirements.md`).
struct Candle {
    date: SharedString,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
}

/// Dummy daily OHLC series -- a small deterministic random walk, for the candlestick
/// chart demo. Mirrors the TUI cycle's own candlestick demo data shape
/// (`crates/bins/bin-tui/src/screen/candlestick_chart.rs`).
fn dummy_candles() -> Vec<Candle> {
    const DAY_COUNT: usize = 30;

    let mut price = 100.0_f64;
    // A tiny xorshift PRNG: deterministic across runs/platforms, no `rand` dependency
    // needed for a feasibility demo.
    let mut seed: u64 = 42;
    let mut random_delta = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed % 400) as f64 / 100.0 - 2.0
    };

    (1..=DAY_COUNT)
        .map(|day| {
            let open = price;
            let close = (open + random_delta()).max(1.0);
            let high = open.max(close) + random_delta().abs();
            let low = (open.min(close) - random_delta().abs()).max(0.5);
            price = close;
            Candle {
                date: format!("Day {day}").into(),
                open,
                high,
                low,
                close,
            }
        })
        .collect()
}

/// One category's dummy budget-vs-actual variance: positive is under budget, negative is
/// over budget (see FR.37 in `docs/product-requirements.md`).
struct Variance {
    label: SharedString,
    amount: f64,
}

/// Dummy budget-variance data -- a mix of under- and over-budget categories, so both bar
/// directions are demonstrated. Same data as the TUI cycle's own divergent chart demo
/// (`crates/bins/bin-tui/src/screen/divergent_chart.rs`).
fn dummy_variances() -> Vec<Variance> {
    [
        ("Groceries", -45.0),
        ("Rent", 5.0),
        ("Transport", 30.0),
        ("Entertainment", -15.0),
        ("Utilities", 60.0),
    ]
    .into_iter()
    .map(|(label, amount)| Variance {
        label: label.into(),
        amount,
    })
    .collect()
}

/// Computes the diverging bar's rectangle within `bounds`, given `amount` and `max_abs`
/// (the largest `|amount|` across all rows, for consistent scaling across the chart).
/// Positive amounts extend right from the vertical centre line; negative amounts extend
/// left. Pure and display-independent so the scaling maths can be unit tested without a
/// live GPUI window.
fn divergent_bar_bounds(bounds: Bounds<Pixels>, amount: f64, max_abs: f64) -> Bounds<Pixels> {
    let width = f32::from(bounds.size.width);
    let height = f32::from(bounds.size.height);
    let origin_x = f32::from(bounds.origin.x);
    let origin_y = f32::from(bounds.origin.y);
    let center_x = origin_x + width / 2.0;
    let half_width = width / 2.0;

    let fraction = if max_abs > 0.0 {
        (amount.abs() / max_abs) as f32
    } else {
        0.0
    };
    let bar_width = half_width * fraction;
    let bar_x = if amount >= 0.0 {
        center_x
    } else {
        center_x - bar_width
    };

    Bounds {
        origin: point(px(bar_x), px(origin_y)),
        size: size(px(bar_width), px(height)),
    }
}

/// One dummy row, in the shape of a Personal Ledger Transaction (see FR.16 in
/// `docs/product-requirements.md`): a date, payee, category, amount, and Transaction
/// Status. Same data as the TUI cycle's own table demo
/// (`crates/bins/bin-tui/src/screen/table.rs`).
struct Transaction {
    date: SharedString,
    payee: SharedString,
    category: SharedString,
    amount: f64,
    status: SharedString,
}

/// Dummy transaction rows spanning every Transaction Status (Open, Cleared, Reconciled)
/// and both income and expense amounts.
fn dummy_transactions() -> Vec<Transaction> {
    [
        (
            "2026-08-01",
            "Employer Pty Ltd",
            "Salary",
            3_200.00,
            "Reconciled",
        ),
        (
            "2026-08-02",
            "Woolworths",
            "Groceries",
            -84.32,
            "Reconciled",
        ),
        ("2026-08-04", "Landlord", "Rent", -1_200.00, "Reconciled"),
        (
            "2026-08-07",
            "Energy Australia",
            "Utilities",
            -145.60,
            "Cleared",
        ),
        ("2026-08-10", "Woolworths", "Groceries", -62.15, "Cleared"),
        ("2026-08-12", "Netflix", "Entertainment", -22.99, "Cleared"),
        (
            "2026-08-14",
            "Opal Transport",
            "Transport",
            -38.40,
            "Cleared",
        ),
        ("2026-08-15", "Employer Pty Ltd", "Salary", 3_200.00, "Open"),
        ("2026-08-17", "Coles", "Groceries", -71.88, "Open"),
        (
            "2026-08-19",
            "Unknown transfer",
            "Uncategorised",
            -250.00,
            "Open",
        ),
    ]
    .into_iter()
    .map(|(date, payee, category, amount, status)| Transaction {
        date: date.into(),
        payee: payee.into(),
        category: category.into(),
        amount,
        status: status.into(),
    })
    .collect()
}

/// `gpui-component`'s `TableDelegate` for the dummy transaction table.
struct TransactionTableDelegate {
    columns: Vec<Column>,
    transactions: Vec<Transaction>,
}

impl TransactionTableDelegate {
    fn new(transactions: Vec<Transaction>) -> Self {
        let columns = vec![
            Column::new("date", "Date").width(px(100.0)),
            Column::new("payee", "Payee").width(px(180.0)),
            Column::new("category", "Category").width(px(140.0)),
            Column::new("amount", "Amount")
                .width(px(100.0))
                .text_right(),
            Column::new("status", "Status").width(px(110.0)),
        ];
        Self {
            columns,
            transactions,
        }
    }
}

impl TableDelegate for TransactionTableDelegate {
    fn columns_count(&self, _cx: &App) -> usize {
        self.columns.len()
    }

    fn rows_count(&self, _cx: &App) -> usize {
        self.transactions.len()
    }

    fn column(&self, col_ix: usize, _cx: &App) -> &Column {
        &self.columns[col_ix]
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let transaction = &self.transactions[row_ix];
        match col_ix {
            0 => div().child(transaction.date.clone()).into_any_element(),
            1 => div().child(transaction.payee.clone()).into_any_element(),
            2 => div().child(transaction.category.clone()).into_any_element(),
            3 => {
                let color = if transaction.amount >= 0.0 {
                    cx.theme().success
                } else {
                    cx.theme().danger
                };
                div()
                    .text_right()
                    .text_color(color)
                    .child(format!("{:.2}", transaction.amount))
                    .into_any_element()
            }
            _ => div().child(transaction.status.clone()).into_any_element(),
        }
    }
}

/// The Live Categories demo's own SQLite file, in the OS temp directory so `cargo run`
/// never leaves a stray file in the repo working directory (it's also `.gitignore`d
/// regardless, via `*.sqlite`) -- separate from the TUI's own demo file
/// (`personal-ledger-tui-feasibility-demo.sqlite`) so the two binaries' feasibility demos
/// don't share (and potentially race on) a single store.
fn live_categories_database_url() -> String {
    let path: std::path::PathBuf =
        std::env::temp_dir().join("personal-ledger-desktop-feasibility-demo.sqlite");
    format!("sqlite://{}?mode=rwc", path.display())
}

/// What the Live Categories screen currently knows about the real SQLite data.
enum LiveCategoriesStatus {
    Loading,
    Loaded(Vec<lib_database::Categories>),
    Failed(String),
}

/// Connects to the demo database, applies `lib-database`'s own migrations, seeds one
/// category if the store is empty (a real write), then reads every category back (a real
/// read) -- proving the embedded-SQLite path works end-to-end through a real client, not a
/// mock, the same proof the TUI's own live-data demo established for FC-TUI-005.
async fn load_live_categories() -> lib_database::DatabaseResult<Vec<lib_database::Categories>> {
    load_live_categories_from(live_categories_database_url()).await
}

/// Same as [`load_live_categories`], against an explicit database URL -- split out so
/// tests can point it at an isolated, throwaway SQLite file instead of the shared demo one.
async fn load_live_categories_from(
    url: String,
) -> lib_database::DatabaseResult<Vec<lib_database::Categories>> {
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
            id: lib_domain::RowID::new(),
            code: "DEM.SEE.D01".to_string(),
            name: "Demo Seed Category".to_string(),
            description: Some(
                "Inserted by the Desktop app's embedded-SQLite feasibility demo (FC-DESKTOP real-data demo)"
                    .to_string(),
            ),
            url_slug: Some(lib_domain::UrlSlug::from("demo-seed-category")),
            category_type: lib_domain::CategoryTypes::Expense,
            color: Some(lib_domain::HexColor::from_rgb(0x4a, 0x9e, 0xd6)),
            icon: None,
            is_active: true,
            created_on: chrono::Utc::now(),
            updated_on: chrono::Utc::now(),
        };
        seed.insert(pool).await?;
    }

    lib_database::Categories::find_all(pool).await
}

/// Kicks off the Live Categories load as a detached `GPUI` task: bridges over to the given
/// Tokio runtime handle for the actual `sqlx`/`lib-database` work (see the module doc for
/// why), then reports the result back onto `DesktopApp`'s own entity state via `GPUI`'s
/// executor once the `oneshot` channel resolves.
fn spawn_live_categories_load(cx: &mut Context<DesktopApp>, tokio_handle: tokio::runtime::Handle) {
    cx.spawn(async move |this, cx| {
        let (tx, rx) = tokio::sync::oneshot::channel();
        tokio_handle.spawn(async move {
            let _ = tx.send(load_live_categories().await);
        });

        let status = match rx.await {
            Ok(Ok(categories)) => LiveCategoriesStatus::Loaded(categories),
            Ok(Err(err)) => LiveCategoriesStatus::Failed(err.to_string()),
            Err(_) => LiveCategoriesStatus::Failed(
                "embedded SQLite load task ended unexpectedly".to_string(),
            ),
        };

        this.update(cx, |view, cx| {
            view.live_categories = status;
            cx.notify();
        })
        .ok();
    })
    .detach();
}

/// Which demo screen is currently shown.
#[derive(Clone, Copy, PartialEq)]
enum Screen {
    Line,
    Doughnut,
    Candlestick,
    Divergent,
    Table,
    LiveCategories,
}

impl Screen {
    const ALL: [Screen; 6] = [
        Screen::Line,
        Screen::Doughnut,
        Screen::Candlestick,
        Screen::Divergent,
        Screen::Table,
        Screen::LiveCategories,
    ];

    fn index(self) -> usize {
        Self::ALL.iter().position(|screen| *screen == self).unwrap()
    }

    fn label(self) -> &'static str {
        match self {
            Screen::Line => "Line Chart",
            Screen::Doughnut => "Doughnut Chart",
            Screen::Candlestick => "Candlestick Chart",
            Screen::Divergent => "Divergent Chart",
            Screen::Table => "Table",
            Screen::LiveCategories => "Live Categories (SQLite)",
        }
    }
}

/// Root view: a `TabBar` switching between the chart demo screens.
struct DesktopApp {
    screen: Screen,
    spend: Vec<SpendPoint>,
    categories: Vec<CategorySpend>,
    candles: Vec<Candle>,
    variances: Vec<Variance>,
    table_state: Entity<TableState<TransactionTableDelegate>>,
    live_categories: LiveCategoriesStatus,
}

impl Render for DesktopApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity();
        let screen = self.screen;
        let selected_index = screen.index();

        let chart_colors = [
            cx.theme().chart_1,
            cx.theme().chart_2,
            cx.theme().chart_3,
            cx.theme().chart_4,
            cx.theme().chart_5,
        ];

        let body = match screen {
            Screen::Line => div()
                .h(px(400.0))
                .w_full()
                .child(
                    LineChart::new(
                        self.spend
                            .iter()
                            .map(|point| (point.month.clone(), point.amount)),
                    )
                    .x(|(month, _)| month.clone())
                    .y(|(_, amount)| *amount)
                    .stroke(chart_colors[0])
                    .dot(),
                )
                .into_any_element(),
            Screen::Doughnut => div()
                .h(px(400.0))
                .w_full()
                .child(
                    PieChart::new(self.categories.iter().enumerate().map(|(index, point)| {
                        (
                            point.category.clone(),
                            point.amount,
                            chart_colors[index % chart_colors.len()],
                        )
                    }))
                    .value(|(_, amount, _)| *amount as f32)
                    .color(|(_, _, color)| *color)
                    .inner_radius(60.0)
                    .outer_radius(100.0),
                )
                .into_any_element(),
            Screen::Candlestick => div()
                .h(px(400.0))
                .w_full()
                .child(
                    CandlestickChart::new(self.candles.iter().map(|candle| {
                        (
                            candle.date.clone(),
                            candle.open,
                            candle.high,
                            candle.low,
                            candle.close,
                        )
                    }))
                    .x(|(date, ..)| date.clone())
                    .open(|(_, open, ..)| *open)
                    .high(|(_, _, high, ..)| *high)
                    .low(|(_, _, _, low, _)| *low)
                    .close(|(_, _, _, _, close)| *close)
                    .tick_margin(4),
                )
                .into_any_element(),
            Screen::Divergent => {
                let max_abs = self
                    .variances
                    .iter()
                    .map(|variance| variance.amount.abs())
                    .fold(0.0_f64, f64::max)
                    .max(1.0);
                let success = cx.theme().success;
                let danger = cx.theme().danger;
                let zero_line_color = cx.theme().border;

                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .w_full()
                    .children(self.variances.iter().map(|variance| {
                        let amount = variance.amount;
                        let bar_color = if amount >= 0.0 { success } else { danger };
                        let sign = if amount >= 0.0 { "+" } else { "" };

                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap_2()
                            .h(px(32.0))
                            .child(div().w(px(140.0)).child(variance.label.clone()))
                            .child(
                                div().flex_1().h_full().child(
                                    canvas(
                                        move |bounds, _window, _cx| {
                                            divergent_bar_bounds(bounds, amount, max_abs)
                                        },
                                        move |row_bounds, bar_bounds, window, _cx| {
                                            let center_x = f32::from(row_bounds.origin.x)
                                                + f32::from(row_bounds.size.width) / 2.0;
                                            window.paint_quad(fill(
                                                Bounds {
                                                    origin: point(
                                                        px(center_x - 1.0),
                                                        row_bounds.origin.y,
                                                    ),
                                                    size: size(px(2.0), row_bounds.size.height),
                                                },
                                                zero_line_color,
                                            ));
                                            window.paint_quad(fill(bar_bounds, bar_color));
                                        },
                                    )
                                    .size_full(),
                                ),
                            )
                            .child(
                                div()
                                    .w(px(60.0))
                                    .text_color(bar_color)
                                    .child(format!("{sign}{amount:.0}")),
                            )
                    }))
                    .into_any_element()
            }
            Screen::Table => div()
                .h(px(400.0))
                .w_full()
                .child(Table::new(&self.table_state).stripe(true))
                .into_any_element(),
            Screen::LiveCategories => {
                let muted = cx.theme().muted_foreground;
                match &self.live_categories {
                    LiveCategoriesStatus::Loading => div()
                        .child("Connecting to embedded SQLite store...")
                        .into_any_element(),
                    LiveCategoriesStatus::Failed(message) => div()
                        .text_color(cx.theme().danger)
                        .child(format!("Failed to load: {message}"))
                        .into_any_element(),
                    LiveCategoriesStatus::Loaded(categories) => div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .w_full()
                        .child(div().text_color(muted).child(format!(
                            "{} categor{} from the embedded SQLite store",
                            categories.len(),
                            if categories.len() == 1 { "y" } else { "ies" }
                        )))
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .gap_4()
                                .text_color(muted)
                                .child(div().w(px(140.0)).child("Code"))
                                .child(div().w(px(220.0)).child("Name"))
                                .child(div().w(px(100.0)).child("Type"))
                                .child(div().w(px(80.0)).child("Active")),
                        )
                        .children(categories.iter().map(|category| {
                            div()
                                .flex()
                                .flex_row()
                                .gap_4()
                                .child(div().w(px(140.0)).child(category.code.clone()))
                                .child(div().w(px(220.0)).child(category.name.clone()))
                                .child(div().w(px(100.0)).child(category.category_type.as_str()))
                                .child(div().w(px(80.0)).child(if category.is_active {
                                    "yes"
                                } else {
                                    "no"
                                }))
                        }))
                        .into_any_element(),
                }
            }
        };

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(
                TabBar::new("screens")
                    .children(
                        Screen::ALL
                            .iter()
                            .map(|screen| Tab::new().label(screen.label())),
                    )
                    .selected_index(selected_index)
                    .on_click(move |index, _window, cx| {
                        let next = Screen::ALL[*index];
                        entity.update(cx, |this, cx| {
                            this.screen = next;
                            cx.notify();
                        });
                    }),
            )
            .child(div().flex_1().p_4().child(body))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bounds(width: f32, height: f32) -> Bounds<Pixels> {
        Bounds {
            origin: point(px(10.0), px(20.0)),
            size: size(px(width), px(height)),
        }
    }

    #[test]
    fn zero_amount_has_no_width() {
        let bar = divergent_bar_bounds(bounds(400.0, 32.0), 0.0, 60.0);
        assert_eq!(f32::from(bar.size.width), 0.0);
    }

    #[test]
    fn positive_amount_extends_right_from_centre() {
        let bounds = bounds(400.0, 32.0);
        let bar = divergent_bar_bounds(bounds, 30.0, 60.0);
        let center_x = f32::from(bounds.origin.x) + f32::from(bounds.size.width) / 2.0;

        assert_eq!(f32::from(bar.origin.x), center_x);
        // Half the max, so half of the half-width available on the positive side.
        assert_eq!(
            f32::from(bar.size.width),
            f32::from(bounds.size.width) / 4.0
        );
    }

    #[test]
    fn negative_amount_extends_left_from_centre() {
        let bounds = bounds(400.0, 32.0);
        let bar = divergent_bar_bounds(bounds, -60.0, 60.0);
        let center_x = f32::from(bounds.origin.x) + f32::from(bounds.size.width) / 2.0;
        let half_width = f32::from(bounds.size.width) / 2.0;

        // The largest magnitude fills the whole half-width, ending exactly at centre.
        assert_eq!(f32::from(bar.size.width), half_width);
        assert_eq!(f32::from(bar.origin.x), center_x - half_width);
    }

    #[test]
    fn zero_max_abs_produces_no_bar() {
        let bar = divergent_bar_bounds(bounds(400.0, 32.0), 0.0, 0.0);
        assert_eq!(f32::from(bar.size.width), 0.0);
    }

    /// Exercises the real embedded-SQLite path end-to-end against an isolated, throwaway
    /// database file: connect, migrate (a write), seed-if-empty (a write), and read back --
    /// proving FC-DESKTOP's real-data ticket against actual `lib-database`/`lib-domain`
    /// code, not a mock. Runs `load_live_categories_from` twice against the same file to
    /// confirm the seed step is idempotent. Mirrors the TUI's own equivalent test
    /// (`crates/bins/bin-tui/src/screen/categories.rs`).
    #[tokio::test]
    async fn load_live_categories_from_seeds_once_and_reads_back() {
        let path = std::env::temp_dir().join(format!(
            "personal-ledger-desktop-test-{}.sqlite",
            lib_domain::RowID::new()
        ));
        let url = format!("sqlite://{}?mode=rwc", path.display());

        let first = load_live_categories_from(url.clone())
            .await
            .expect("first load should connect, migrate, seed, and read successfully");
        assert_eq!(
            first.len(),
            1,
            "a fresh database should be seeded with one category"
        );
        assert_eq!(first[0].code, "DEM.SEE.D01");

        let second = load_live_categories_from(url)
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

// `#[tokio::main]` so a real Tokio runtime exists for the Live Categories screen's
// `lib-database`/`sqlx` work to run on -- see the module doc for why `GPUI`'s own
// (`smol`-based) executor can't run it directly. `Application::run` below is still a plain
// synchronous, blocking call (GPUI owns the native event loop until the app quits); running
// it un-awaited inside this async fn body just means it executes on Tokio's `block_on`
// thread rather than a worker thread, which is exactly where the main/UI thread needs to be.
#[tokio::main]
async fn main() {
    let tokio_handle = tokio::runtime::Handle::current();

    Application::new().run(move |cx: &mut App| {
        gpui_component::init(cx);

        let bounds = Bounds::centered(None, size(px(800.0), px(500.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            move |window, cx| {
                cx.new(|cx| {
                    let table_state = cx.new(|state_cx| {
                        TableState::new(
                            TransactionTableDelegate::new(dummy_transactions()),
                            window,
                            state_cx,
                        )
                    });
                    spawn_live_categories_load(cx, tokio_handle.clone());
                    DesktopApp {
                        screen: Screen::Line,
                        spend: dummy_spend(),
                        categories: dummy_categories(),
                        candles: dummy_candles(),
                        variances: dummy_variances(),
                        table_state,
                        live_categories: LiveCategoriesStatus::Loading,
                    }
                })
            },
        )
        .unwrap();
        cx.activate(true);
    });
}
