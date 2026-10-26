//! The **Accounts** Settings page (ADR-0037's amendment: one folder per page). [`AccountsPage`] is
//! the page's Entity: it owns the selection, renders itself (`render.rs`), handles the page's keys
//! and movement, builds its Dialogs and reaches `Root` only through [`ViewEvent`]. The Dialog host
//! holds the Dialogs, and they draw, read back and confirm themselves (`dialogs/`, `apply.rs`),
//! handing each outcome back to [`AccountsPage::land`]. The Settings View forwards to the page
//! while it is open.

pub mod apply;
pub mod dialogs;
pub mod form;
pub(crate) mod hints;
mod render;
pub mod snapshot;

use crate::settings::SettingsPage;
use crate::settings::pages::Page;
use gpui::{AnyElement, IntoElement as _};
use gpui::{App, Context, Entity, EventEmitter, Keystroke, Subscription};
use lib_accounts::{Account, NO_INSTITUTION, NameLookup, NetWorth, display_order, find_by_name};
use lib_core::{AccountId, InstitutionId};
use lib_institutions::Institution;
use lib_ledger::Domain;
use lib_locale::Label;

use apply::AccountOutcome;
use dialogs::AccountsDialog;
use form::{AccountForm, AccountOptions, DeleteAccountForm};
use hints::accounts_hints;

use crate::{
    form::field::TextField,
    ledger_store::{LedgerStore, notify_on},
    navigation::{command::AccountsVerb, movement::Movement},
    shell::view::{
        dialog_host::open_dialog,
        statusline::{Hint, PageStatus, owned_hints},
    },
    view::event::{Reveal, ViewEvent},
};

/// What to show for an account's `institution`: the placeholder's Message in the Locale in
/// effect, or the name of the one of `institutions` it names, unchanged (Ledger data, not a
/// Message). An id `institutions` doesn't hold shows nothing.
pub fn institution_label(institution: InstitutionId, institutions: &[Institution]) -> String {
    if institution == NO_INSTITUTION {
        lib_locale::msg::institution_none()
    } else {
        lib_institutions::get(institutions, institution)
            .map_or_else(String::new, |institution| institution.name.clone())
    }
}

/// `"7 accounts"`, or `"1 account"`.
pub fn count_text(net: &NetWorth) -> String {
    crate::msg::desktop_accounts_count(i64::try_from(net.account_count).unwrap_or(i64::MAX))
}

/// `"vas, btc held separately"`, or `None` when every account is in the base Unit.
pub fn held_separately_text(net: &NetWorth, units: &[lib_units::UnitRow]) -> Option<String> {
    (!net.held_separately.is_empty()).then(|| {
        let codes: Vec<&str> = net
            .held_separately
            .iter()
            .map(|id| {
                units
                    .iter()
                    .find(|unit| unit.id == *id)
                    .map_or("", |unit| unit.code.as_str())
            })
            .collect();
        crate::msg::desktop_accounts_held_separately(&codes.join(", "))
    })
}

/// The Accounts group labels as Messages, in [`lib_accounts::GROUP_ORDER`]'s order.
pub fn group_labels() -> Vec<String> {
    lib_accounts::GROUP_ORDER.iter().map(Label::label).collect()
}

/// The three selects' option lists, read live from Settings (so an institution added there
/// appears in the Accounts dialogs) and the fixed type order. A deactivated Unit is left out of the Unit
/// picker, except `keep`: the Unit an Account being edited is already on, so its read-only Unit
/// field still resolves.
pub fn dialog_options(
    ledger: &Entity<LedgerStore>,
    keep: Option<lib_core::UnitId>,
    cx: &App,
) -> AccountOptions {
    AccountOptions::new(
        ledger
            .read(cx)
            .ledger()
            .institutions()
            .iter()
            .map(|institution| (institution.id, institution.name.clone()))
            .collect(),
        ledger
            .read(cx)
            .ledger()
            .units()
            .iter()
            .filter(|unit| unit.is_active || Some(unit.id) == keep)
            .map(|unit| (unit.id, unit.code.clone()))
            .collect(),
    )
}

impl EventEmitter<ViewEvent> for AccountsPage {}

/// The Accounts page. Its rows and the Units its heading reads come from [`LedgerStore`], which
/// holds the only copy. The selection is a position in [`display_order`], not an index into the
/// rows, because the page groups accounts by type.
pub struct AccountsPage {
    ledger: Entity<LedgerStore>,
    selected: usize,
    /// Whether the page, not Settings' index, has keyboard focus, told by the Settings View as it
    /// draws the page.
    pub(crate) page_focused: bool,
    /// Repaint the page when the Accounts or the Units it reads change.
    _accounts_changed: Subscription,
    _units_changed: Subscription,
}

