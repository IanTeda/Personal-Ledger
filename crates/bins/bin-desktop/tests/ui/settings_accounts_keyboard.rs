//! The Settings Accounts page driven by keys: row selection, `enter` opening an Account's ledger,
//! the add, edit and delete dialogs with what confirming each one does, the status line, and the
//! `accounts` command. The dialogs' own forms are in `accounts_keyboard.rs`; the Delete Dialog is
//! `lib-account-deletion`'s on the Entity host, whose rules its Presenter tests cover. State is
//! read back from `Root`.

use super::common;

use bin_desktop::navigation::nav::{InputMode, Noun};
use common::Harness;
use gpui::TestAppContext;

fn mode(ui: &mut Harness<'_>) -> InputMode {
    ui.read(|shell| shell.nav().mode())
}

/// Settings on the Accounts page (four presses down the index) with focus stepped into it.
fn in_accounts(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.press("g s");
    ui.press("j j j j");
    assert_eq!(ui.settings().page, "Accounts");
    ui.press("l");
    assert!(ui.settings().page_focused);
    ui
}

/// Types `text` into the open palette, a key per character.
fn type_text(ui: &mut Harness<'_>, text: &str) {
    for ch in text.chars() {
        ui.press(&if ch == ' ' {
            "space".to_string()
        } else {
            ch.to_string()
        });
    }
}

#[gpui::test]
fn rows_move_with_j_k_and_the_ends(app: &mut TestAppContext) {
    let mut ui = in_accounts(app);

    let first = ui.settings_accounts().selected_account;
    assert_eq!(
        first.as_deref(),
        ui.settings_accounts()
            .account_names
            .first()
            .map(String::as_str)
    );
    ui.press("j");
    assert_eq!(
        ui.settings_accounts().selected_account,
        Some(ui.settings_accounts().account_names[1].clone())
    );
    ui.press("shift-g");
    let accounts_page = ui.settings_accounts();
    assert_eq!(
        accounts_page.selected_account.as_ref(),
        accounts_page.account_names.last()
    );
    ui.press("g g");
    assert_eq!(ui.settings_accounts().selected_account, first);
}

#[gpui::test]
fn n_opens_add_account_and_escape_cancels_it(app: &mut TestAppContext) {
    let mut ui = in_accounts(app);

    ui.press("n");
    assert_eq!(ui.dialog(), Some("AddAccount"));
    assert_eq!(mode(&mut ui), InputMode::Dialog);
    ui.press("escape");
    assert_eq!(ui.dialog(), None);
    assert_eq!(mode(&mut ui), InputMode::Normal);
}

#[gpui::test]
fn typing_a_name_and_pressing_enter_adds_the_account(app: &mut TestAppContext) {
    let mut ui = in_accounts(app);
    let before = ui.settings_accounts().account_names.len();

    ui.press("n");
    ui.press("r a i n y");
    assert_eq!(ui.dialog_value("name").as_deref(), Some("rainy"));
    ui.press("enter");
    let accounts_page = ui.settings_accounts();
    assert_eq!(ui.dialog(), None);
    assert_eq!(accounts_page.account_names.len(), before + 1);
    assert!(
        accounts_page
            .account_names
            .iter()
            .any(|name| name == "rainy")
    );
}

#[gpui::test]
fn a_cash_account_added_from_the_keyboard_names_the_personal_institution(app: &mut TestAppContext) {
    let mut ui = in_accounts(app);

    ui.press("n");
    ui.press("t i n");
    // Tab to Type, step Bank up to Cash, and back to Name, where Enter confirms.
    ui.press("tab tab up shift-tab shift-tab");
    assert_eq!(ui.dialog_value("institution").as_deref(), Some("Personal"));
    ui.press("enter");
    assert_eq!(ui.dialog(), None);
    assert_eq!(
        ui.settings_accounts().selected_account.as_deref(),
        Some("tin")
    );

    ui.press("e");
    assert_eq!(ui.dialog_value("type").as_deref(), Some("Cash"));
    assert_eq!(ui.dialog_value("institution").as_deref(), Some("Personal"));
}

