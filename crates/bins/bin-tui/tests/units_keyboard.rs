//! Keyboard tests for the Units View on the `lib-units` Presenters (ADR-0035): a key reaches the
//! page or Dialog Presenter and its view model is drawn. The rules themselves are the Presenter
//! tests' in `lib-units`; these check the wiring, and that the TUI shows the same refusals.

mod common;

use bin_tui::event::Event;
use common::{press, screen, shell, shell_with_failures, type_text};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use lib_database::{Domain, WriteFailure};

#[test]
fn g_u_draws_the_sample_units_and_price_sources() {
    let mut shell = shell();

    press(&mut shell, "g u");
    let frame = screen(&mut shell);

    assert!(frame.contains("UNITS"), "{frame}");
    for code in ["AUD", "BTC", "VAS"] {
        assert!(frame.contains(code), "{code} missing:\n{frame}");
    }
    assert!(frame.contains("PRICE SOURCES"), "{frame}");
    assert!(frame.contains("Manual entry"), "{frame}");
    // The first Unit is selected, so the summary names it.
    assert!(frame.contains("Australian Dollar"), "{frame}");
}

#[test]
fn j_moves_the_selection_and_the_summary_follows() {
    let mut shell = shell();
    press(&mut shell, "g u");

    press(&mut shell, "j");
    let frame = screen(&mut shell);

    assert!(frame.contains("Bitcoin"), "{frame}");
    assert!(!frame.contains("Australian Dollar"), "{frame}");
}

#[test]
fn a_unit_added_through_its_dialog_shows_in_the_table() {
    let mut shell = shell();
    press(&mut shell, "g u n");
    let open = screen(&mut shell);
    assert!(open.contains("INSERT"), "{open}");
    assert!(open.contains("esc close unit form"), "{open}");

    // Code, then Tab past Type to Name.
    type_text(&mut shell, "EUR");
    press(&mut shell, "Tab Tab");
    type_text(&mut shell, "Euro");
    press(&mut shell, "Enter");
    let frame = screen(&mut shell);

    assert!(!frame.contains("INSERT"), "the Dialog closed:\n{frame}");
    assert!(frame.contains("EUR"), "{frame}");
}

/// The words on the Dialog's Precision line, none when it isn't drawn.
fn precision_line(frame: &str) -> Vec<String> {
    frame
        .lines()
        .find(|line| line.contains("Precision"))
        .unwrap_or_default()
        .split_whitespace()
        .map(str::to_string)
        .collect()
}

#[test]
fn add_unit_starts_precision_on_its_type_and_takes_a_typed_one() {
    let mut shell = shell();
    press(&mut shell, "g u n");
    assert!(precision_line(&screen(&mut shell)).contains(&"2".to_string()));

    // Tab to Type and step Currency to Crypto.
    press(&mut shell, "Tab Right");
    assert!(precision_line(&screen(&mut shell)).contains(&"8".to_string()));

    // Tab past Name to Precision and type over the 8.
    press(&mut shell, "Tab Tab Backspace");
    type_text(&mut shell, "6");
    let frame = screen(&mut shell);

    assert!(precision_line(&frame).contains(&"6".to_string()), "{frame}");
}

#[test]
fn deleting_a_unit_in_use_opens_offering_deactivate_with_its_usage() {
    let mut shell = shell();
    press(&mut shell, "g u d");

    let frame = screen(&mut shell);

    assert!(
        frame.contains(&lib_units::msg::units_deactivate_title("AUD")),
        "{frame}"
    );
    assert!(
        frame.contains("AUD is used by") && frame.contains("locations"),
        "the usage says why, Locations included:\n{frame}"
    );
    assert!(
        frame.contains("It is also the base unit"),
        "the base Unit's Deactivate says why it is disabled:\n{frame}"
    );
}

#[test]
fn capital_b_opens_set_base_unit() {
    let mut shell = shell();

    press(&mut shell, "g u B");
    let frame = screen(&mut shell);

    assert!(
        frame.contains(&lib_units::msg::units_set_base_title()),
        "{frame}"
    );
}

#[test]
fn ctrl_u_moves_the_selection_up_half_a_page() {
    let mut shell = shell();
    press(&mut shell, "g u G");
    assert!(!screen(&mut shell).contains("Bitcoin"));

    shell.handle_event(Event::Key(KeyEvent::new(
        KeyCode::Char('u'),
        KeyModifiers::CONTROL,
    )));
    let frame = screen(&mut shell);

    assert!(
        frame.contains("Bitcoin"),
        "from the last Price Source back into the Units, not to the Units View again:\n{frame}"
    );
}