impl AccountsPage {
    pub fn new(ledger: Entity<LedgerStore>, cx: &mut Context<'_, Self>) -> Self {
        let _accounts_changed = notify_on(&ledger, Domain::Accounts, cx);
        let _units_changed = notify_on(&ledger, Domain::Units, cx);
        Self {
            ledger,
            selected: 0,
            page_focused: false,
            _accounts_changed,
            _units_changed,
        }
    }

    pub(crate) fn accounts<'a>(&self, cx: &'a App) -> &'a [Account] {
        self.ledger.read(cx).ledger().accounts()
    }
}

impl Page for AccountsPage {
    const PAGE: SettingsPage = SettingsPage::Accounts;

    fn set_page_focused(&mut self, focused: bool) {
        self.page_focused = focused;
    }

    fn hints(&self, _cx: &App) -> Vec<Hint> {
        owned_hints(accounts_hints())
    }

    fn heading_meta(&self, cx: &App) -> AnyElement {
        self.scope_note(cx).into_any_element()
    }

    /// The page's bare `n`, `e` and `d`, each through the same handler its button uses. Returns
    /// whether the key was one of them.
    fn handle_key(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        let modifiers = &keystroke.modifiers;
        if modifiers.control || modifiers.alt || modifiers.platform || modifiers.shift {
            return false;
        }
        match keystroke.key.as_str() {
            "n" => self.open_add_dialog("", cx),
            "e" => {
                if let Some(id) = self.selected_id(cx) {
                    self.open_edit_dialog(id, cx);
                }
            }
            "d" => {
                if let Some(id) = self.selected_id(cx) {
                    self.open_delete_dialog(id, cx);
                }
            }
            _ => return false,
        }
        true
    }

    /// `j`/`k`/`g`/`G`/`Ctrl-d`/`Ctrl-u` step the row selection. `enter` opens the selected
    /// account's ledger: Transactions filtered to it.
    fn apply_movement(&mut self, movement: Movement, cx: &mut Context<'_, Self>) -> bool {
        let len = self.accounts(cx).len();
        match movement.step() {
            Some(step) => self.selected = lib_list_cursor::step(self.selected, len, step),
            None => {
                if let Some(id) = self.selected_id(cx) {
                    open_ledger(id, cx);
                }
            }
        }
        cx.notify();
        true
    }

    /// Selects an Account another View handed off.
    fn reveal(&mut self, reveal: Reveal, cx: &mut Context<'_, Self>) {
        if let Reveal::Account(id) = reveal {
            self.select_id(id, cx);
        }
    }

    /// The page's legend and its account count.
    fn page_status(&self, cx: &App) -> Option<PageStatus> {
        Some(PageStatus {
            hints: owned_hints(accounts_hints()),
            right: crate::msg::desktop_accounts_count(
                i64::try_from(self.accounts(cx).len()).unwrap_or(i64::MAX),
            ),
        })
    }
}

impl AccountsPage {
    // -----------------------------------------------------------------------------------------
    // Selection
    // -----------------------------------------------------------------------------------------

    /// The selected account's index in the store's rows, `None` when there are none. The stored
    /// position is clamped, so removing accounts can never leave it pointing past the end.
    pub fn selected_index(&self, cx: &App) -> Option<usize> {
        let order = display_order(self.accounts(cx));
        order
            .get(self.selected.min(order.len().saturating_sub(1)))
            .copied()
    }

    /// The selected account's id, or `None` when the page has no rows.
    pub fn selected_id(&self, cx: &App) -> Option<AccountId> {
        let index = self.selected_index(cx)?;
        self.accounts(cx).get(index).map(|account| account.id)
    }

    /// Selects the row for account `id`, if it still exists.
    pub fn select_id(&mut self, id: AccountId, cx: &mut Context<'_, Self>) {
        let rows = self.accounts(cx);
        let Some(index) = rows.iter().position(|account| account.id == id) else {
            return;
        };
        if let Some(position) = display_order(rows).iter().position(|&i| i == index) {
            self.selected = position;
            cx.notify();
        }
    }

    /// Lands a confirmed Dialog. Add and Edit select the account (an Edit regroups it if its Type
    /// changed); Delete keeps the selection in range, so it lands on the account that slid into the
    /// deleted row's place (or the last one). The Toast is the host's.
    fn land(&mut self, outcome: AccountOutcome, cx: &mut Context<'_, Self>) {
        match outcome {
            AccountOutcome::Saved(id) => self.select_id(id, cx),
            AccountOutcome::Deleted { .. } => self.clamp_selection(cx),
        }
    }