#[gpui::test]
fn e_edits_the_selected_account_and_enter_saves_the_new_name(app: &mut TestAppContext) {
    let mut ui = in_accounts(app);
    let name = ui
        .settings_accounts()
        .selected_account
        .expect("seeded accounts");

    ui.press("e");
    assert_eq!(ui.dialog(), Some("EditAccount"));
    assert_eq!(ui.dialog_value("name").as_deref(), Some(name.as_str()));
    ui.press("x");
    ui.press("enter");
    let accounts_page = ui.settings_accounts();
    assert_eq!(ui.dialog(), None);
    assert!(accounts_page.account_names.contains(&format!("{name}x")));
}

#[gpui::test]
fn d_deletes_an_account_only_once_its_name_is_typed_back(app: &mut TestAppContext) {
    let mut ui = in_accounts(app);
    let before = ui.settings_accounts().account_names.len();
    let name = ui
        .settings_accounts()
        .selected_account
        .expect("seeded accounts");

    ui.press("d");
    assert_eq!(ui.dialog(), Some("DeleteAccount"));
    ui.press("enter");
    assert_eq!(ui.dialog(), Some("DeleteAccount"));
    assert_eq!(ui.settings_accounts().account_names.len(), before);
    for ch in name.chars() {
        ui.press(&if ch == ' ' {
            "space".to_string()
        } else {
            ch.to_string()
        });
    }
    assert_eq!(ui.dialog_value("confirm").as_deref(), Some(name.as_str()));
    ui.press("enter");
    let accounts_page = ui.settings_accounts();
    assert_eq!(ui.dialog(), None);
    assert_eq!(accounts_page.account_names.len(), before - 1);
    assert!(!accounts_page.account_names.contains(&name));
}

#[gpui::test]
fn enter_opens_the_selected_accounts_ledger(app: &mut TestAppContext) {
    let mut ui = in_accounts(app);
    let name = ui
        .settings_accounts()
        .selected_account
        .expect("seeded accounts");

    let before = ui.commits();
    ui.press("enter");
    assert_eq!(ui.noun(), Noun::Transactions);
    assert_eq!(ui.commits(), before + 1, "the key is one input");
    let page = ui.transactions();
    assert_eq!(page.selected, 0);
    assert!(
        page.chips
            .iter()
            .any(|chip| chip.active && chip.label == format!("account: {name}")),
        "the ledger is filtered to {name}"
    );

    ui.press("g s");
    assert_eq!(ui.settings().page, "Accounts");
    assert_eq!(ui.settings_accounts().selected_account, Some(name));
}

#[gpui::test]
fn h_steps_back_to_the_index_and_the_page_keys_stop(app: &mut TestAppContext) {
    let mut ui = in_accounts(app);

    ui.press("h");
    assert!(!ui.settings().page_focused);
    ui.press("n");
    assert_eq!(ui.dialog(), None);
    ui.press("j");
    assert_eq!(ui.settings().page, "Categories", "j walks the index again");
}

#[gpui::test]
fn slash_is_inert_on_the_page(app: &mut TestAppContext) {
    let mut ui = in_accounts(app);

    ui.press("/");
    assert_eq!(mode(&mut ui), InputMode::Normal);
    assert!(ui.settings().page_focused);
}

#[gpui::test]
fn deleting_an_account_toasts_and_selects_the_row_that_took_its_place(app: &mut TestAppContext) {
    let mut ui = in_accounts(app);
    let names = ui.settings_accounts().account_names;
    // ANZ Offset: no Bill Plan pays from it.
    ui.press("j j");
    let name = names[2].clone();

    ui.press("d");
    type_text(&mut ui, &name);
    ui.press("enter");
    assert_eq!(ui.dialog(), None);
    assert_eq!(
        ui.settings_accounts().selected_account.as_ref(),
        names.get(3),
        "the next account slid into the deleted row"
    );
    assert!(
        ui.toasts()
            .history
            .first()
            .is_some_and(|toast| toast.text.contains(&name))
    );
}