#[test]
fn editing_a_unit_shows_what_uses_it() {
    let mut shell = shell();
    press(&mut shell, "g u e");

    let frame = screen(&mut shell);

    assert!(frame.contains("Used by "), "{frame}");
}

#[test]
fn deleting_the_base_unit_moves_the_base_first() {
    let mut shell = shell();
    // Add xau, which nothing uses, and make it the base: the base select lists by code, so it is
    // the last option.
    press(&mut shell, "g u n");
    type_text(&mut shell, "XAU");
    press(&mut shell, "Tab Tab");
    type_text(&mut shell, "Gold");
    press(&mut shell, "Enter B Right Right Right Enter");
    // xau is the fourth Unit, under aud, which is still selected.
    press(&mut shell, "j j j d");
    let dialog = screen(&mut shell);
    assert!(
        dialog.contains(&lib_units::msg::units_delete_new_base_label()),
        "the base can't go without a new one:\n{dialog}"
    );

    type_text(&mut shell, "XAU");
    press(&mut shell, "Enter");
    let frame = screen(&mut shell);

    assert!(frame.contains("Deleted unit XAU"), "{frame}");
    assert!(
        frame.contains("Currency \u{b7} base"),
        "AUD, the first candidate, is the base:\n{frame}"
    );
}

#[test]
fn esc_cancels_a_dialog_and_leaves_the_view_open() {
    let mut shell = shell();
    press(&mut shell, "g u n");
    type_text(&mut shell, "zzz");

    press(&mut shell, "Esc");
    let frame = screen(&mut shell);

    assert!(frame.contains("UNITS"), "{frame}");
    assert!(!frame.contains("zzz"), "nothing was added:\n{frame}");
}

#[test]
fn a_failed_units_write_raises_an_error_toast() {
    let (mut shell, report) = shell_with_failures();
    press(&mut shell, "g u");

    report
        .send(WriteFailure {
            domains: vec![Domain::Units],
            error: lib_database::Error::WriterStopped,
        })
        .expect("the Shell holds the receiver");
    shell.handle_event(Event::Tick);
    let frame = screen(&mut shell);

    assert!(
        frame.contains("Couldn't save the change to units"),
        "{frame}"
    );
}

#[test]
fn p_sets_a_manual_entry_units_price_and_toasts_its_week() {
    let mut shell = shell();
    // vas, the third Unit, is on Manual entry.
    press(&mut shell, "g u j j p");
    let open = screen(&mut shell);
    assert!(open.contains("Set price — VAS"), "{open}");

    type_text(&mut shell, "42");
    press(&mut shell, "Enter");
    let frame = screen(&mut shell);

    assert!(
        !frame.contains("Set price — VAS"),
        "the Dialog closed:\n{frame}"
    );
    assert!(frame.contains("Set the price of VAS"), "{frame}");
}

#[test]
fn p_on_a_unit_its_source_prices_opens_nothing() {
    let mut shell = shell();

    press(&mut shell, "g u j p");
    let frame = screen(&mut shell);

    assert!(!frame.contains("Set price"), "{frame}");
}

#[test]
fn deleting_a_unit_an_account_in_the_opened_file_uses_is_refused() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let path = directory.path().join("household.pldb");
    // xau, which nothing in the sample uses, with an Account in it in the file.
    common::ledger_file_with(&path, |storage| {
        let mut xau = storage
            .units
            .units()
            .unit(lib_core::UnitId::SEED_BTC)
            .expect("the seed has BTC");
        xau.id = lib_core::UnitId::new();
        xau.code = "XAU".to_string();
        xau.name = "Gold".to_string();
        let unused = lib_units::UnitUse::default();
        storage
            .units
            .split_mut(&unused)
            .units
            .upsert_unit(xau.clone());
        let mut account = storage.accounts.accounts().accounts()[0].clone();
        account.id = lib_core::AccountId::new();
        account.unit = xau.id;
        storage.accounts.accounts_mut().upsert_account(account);
    });
    let mut shell = common::cold_shell(None);
    common::run_command(&mut shell, &format!("open {}", path.display()));

    press(&mut shell, "g u j j j d");
    let frame = screen(&mut shell);

    assert!(
        frame.contains(&lib_units::msg::units_deactivate_title("XAU")),
        "refused before the commit, offering Deactivate:\n{frame}"
    );
    assert!(frame.contains("XAU is used by"), "{frame}");
}