    /// Keeps the selection in range after rows are removed, so it lands on the account that
    /// slid into the deleted row's place (or the last one).
    pub fn clamp_selection(&mut self, cx: &mut Context<'_, Self>) {
        let len = self.accounts(cx).len();
        self.selected = self.selected.min(len.saturating_sub(1));
        cx.notify();
    }

    // -----------------------------------------------------------------------------------------
    // Clicks
    // -----------------------------------------------------------------------------------------

    /// A click on an account row: selects it and, as `enter` does, opens its ledger.
    pub(crate) fn click_row(&mut self, id: AccountId, cx: &mut Context<'_, Self>) {
        self.select_id(id, cx);
        open_ledger(id, cx);
    }

    /// **+ Add account**: the same handler `n` reaches.
    pub(crate) fn click_add(&mut self, cx: &mut Context<'_, Self>) {
        self.open_add_dialog("", cx);
    }

    pub(crate) fn click_edit(&mut self, id: AccountId, cx: &mut Context<'_, Self>) {
        self.select_id(id, cx);
        self.open_edit_dialog(id, cx);
    }

    pub(crate) fn click_delete(&mut self, id: AccountId, cx: &mut Context<'_, Self>) {
        self.select_id(id, cx);
        self.open_delete_dialog(id, cx);
    }

    // -----------------------------------------------------------------------------------------
    // Dialogs and the `accounts` command
    // -----------------------------------------------------------------------------------------

    fn dialog_options(&self, keep: Option<lib_core::UnitId>, cx: &App) -> AccountOptions {
        dialog_options(&self.ledger, keep, cx)
    }

    /// Opens the Add account dialog on a fresh form (Unit starting on Settings' default Unit),
    /// with Name pre-filled from `name` -- empty for `n` and the button, the typed argument for
    /// `accounts new <account name>`.
    fn open_add_dialog(&mut self, name: &str, cx: &mut Context<'_, Self>) {
        let options = self.dialog_options(None, cx);
        let default_unit = self
            .ledger
            .read(cx)
            .ledger()
            .base_unit_row()
            .map(|unit| unit.code.as_str());
        let mut form = AccountForm::new(&options, default_unit);
        form.name = TextField::new(name.trim());
        open_dialog(AccountsDialog::Add(form), Self::land, cx);
    }

    /// Opens the Edit account dialog on `id`, pre-filled. A no-op if the account is gone.
    fn open_edit_dialog(&mut self, id: AccountId, cx: &mut Context<'_, Self>) {
        let Some(account) = self.accounts(cx).iter().find(|account| account.id == id) else {
            return;
        };
        let options = self.dialog_options(Some(account.unit), cx);
        let form = AccountForm::from_account(account, &options);
        open_dialog(AccountsDialog::Edit(id, form), Self::land, cx);
    }

    /// Opens the Delete account dialog on `id`. A no-op if the account is gone.
    fn open_delete_dialog(&mut self, id: AccountId, cx: &mut Context<'_, Self>) {
        let Some(account) = self.accounts(cx).iter().find(|account| account.id == id) else {
            return;
        };
        let form = DeleteAccountForm::new(account.name.as_str());
        open_dialog(AccountsDialog::Delete(id, form), Self::land, cx);
    }

    /// `accounts new|edit|delete [<account name>]`, once `Root` has opened the page: opens the
    /// same dialog the page's `n`/`e`/`d` and buttons do. `new` pre-fills Name with the argument.
    /// `edit` and `delete` resolve the typed name ([`find_by_name`]), or use the selected row when
    /// none is given; a name that fits nothing or several accounts puts a status-line message
    /// naming the problem rather than guessing.
    pub fn run_command(
        &mut self,
        command_name: &str,
        verb: AccountsVerb,
        argument: &str,
        cx: &mut Context<'_, Self>,
    ) {
        if verb == AccountsVerb::New {
            self.open_add_dialog(argument, cx);
            return;
        }
        let command = format!(":{command_name}");
        let index = if argument.is_empty() {
            self.selected_index(cx)
                .ok_or_else(|| crate::msg::desktop_status_no_accounts(&command))
        } else {
            match find_by_name(self.accounts(cx), argument) {
                NameLookup::Found(index) => Ok(index),
                NameLookup::NotFound => Err(crate::msg::desktop_status_no_account_named(
                    &command, argument,
                )),
                NameLookup::Ambiguous(names) => Err(crate::msg::desktop_status_account_ambiguous(
                    &command,
                    argument,
                    &names.join(", "),
                )),
            }
        };
        let index = match index {
            Ok(index) => index,
            Err(message) => {
                cx.emit(ViewEvent::SetStatus(Some(message)));
                return;
            }
        };
        let id = self.accounts(cx)[index].id;
        self.select_id(id, cx);
        match verb {
            AccountsVerb::Edit => self.open_edit_dialog(id, cx),
            AccountsVerb::Delete => self.open_delete_dialog(id, cx),
            AccountsVerb::New => {}
        }
    }
}