#[gpui::test]
fn the_accounts_command_opens_the_page_and_its_dialog(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    let name = ui.settings_accounts().account_names[2].clone();

    ui.press(":");
    type_text(&mut ui, &format!("accounts edit {name}"));
    ui.press("enter");
    assert_eq!(ui.noun(), Noun::Settings);
    let page = ui.settings();
    assert_eq!(page.page, "Accounts");
    assert!(page.page_focused);
    assert_eq!(ui.dialog(), Some("EditAccount"));
    assert_eq!(ui.dialog_value("name").as_deref(), Some(name.as_str()));
    assert_eq!(ui.settings_accounts().selected_account, Some(name));
    assert_eq!(mode(&mut ui), InputMode::Dialog);
}

#[gpui::test]
fn the_accounts_command_names_an_account_it_cannot_find(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);

    ui.press(":");
    type_text(&mut ui, "accounts delete rainy");
    ui.press("enter");
    let page = ui.settings();
    assert_eq!(page.page, "Accounts");
    assert_eq!(ui.dialog(), None);
    assert_eq!(
        page.status_message.as_deref(),
        Some(":accounts delete \u{2014} no account named \"rainy\"")
    );
}

#[gpui::test]
fn the_status_line_shows_the_page_legend_and_the_account_count(app: &mut TestAppContext) {
    let mut ui = in_accounts(app);

    let (keys, right) = ui
        .status()
        .map(|status| {
            let keys: Vec<String> = status.hints.into_iter().map(|(key, _)| key).collect();
            (keys, status.right)
        })
        .unwrap_or_default();
    assert_eq!(keys, ["j/k", "enter", "e", "d", "n"]);
    let accounts = ui.settings_accounts().account_names.len();
    assert!(right.contains(&accounts.to_string()), "{right}");
}

#[gpui::test]
fn a_deactivated_unit_is_missing_from_the_add_account_unit_picker(app: &mut TestAppContext) {
    let mut ui = in_accounts(app);
    ui.press("n");
    let offered = ui.dialog_value("unit_options").unwrap_or_default();
    assert!(offered.split('|').any(|code| code == "btc"));
    ui.press("escape");

    // Deactivate btc from the Units page: it is in use by an Account, so Delete offers it.
    ui.press("escape");
    ui.press("k k");
    assert_eq!(ui.settings().page, "Units");
    ui.press("l");
    ui.press("j");
    ui.press("d");
    assert_eq!(ui.dialog_value("in_use").as_deref(), Some("true"));
    ui.press("enter");
    ui.press("escape");
    ui.press("j j");
    assert_eq!(ui.settings().page, "Accounts");
    ui.press("l");
    ui.press("n");

    let offered = ui.dialog_value("unit_options").unwrap_or_default();
    assert!(!offered.split('|').any(|code| code == "btc"), "{offered}");
    assert!(offered.split('|').any(|code| code == "aud"));
}

#[gpui::test]
fn a_failed_write_puts_the_accounts_back_with_an_error_toast(app: &mut TestAppContext) {
    let mut ui = Harness::with_failing_accounts(app);
    ui.press("g s");
    ui.press("j j j j");
    ui.press("l");
    let stored = ui.settings_accounts().account_names;

    ui.press("n");
    ui.press("r a i n y");
    ui.press("enter");
    assert_eq!(ui.dialog(), None);
    assert_eq!(
        ui.settings_accounts().account_names.len(),
        stored.len() + 1,
        "memory runs ahead of the file until the failure is heard"
    );
    ui.settle_writes();

    assert_eq!(ui.settings_accounts().account_names, stored);
    assert!(ui.toasts().visible.into_iter().any(|toast| {
        toast.kind == "Error"
            && toast.text == "Couldn't save the change to accounts, so they are back as last saved"
    }));
}

/// The legacy Add Dialog writes through the Accounts forwarding shim, so its confirm commits once
/// with the input it ran in, and the page reads the Account back from the store.
#[gpui::test]
fn adding_an_account_commits_once_through_the_shim(app: &mut TestAppContext) {
    let mut ui = in_accounts(app);
    ui.press("n");
    ui.press("r a i n y");
    let before = ui.commits();

    ui.press("enter");

    assert_eq!(ui.commits(), before + 1);
    assert!(
        ui.settings_accounts()
            .account_names
            .iter()
            .any(|name| name == "rainy")
    );
}