#[test]
fn deleting_a_unit_a_budget_in_the_opened_file_uses_is_refused() {
    use lib_budgets::BudgetRepository as _;

    let directory = tempfile::tempdir().expect("a temporary directory");
    let path = directory.path().join("household.pldb");
    // xau, which nothing in the sample uses, with only a Budget in it in the file (#952).
    common::ledger_file_with(&path, |storage| {
        let mut xau = storage
            .units
            .units()
            .unit(lib_core::UnitId::SEED_BTC)
            .expect("the seed has BTC");
        xau.id = lib_core::UnitId::new();
        xau.code = "XAU".to_string();
        xau.name = "Gold".to_string();
        let unused = lib_units::UnitUse::default();
        storage
            .units
            .split_mut(&unused)
            .units
            .upsert_unit(xau.clone());
        let mut budget = storage.budgets.budgets()[1].clone();
        budget.id = lib_core::BudgetId::new();
        budget.name = "Gold".to_string();
        budget.is_default = false;
        budget.unit = xau.id;
        budget.account_ids.clear();
        storage.budgets.upsert_budget(&budget);
    });
    let mut shell = common::cold_shell(None);
    common::run_command(&mut shell, &format!("open {}", path.display()));

    press(&mut shell, "g u j j j d");
    let frame = screen(&mut shell);

    assert!(
        frame.contains(&lib_units::msg::units_deactivate_title("XAU")),
        "refused before the commit, offering Deactivate:\n{frame}"
    );
    assert!(
        frame.contains("XAU is used by") && frame.contains("budget"),
        "{frame}"
    );
}

#[test]
fn a_unit_only_the_sample_uses_can_be_deleted_from_the_opened_file() {
    use lib_database::InstitutionsStore;

    let directory = tempfile::tempdir().expect("a temporary directory");
    let path = directory.path().join("household.pldb");
    // The sample's btc Accounts are in aud in the file, and its Institution has no Usual unit, so
    // nothing in the file uses btc.
    common::ledger_file_with(&path, |storage| {
        for mut account in storage.accounts.accounts().accounts() {
            if account.unit == lib_core::UnitId::SEED_BTC {
                account.unit = lib_core::UnitId::SEED_AUD;
                storage.accounts.accounts_mut().upsert_account(account);
            }
        }
        for mut institution in storage.institutions.institutions().institutions() {
            if institution.usual_unit == Some(lib_core::UnitId::SEED_BTC) {
                institution.usual_unit = None;
                storage
                    .institutions
                    .institutions_mut()
                    .upsert_institution(institution);
            }
        }
    });
    let mut shell = common::cold_shell(None);
    common::run_command(&mut shell, &format!("open {}", path.display()));

    press(&mut shell, "g u j d");
    let dialog = screen(&mut shell);
    assert!(
        !dialog.contains(&lib_units::msg::units_deactivate_title("BTC")),
        "the sample's usage doesn't count:\n{dialog}"
    );
    type_text(&mut shell, "BTC");
    press(&mut shell, "Enter");
    let frame = screen(&mut shell);

    assert!(frame.contains("Deleted unit BTC"), "{frame}");
}