/// Asks `Root` for the account's ledger: Transactions filtered to it, from their first row. The
/// selection is untouched, so returning to Accounts finds the same row selected.
fn open_ledger(id: AccountId, cx: &mut Context<'_, AccountsPage>) {
    cx.emit(ViewEvent::Reveal(Reveal::AccountTransactions(id)));
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use lib_accounts::{NO_INSTITUTION, default_accounts, net_worth};
    use lib_core::{AccountType, Money};

    use super::*;

    fn date(year: i32, month: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, 1).expect("test dates are valid")
    }

    fn account(id: AccountId, name: &str, account_type: AccountType) -> Account {
        Account {
            id,
            name: name.to_string(),
            institution: NO_INSTITUTION,
            account_type,
            unit: lib_core::UnitId::SEED_AUD,
            balance: money("0"),
            account_number: None,
            opened_at: date(2024, 1),
        }
    }

    fn money(text: &str) -> Money {
        text.parse().expect("test amounts are valid decimals")
    }

    #[test]
    fn group_order_puts_loan_before_investment() {
        let labels = group_labels();
        assert_eq!(
            labels,
            vec!["Cash", "Bank", "Credit card", "Loan", "Investment"]
        );
    }

    #[test]
    fn seeded_net_worth_text_names_the_held_separately_units() {
        crate::locale::init_for_tests();
        let net = net_worth(&default_accounts(), Some(lib_core::UnitId::SEED_AUD));
        assert_eq!(count_text(&net), "7 accounts");
        assert_eq!(
            held_separately_text(&net, &lib_units::default_units(chrono::Utc::now())).as_deref(),
            Some("vas, btc held separately")
        );
    }

    #[test]
    fn account_type_is_still_in_scope_for_the_group_labels() {
        assert_eq!(lib_accounts::GROUP_ORDER[0], AccountType::Cash);
    }

    #[test]
    fn institution_label_renders_the_placeholder_and_names_an_institution_by_its_id() {
        lib_locale::with_locale(lib_locale::Locale::EnUs, || {
            let institutions = lib_institutions::default_institutions();
            assert_eq!(
                institution_label(NO_INSTITUTION, &institutions),
                "No institution"
            );
            assert_eq!(
                institution_label(InstitutionId::SEED_ANZ, &institutions),
                "ANZ Banking Group"
            );
            assert_eq!(institution_label(InstitutionId::new(), &institutions), "");
        });
    }

    #[test]
    fn net_worth_count_text_is_singular_for_one_account() {
        crate::locale::init_for_tests();
        let net = net_worth(
            &[account(AccountId::seed(1), "Only", AccountType::Cash)],
            Some(lib_core::UnitId::SEED_AUD),
        );
        assert_eq!(count_text(&net), "1 account");
        assert_eq!(
            held_separately_text(&net, &lib_units::default_units(chrono::Utc::now())),
            None
        );
    }

    #[test]
    fn seed_rows_parse_and_link() {
        let accounts = default_accounts();
        assert_eq!(accounts.len(), 7);

        let mut ids: Vec<_> = accounts.iter().map(|a| a.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), accounts.len(), "ids must be unique");

        let units = lib_units::default_units(chrono::Utc::now());
        let institutions = lib_institutions::default_institutions();
        for account in &accounts {
            assert!(
                units.iter().any(|unit| unit.id == account.unit),
                "{} names an unknown unit {}",
                account.name,
                account.unit
            );
            assert!(
                account.institution == NO_INSTITUTION
                    || lib_institutions::get(&institutions, account.institution).is_some(),
                "{} names an unknown institution {}",
                account.name,
                account.institution
            );
        }
    }

    #[test]
    fn seeded_base_unit_rows_sum_to_the_mockups_net_worth() {
        let base = Some(lib_core::UnitId::SEED_AUD);
        let net = default_accounts()
            .into_iter()
            .filter(|account| Some(account.unit) == base)
            .fold(money("0"), |sum, account| Money(sum.0 + account.balance.0));
        assert_eq!(net, money("83995.87"));
    }
}
