//! Keyboard tests for the Units View on the `lib-units` Presenters (ADR-0035): a key reaches the
//! page or Dialog Presenter and its view model is drawn. The rules themselves are the Presenter
//! tests' in `lib-units`; these check the wiring, and that the TUI shows the same refusals.

mod common;

use bin_tui::event::Event;
use common::{press, screen, shell, shell_with_failures, type_text};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use lib_database::{Domain, WriteFailure};

#[test]
fn g_u_draws_the_stub_units_and_price_sources() {
    let mut shell = shell();

    press(&mut shell, "g u");
    let frame = screen(&mut shell);

    assert!(frame.contains("UNITS"), "{frame}");
    for code in ["aud", "btc", "vas"] {
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
    type_text(&mut shell, "eur");
    press(&mut shell, "Tab Tab");
    type_text(&mut shell, "Euro");
    press(&mut shell, "Enter");
    let frame = screen(&mut shell);

    assert!(!frame.contains("INSERT"), "the Dialog closed:\n{frame}");
    assert!(frame.contains("eur"), "{frame}");
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
        frame.contains(&lib_units::msg::units_deactivate_title("aud")),
        "{frame}"
    );
    assert!(
        frame.contains("aud is used by") && frame.contains("locations"),
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
    type_text(&mut shell, "xau");
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

    type_text(&mut shell, "xau");
    press(&mut shell, "Enter");
    let frame = screen(&mut shell);

    assert!(frame.contains("Deleted unit xau"), "{frame}");
    assert!(
        frame.contains("Currency \u{b7} base"),
        "aud, the first candidate, is the base:\n{frame}"
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
    assert!(open.contains("Set price — vas"), "{open}");

    type_text(&mut shell, "42");
    press(&mut shell, "Enter");
    let frame = screen(&mut shell);

    assert!(
        !frame.contains("Set price — vas"),
        "the Dialog closed:\n{frame}"
    );
    assert!(frame.contains("Set the price of vas"), "{frame}");
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
    // xau, which nothing in the stub uses, with an Account in it in the file.
    common::ledger_file_with(&path, |storage| {
        let mut xau = storage
            .units
            .units()
            .unit(lib_core::UnitId::SEED_BTC)
            .expect("the seed has btc");
        xau.id = lib_core::UnitId::new();
        xau.code = "xau".to_string();
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
        frame.contains(&lib_units::msg::units_deactivate_title("xau")),
        "refused before the commit, offering Deactivate:\n{frame}"
    );
    assert!(frame.contains("xau is used by"), "{frame}");
}

#[test]
fn deleting_a_unit_a_budget_in_the_opened_file_uses_is_refused() {
    use lib_budgets::BudgetRepository as _;

    let directory = tempfile::tempdir().expect("a temporary directory");
    let path = directory.path().join("household.pldb");
    // xau, which nothing in the stub uses, with only a Budget in it in the file (#952).
    common::ledger_file_with(&path, |storage| {
        let mut xau = storage
            .units
            .units()
            .unit(lib_core::UnitId::SEED_BTC)
            .expect("the seed has btc");
        xau.id = lib_core::UnitId::new();
        xau.code = "xau".to_string();
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
        frame.contains(&lib_units::msg::units_deactivate_title("xau")),
        "refused before the commit, offering Deactivate:\n{frame}"
    );
    assert!(
        frame.contains("xau is used by") && frame.contains("budget"),
        "{frame}"
    );
}

#[test]
fn a_unit_only_the_stub_uses_can_be_deleted_from_the_opened_file() {
    use lib_database::InstitutionsStore;

    let directory = tempfile::tempdir().expect("a temporary directory");
    let path = directory.path().join("household.pldb");
    // The stub's btc Accounts are in aud in the file, and its Institution has no Usual unit, so
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
        !dialog.contains(&lib_units::msg::units_deactivate_title("btc")),
        "the stub's usage doesn't count:\n{dialog}"
    );
    type_text(&mut shell, "btc");
    press(&mut shell, "Enter");
    let frame = screen(&mut shell);

    assert!(frame.contains("Deleted unit btc"), "{frame}");
}