#[test]
fn f_on_a_frankfurter_unit_writes_this_weeks_price_and_shows_a_success_toast() {
    let (transport, responses) = lib_units::fake::FakeTransport::channel();
    let mut shell = common::fetching_shell(transport.clone(), responses);
    press(&mut shell, "g u j j");
    assert!(screen(&mut shell).contains("Frankfurter"));

    press(&mut shell, "f");
    assert_eq!(
        transport.urls(),
        ["https://api.frankfurter.dev/v2/rate/EUR/AUD"]
    );
    transport.answer_last(200, r#"{"base":"EUR","quote":"AUD","rate":1.6512}"#);
    shell.handle_event(Event::Tick);

    let frame = screen(&mut shell);
    assert!(frame.contains("Fetched EUR at 1.65 AUD"), "{frame}");
}

#[test]
fn f_on_a_coingecko_unit_asks_in_lower_case_and_writes_the_price() {
    let (transport, responses) = lib_units::fake::FakeTransport::channel();
    let mut shell = common::fetching_shell(transport.clone(), responses);
    press(&mut shell, "g u j");
    let docked = common::screen_sized(&mut shell, 220, 60);
    assert!(docked.contains("Powered by CoinGecko"), "{docked}");

    press(&mut shell, "f");
    assert_eq!(
        transport.urls(),
        ["https://api.coingecko.com/api/v3/simple/price?ids=bitcoin&vs_currencies=aud"]
    );
    transport.answer_last(200, r#"{"bitcoin":{"aud":92568}}"#);
    shell.handle_event(Event::Tick);

    let frame = screen(&mut shell);
    assert!(frame.contains("Fetched BTC at 92,568.00 AUD"), "{frame}");
}

#[test]
fn f_after_a_429_waits_out_its_retry_after_and_a_tick_sends_it_once_the_clock_passes() {
    let (transport, responses) = lib_units::fake::FakeTransport::channel();
    let mut shell = common::fetching_shell(transport.clone(), responses);
    press(&mut shell, "g u j f");
    // The TUI reads the real time, so the provider asks for the shortest wait it can.
    transport.answer_last_with(429, "", Some("1"));
    shell.handle_event(Event::Tick);

    press(&mut shell, "f");
    shell.handle_event(Event::Tick);
    assert_eq!(
        transport.requests().len(),
        1,
        "held by CoinGecko's Retry-After"
    );
    let waiting = screen(&mut shell);
    assert!(waiting.contains("Waiting for CoinGecko, 1 s"), "{waiting}");

    std::thread::sleep(std::time::Duration::from_millis(1100));
    shell.handle_event(Event::Tick);

    assert_eq!(
        transport.requests().len(),
        2,
        "the tick sent the paced call"
    );
    transport.answer_last(200, r#"{"bitcoin":{"aud":92568}}"#);
    shell.handle_event(Event::Tick);
    let frame = screen(&mut shell);
    assert!(frame.contains("Fetched BTC at 92,568.00 AUD"), "{frame}");
}

/// Adds ABC on a Custom source sending one header and BHP on Yahoo Finance, both quoted in AUD.
fn abc_on_a_broker_and_bhp_on_yahoo(fakes: &mut lib_database::FakeStores) {
    let broker = lib_units::CustomSource {
        id: lib_core::PriceSourceId::new(),
        name: "Broker".into(),
        endpoint: lib_units::Endpoint {
            address: "https://broker.example".to_string().into(),
            request: "/quote/{symbol}".to_string().into(),
            price_path: "/last".to_string().into(),
            auth: lib_units::PriceSourceAuth::None,
            headers: vec![lib_units::RequestHeader {
                name: "X-Client".to_string().into(),
                value: "ledger; v1".to_string().into(),
            }]
            .into(),
            batch: None,
            history: None,
        },
        updates: lib_units::Updates::Hourly,
    };
    for (code, source_id, symbol) in [
        ("ABC", broker.id, "ABC"),
        ("BHP", lib_units::YAHOO_FINANCE, "BHP.AX"),
    ] {
        fakes.units.units.units.push(lib_units::UnitRow::new(
            code.into(),
            code.into(),
            lib_core::UnitKind::Equity,
            lib_units::Precision::of_kind(lib_core::UnitKind::Equity),
            lib_units::PriceLink {
                source_id,
                symbol: symbol.into(),
                quote: lib_core::UnitId::SEED_AUD,
            },
        ));
    }
    fakes
        .units
        .price_sources
        .records
        .push(lib_units::PriceSourceRecord::Custom(broker));
}

#[test]
fn f_on_a_custom_sources_and_a_yahoo_unit_sends_their_headers_and_writes_their_prices() {
    let (transport, responses) = lib_units::fake::FakeTransport::channel();
    let mut shell = common::fetching_shell_with(
        transport.clone(),
        responses,
        abc_on_a_broker_and_bhp_on_yahoo,
    );
    // ABC, AUD, BHP, …
    press(&mut shell, "g u f");
    let (_, abc) = transport.requests().pop().expect("f on ABC sent a request");
    assert_eq!(abc.url, "https://broker.example/quote/ABC");
    assert_eq!(abc.headers, [("X-Client".into(), "ledger; v1".into())]);
    transport.answer_last(200, r#"{"last":12.5}"#);
    shell.handle_event(Event::Tick);
    assert!(
        screen(&mut shell).contains("Fetched ABC at 12.50 AUD"),
        "{}",
        screen(&mut shell)
    );

    press(&mut shell, "j j f");
    let (_, bhp) = transport.requests().pop().expect("f on BHP sent a request");
    assert_eq!(
        bhp.headers,
        [(
            "User-Agent".into(),
            format!("Personal-Ledger/{}", env!("CARGO_PKG_VERSION"))
        )]
    );
    transport.answer_last(
        200,
        r#"{"chart":{"result":[{"meta":{"regularMarketPrice":52.52}}],"error":null}}"#,
    );
    shell.handle_event(Event::Tick);
    let frame = screen(&mut shell);
    assert!(frame.contains("Fetched BHP at 52.52 AUD"), "{frame}");
}

#[test]
fn closing_the_ledger_drops_in_flight_fetches_without_writing() {
    let (transport, responses) = lib_units::fake::FakeTransport::channel();
    let mut shell = common::fetching_shell(transport.clone(), responses);
    press(&mut shell, "g u j j f");
    assert_eq!(transport.requests().len(), 1);

    common::run_command(&mut shell, "close");
    transport.answer_last(200, r#"{"base":"EUR","quote":"AUD","rate":1.6512}"#);
    shell.handle_event(Event::Tick);

    let frame = screen(&mut shell);
    assert!(!frame.contains("Fetched EUR"), "{frame}");
}

#[test]
fn f_answered_not_found_warns_and_flags_the_row_the_weekly_prices_and_last_fetch() {
    let (transport, responses) = lib_units::fake::FakeTransport::channel();
    let mut shell = common::fetching_shell(transport.clone(), responses);
    press(&mut shell, "g u j j f");
    transport.answer_last(404, r#"{"message":"not found"}"#);
    shell.handle_event(Event::Tick);

    let frame = large(&mut shell);
    // The Toast's text is cut to its box.
    assert!(
        frame.contains("! Couldn't fetch EUR: Frankfurter has no"),
        "{frame}"
    );
    assert!(frame.contains("Flags           fetch failed"), "{frame}");
    assert!(frame.contains("⚠ fetch failed"), "{frame}");

    let docked = common::screen_sized(&mut shell, 220, 60);
    assert!(docked.contains("Last fetch"), "{docked}");
    assert!(docked.contains("Failed just now · Not found"), "{docked}");
}

#[test]
fn f_failing_its_round_warns_and_shows_the_reason_in_last_updated() {
    let (transport, responses) = lib_units::fake::FakeTransport::channel();
    let mut shell = common::fetching_shell(transport.clone(), responses);
    press(&mut shell, "g u j j f");
    transport.answer(
        transport.requests()[0].0,
        lib_units::TransportOutcome::Failed(lib_units::TransportError::Offline),
    );
    shell.handle_event(Event::Tick);

    let frame = large(&mut shell);
    assert!(
        frame.contains("! Couldn't fetch from Frankfurter"),
        "{frame}"
    );
    let row = frame
        .lines()
        .find(|line| line.starts_with("Frankfurter"))
        .unwrap_or_else(|| panic!("no Frankfurter row:\n{frame}"));
    assert!(row.trim_end().ends_with("Offline"), "{row}");
}

#[test]
fn an_out_of_date_price_flags_the_weekly_prices_heading() {
    // The sample's btc, its prices written a day ago: past twice CoinGecko's 15 minutes.
    let ports =
        lib_units::fake::InMemoryPorts::sample(chrono::Utc::now() - chrono::Duration::days(1));
    let (mut shell, _) = common::shell_on(
        Box::new(ports),
        std::rc::Rc::new(lib_database::InMemoryCommit::default()),
    );
    press(&mut shell, "g u j");
    let frame = large(&mut shell);

    assert!(frame.contains("WEEKLY PRICES  BTC · CoinGecko"), "{frame}");
    assert!(frame.contains("⚠ Out of date"), "{frame}");
}

/// A Large terminal, tall enough for the weekly prices region to list every week it holds.
fn large(shell: &mut bin_tui::shell::Shell) -> String {
    common::screen_sized(shell, 160, 48)
}

#[test]
fn the_weekly_prices_region_draws_the_selected_units_price_history() {
    let mut shell = shell();
    press(&mut shell, "g u j");
    let frame = large(&mut shell);

    assert!(frame.contains("WEEKLY PRICES  BTC · CoinGecko"), "{frame}");
    assert!(frame.contains("this week · provisional"), "{frame}");
    for column in ["WEEK", "OPEN", "HIGH", "LOW", "CLOSE", "CHANGE"] {
        assert!(frame.contains(column), "{column} missing:\n{frame}");
    }
    let provisional = frame
        .lines()
        .filter(|line| line.contains(" provisional ") && !line.contains("this week"))
        .count();
    assert_eq!(provisional, 1, "only this week's row is tagged:\n{frame}");
    assert!(
        frame.contains("Newest week first · 12 of 52 weeks  ·  enter all prices"),
        "{frame}"
    );
}

#[test]
fn the_weekly_prices_region_keeps_the_last_unit_while_a_price_source_is_selected() {
    let mut shell = shell();
    press(&mut shell, "g u j Tab");
    let frame = large(&mut shell);

    assert!(frame.contains("WEEKLY PRICES  BTC · CoinGecko"), "{frame}");
}

#[test]
fn the_weekly_prices_region_says_why_the_base_and_an_unpriced_unit_have_no_prices() {
    let mut shell = shell();
    press(&mut shell, "g u");
    let base = large(&mut shell);
    assert!(
        base.contains("AUD is the base unit: everything is valued in it, so it has no price."),
        "{base}"
    );

    press(&mut shell, "j j");
    let vas = large(&mut shell);
    assert!(vas.contains("WEEKLY PRICES  VAS · Manual entry"), "{vas}");
    assert!(
        vas.contains("VAS has no prices yet. A fetch or backfill will fill this."),
        "{vas}"
    );
}

#[test]
fn a_manual_entry_units_week_shows_its_close_with_no_open_high_or_low() {
    let mut shell = shell();
    press(&mut shell, "g u j j p");
    type_text(&mut shell, "42");
    press(&mut shell, "Enter");
    let frame = large(&mut shell);

    let row = frame
        .lines()
        .find(|line| line.contains("42.00") && line.contains("provisional"))
        .unwrap_or_else(|| panic!("no week of 42.00:\n{frame}"));
    assert_eq!(
        row.matches('—').count(),
        4,
        "OPEN, HIGH, LOW and CHANGE: {row}"
    );
    assert!(frame.contains("Newest week first · 1 of 1 week"), "{frame}");
}

#[test]
fn a_medium_terminal_shows_week_close_and_change_only() {
    let mut shell = shell();
    press(&mut shell, "g u j");
    let frame = common::screen_sized(&mut shell, 96, 48);

    assert!(
        frame.contains("CLOSE") && frame.contains("CHANGE"),
        "{frame}"
    );
    assert!(!frame.contains("OPEN"), "{frame}");
    assert!(frame.contains("148,212.40"), "{frame}");
    assert!(
        frame
            .lines()
            .any(|line| line.contains("WEEKLY PRICES  BTC · CoinGecko")),
        "the heading names the Unit before the flag, which gives way:\n{frame}"
    );
}

#[test]
fn f_on_a_price_source_row_fetches_its_units_and_toasts_the_start_then_the_outcome() {
    let (transport, responses) = lib_units::fake::FakeTransport::channel();
    let mut shell = common::fetching_shell(transport.clone(), responses);
    press(&mut shell, "g u Tab j f");

    assert_eq!(
        transport.urls(),
        ["https://api.frankfurter.dev/v2/rate/EUR/AUD"]
    );
    let started = screen(&mut shell);
    assert!(
        started.contains("Fetching prices from Frankfurter"),
        "{started}"
    );
    assert!(
        started.contains("Fetching\u{2026}"),
        "the row says so:\n{started}"
    );

    press(&mut shell, "f");
    assert_eq!(
        transport.requests().len(),
        1,
        "a second press sends nothing"
    );
    assert!(
        screen(&mut shell).contains("Already fetching prices from Frankfurter"),
        "a second press says so"
    );

    transport.answer_last(200, r#"{"base":"EUR","quote":"AUD","rate":1.6512}"#);
    shell.handle_event(Event::Tick);

    let frame = screen(&mut shell);
    assert!(frame.contains("Fetched EUR at 1.65 AUD"), "{frame}");
}

#[test]
fn coingeckos_edit_dialog_takes_a_masked_demo_key_and_remove_key_clears_it() {
    let mut shell = shell();
    // The sources list by name, so CoinGecko is first.
    press(&mut shell, "g u Tab");
    let pane = common::screen_sized(&mut shell, 220, 60);
    assert!(
        pane.contains("API key") && pane.contains("Not set"),
        "{pane}"
    );

    press(&mut shell, "e");
    press(&mut shell, "Tab Tab");
    type_text(&mut shell, "demo");
    let typed = screen(&mut shell);
    assert!(
        typed.contains("\u{2022}\u{2022}\u{2022}\u{2022}"),
        "{typed}"
    );
    assert!(!typed.contains("demo"), "never drawn:\n{typed}");
    press(&mut shell, "Enter");
    let saved = common::screen_sized(&mut shell, 220, 60);
    assert!(saved.contains("Saved"), "{saved}");

    press(&mut shell, "e");
    let open = screen(&mut shell);
    assert!(
        open.contains("Key saved · leave blank to keep it"),
        "{open}"
    );
    assert!(open.contains("[ Remove key ]"), "{open}");
    press(&mut shell, "Tab Tab Tab Enter");
    let removed = screen(&mut shell);
    assert!(!removed.contains("[ Remove key ]"), "{removed}");
    press(&mut shell, "Enter");

    let cleared = common::screen_sized(&mut shell, 220, 60);
    assert!(cleared.contains("Not set"), "{cleared}");
}

#[test]
fn t_on_a_price_source_row_tests_its_first_unit_and_answers_with_one_toast_writing_nothing() {
    let (transport, responses) = lib_units::fake::FakeTransport::channel();
    let mut shell = common::fetching_shell(transport.clone(), responses);
    press(&mut shell, "g u Tab j t");
    assert_eq!(
        transport.urls(),
        ["https://api.frankfurter.dev/v2/rate/EUR/AUD"]
    );
    press(&mut shell, "t");
    assert!(
        screen(&mut shell).contains("Already testing Frankfurter"),
        "a second press says so"
    );
    assert_eq!(transport.requests().len(), 1);

    transport.answer_last(200, r#"{"base":"EUR","quote":"AUD","rate":1.6512}"#);
    shell.handle_event(Event::Tick);

    let frame = screen(&mut shell);
    assert!(
        frame.contains("Frankfurter works: EUR · 1.65 AUD"),
        "{frame}"
    );
    assert!(
        !frame.contains("Fetched EUR"),
        "a test writes no price:\n{frame}"
    );
}

#[test]
fn a_built_ins_edit_dialog_tests_its_first_unit_inline() {
    let (transport, responses) = lib_units::fake::FakeTransport::channel();
    let mut shell = common::fetching_shell(transport.clone(), responses);
    press(&mut shell, "g u Tab e");
    let open = screen(&mut shell);
    assert!(open.contains("[ Test price source ]"), "{open}");
    assert!(
        open.contains("Prices BTC, the first unit it values."),
        "{open}"
    );
    assert!(open.contains("A test writes nothing."), "{open}");

    press(&mut shell, "Tab Tab Tab Enter");
    assert_eq!(
        transport.urls(),
        ["https://api.coingecko.com/api/v3/simple/price?ids=bitcoin&vs_currencies=aud"]
    );
    assert!(screen(&mut shell).contains("Testing…"));
    transport.answer_last(200, r#"{"bitcoin":{"aud":92568}}"#);
    shell.handle_event(Event::Tick);

    let tested = screen(&mut shell);
    assert!(tested.contains("Price found"), "{tested}");
    assert!(
        tested.contains("bitcoin · 92,568.00 AUD · tested"),
        "{tested}"
    );
    assert!(!tested.contains("Fetched BTC"), "{tested}");
}

#[test]
fn test_with_symbol_on_an_unsaved_custom_source_shows_no_price_and_its_response_indented() {
    let (transport, responses) = lib_units::fake::FakeTransport::channel();
    let mut shell = common::fetching_shell(transport.clone(), responses);
    press(&mut shell, "g u Tab n");
    type_text(&mut shell, "Broker");
    press(&mut shell, "Tab Tab");
    type_text(&mut shell, "https://broker.example");
    press(&mut shell, "Tab");
    type_text(&mut shell, "/quote/{symbol}");
    press(&mut shell, "Tab");
    type_text(&mut shell, "/last");
    let incomplete = screen(&mut shell);
    assert!(
        incomplete.contains("Type a symbol to test."),
        "{incomplete}"
    );

    press(&mut shell, "Tab Tab Tab");
    type_text(&mut shell, "ABC");
    press(&mut shell, "Enter");
    assert_eq!(transport.urls(), ["https://broker.example/quote/ABC"]);
    transport.answer_last(200, r#"{"price":12.5}"#);
    shell.handle_event(Event::Tick);

    let frame = screen(&mut shell);
    assert!(frame.contains("No price"), "{frame}");
    assert!(frame.contains("Response · HTTP 200"), "{frame}");
    assert!(frame.contains(r#"    "price": 12.5"#), "indented:\n{frame}");
    assert!(frame.contains("Add price source"), "the form stays open");
}

/// Adds AED, quoted in aud, on Frankfurter, with an Account in it opened in June 2019, so every
/// week since then wants a Weekly Price. Its code sorts first, so `g u` selects it.
#[expect(
    clippy::expect_used,
    reason = "test helper: the sample has Accounts and the date is a literal"
)]
fn aed_held_since_2019(fakes: &mut lib_database::FakeStores) {
    let aed = lib_units::UnitRow::new(
        "AED".into(),
        "UAE Dirham".into(),
        lib_core::UnitKind::Currency,
        lib_units::Precision::of_kind(lib_core::UnitKind::Currency),
        lib_units::PriceLink {
            source_id: lib_units::FRANKFURTER,
            symbol: "AED".into(),
            quote: lib_core::UnitId::SEED_AUD,
        },
    );
    let mut account = fakes
        .accounts
        .accounts
        .accounts
        .first()
        .cloned()
        .expect("the sample has Accounts");
    account.id = lib_core::AccountId::new();
    account.name = "Dirham savings".into();
    account.unit = aed.id;
    account.opened_at = chrono::NaiveDate::from_ymd_opt(2019, 6, 5).expect("a real date");
    fakes.accounts.accounts.accounts.push(account);
    fakes.units.units.units.push(aed);
}

/// The history request sent for AED.
#[expect(
    clippy::expect_used,
    reason = "test helper: no history request for AED should fail the test"
)]
fn aed_history(transport: &lib_units::fake::FakeTransport) -> (lib_units::RequestId, String) {
    transport
        .requests()
        .into_iter()
        .map(|(id, request)| (id, request.url))
        .find(|(_, url)| url.contains("/v2/rates?base=AED"))
        .expect("opening the Ledger asked AED's history")
}

#[test]
fn a_unit_with_accounts_opened_in_2019_fills_its_empty_weeks_back_to_then_on_open() {
    let (transport, responses) = lib_units::fake::FakeTransport::channel();
    let mut shell = common::fetching_shell_with(transport.clone(), responses, aed_held_since_2019);
    press(&mut shell, "g u");
    let (id, url) = aed_history(&transport);
    assert!(url.contains("&from=2019-06-03&"), "{url}");
    let frame = common::screen_sized(&mut shell, 220, 60);
    assert!(frame.contains("Fetching history\u{2026}"), "{frame}");

    // The TUI reads the real day, so the source answers for last week, whenever that is.
    let last_week = lib_units::WeeklyPrice::week_of(chrono::Local::now().date_naive())
        - chrono::Duration::weeks(1);
    transport.answer(
        id,
        lib_units::TransportOutcome::Answered {
            status: 200,
            body: format!(r#"[{{"date":"{last_week}","base":"AED","quote":"AUD","rate":2.3456}}]"#),
            retry_after: None,
        },
    );
    shell.handle_event(Event::Tick);

    let frame = common::screen_sized(&mut shell, 220, 60);
    assert!(
        frame.contains("2.35"),
        "last week's backfilled close:\n{frame}"
    );
    assert!(!frame.contains("Fetching history"), "{frame}");
    assert!(
        !frame.contains("Fetched AED"),
        "a backfill never toasts:\n{frame}"
    );
}

#[test]
fn a_failed_backfill_says_so_in_place_and_raises_no_toast() {
    let (transport, responses) = lib_units::fake::FakeTransport::channel();
    let mut shell = common::fetching_shell_with(transport.clone(), responses, aed_held_since_2019);
    press(&mut shell, "g u");
    let (id, _) = aed_history(&transport);

    transport.answer(
        id,
        lib_units::TransportOutcome::Failed(lib_units::TransportError::Offline),
    );
    shell.handle_event(Event::Tick);

    let frame = common::screen_sized(&mut shell, 220, 60);
    assert!(
        frame.contains("History failed \u{b7} Offline \u{b7} tries again in"),
        "{frame}"
    );
    assert!(!frame.contains("Couldn't fetch"), "never a Toast:\n{frame}");
    assert!(!frame.contains("fetch failed"), "{frame}");
}