/// The Delete Dialog is on the Entity host: it quotes the counts it reads when it opens, and its
/// confirm writes the Account, its Transactions and the Unmatch in the one commit of its input.
#[gpui::test]
fn deleting_an_account_quotes_its_counts_and_commits_once(app: &mut TestAppContext) {
    let mut ui = in_accounts(app);
    let name = ui
        .settings_accounts()
        .selected_account
        .expect("seeded accounts");

    ui.press("d");
    assert_eq!(ui.dialog(), Some("DeleteAccount"));
    let transactions: usize = ui
        .dialog_value("transactions")
        .and_then(|count| count.parse().ok())
        .expect("the Dialog quotes a count");
    assert!(transactions > 0, "the stub books to {name}");
    assert_eq!(
        ui.dialog_value("title").as_deref(),
        Some(format!("Delete account \u{2014} {name}").as_str())
    );
    type_text(&mut ui, &name);
    assert_eq!(ui.dialog_value("can_confirm").as_deref(), Some("true"));
    let before = ui.commits();

    ui.press("enter");

    assert_eq!(ui.dialog(), None);
    assert_eq!(ui.commits(), before + 1);
    assert!(!ui.settings_accounts().account_names.contains(&name));
    assert!(ui.toasts().history.first().is_some_and(|toast| toast.text
        == format!("Deleted account {name} \u{b7} {transactions} transactions deleted")));
}

/// An Account a Bill Plan pays from says how many and never confirms, so `enter` writes nothing.
#[gpui::test]
fn an_account_a_bill_plan_pays_from_cannot_be_deleted(app: &mut TestAppContext) {
    let mut ui = in_accounts(app);
    ui.press("j");
    let name = ui
        .settings_accounts()
        .selected_account
        .expect("seeded accounts");
    assert_eq!(name, "ANZ Everyday");

    ui.press("d");
    type_text(&mut ui, &name);
    ui.press("enter");

    assert_eq!(ui.dialog(), Some("DeleteAccount"));
    assert_eq!(ui.dialog_value("can_confirm").as_deref(), Some("false"));
    assert!(
        ui.dialog_value("blocked")
            .is_some_and(|text| text.starts_with("6 bills are paid from this account")),
        "{:?}",
        ui.dialog_value("blocked")
    );
    ui.press("escape");
    assert_eq!(ui.dialog(), None);
    assert!(ui.settings_accounts().account_names.contains(&name));
}

#[gpui::test]
fn question_mark_lists_the_keys_the_status_line_shows(app: &mut TestAppContext) {
    let mut ui = in_accounts(app);

    ui.assert_cheat_sheet_matches_status("Settings keys");
}

#[gpui::test]
fn the_cheat_sheet_and_the_legend_show_a_rebound_key(app: &mut TestAppContext) {
    let mut ui = Harness::with_keybindings(app, common::overriding("new", "ctrl+n"));
    ui.press("g s");
    ui.press("j j j j");
    ui.press("l");
    let keys: Vec<String> = ui
        .status()
        .map(|status| status.hints.into_iter().map(|(key, _)| key).collect())
        .unwrap_or_default();
    assert_eq!(keys, ["j/k", "enter", "e", "d", "ctrl+n"]);

    ui.assert_cheat_sheet_matches_status("Settings keys");
    ui.press("escape");
    ui.press("ctrl-n");
    assert_eq!(ui.dialog(), Some("AddAccount"));
}

#[gpui::test]
fn the_accounts_new_command_prefills_the_name(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);

    ui.press(":");
    type_text(&mut ui, "accounts new Rainy Day");
    ui.press("enter");
    assert_eq!(ui.dialog(), Some("AddAccount"));
    assert_eq!(ui.dialog_value("name").as_deref(), Some("Rainy Day"));
}
