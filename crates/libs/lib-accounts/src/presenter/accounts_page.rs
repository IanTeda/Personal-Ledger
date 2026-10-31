//! The Accounts Settings page (`docs/ux/desktop-mockups/16-settings/README.md`'s 2o): one table per
//! Account type in [`GROUP_ORDER`](crate::GROUP_ORDER), the Accounts within a type in the order
//! they were added, with `edit · delete` per row, and the heading's count, net worth in the base
//! Unit and the Units held separately.
//!
//! The selection is an Account id, so it follows its row when a type changes. It may name an
//! Account that has since gone, so it is resolved on every read: a gone Account leaves the first
//! row selected. A delete that lands is the exception: the Presenter remembers where the row was
//! when Delete opened, and selects the Account that slid into its place.

use lib_core::{AccountId, UnitId};
use lib_list_cursor::{Step, resolve, step_id};
use lib_locale::Label;
use lib_locale::format::{AmountStyle, format_amount};
use lib_toast::Notice;
use lib_viper::keymap::{DELETE, EDIT, Keymap, MOVE_DOWN, MOVE_UP, NEW, SELECT};
use lib_viper::view::Hint;

use super::{count, institution_label};
use crate::protocols::{AccountUnit, AccountsDialog, AccountsOutcome, AccountsRouter, SharedPorts};
use crate::{Account, NameLookup, display_order, find_by_name, group_accounts, msg, net_worth};

/// The verb of a typed `:accounts` command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountsCommand {
    New,
    Edit,
    Delete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountsPageEvent {
    /// `j`/`k`, `g g`, `G`, `Ctrl-d`/`Ctrl-u` and their arrow, Home/End and Page keys.
    Move(Step),
    /// `enter`: opens the selected Account's ledger.
    OpenLedger,
    /// A click on a row: selects it and opens its ledger, as `enter` does.
    Open(AccountId),
    /// New opens Add; Edit and Delete act on the selected Account.
    New,
    Edit,
    Delete,
    /// A row's own **edit** or **delete**: selects the row, then opens its Dialog.
    EditRow(AccountId),
    DeleteRow(AccountId),
    /// Another View handed off Account `id`: selects it, if it still exists.
    Reveal(AccountId),
    /// The open Dialog closed, with what its confirm did or `None` when cancelled.
    DialogClosed(Option<AccountsOutcome>),
    /// The Delete Dialog removed the Account Delete last opened on: the selection lands on the
    /// row that slid into its place.
    Deleted,
    /// A typed `accounts new|edit|delete [<account name>]`. `command` is what was typed, for the
    /// status line's refusals. `New` opens Add with Name pre-filled from `argument`; the others
    /// resolve it to an Account by name, or use the selected row when it is empty.
    Command {
        command: String,
        verb: AccountsCommand,
        argument: String,
    },
}

/// The page as drawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountsPageViewModel {
    /// The heading's meta: `7 accounts · net worth 83,995.87 aud · vas, btc held separately`.
    pub scope: String,
    /// The count alone, for the status line.
    pub status: String,
    pub kicker: String,
    pub add_button: String,
    pub columns: AccountColumns,
    /// One block per type that has Accounts, in the fixed type order.
    pub groups: Vec<AccountGroupView>,
    /// What shows in place of the tables while there are no Accounts.
    pub empty: Option<String>,
    pub row_edit: String,
    pub row_delete: String,
    pub selected: Option<AccountId>,
    pub hints: Vec<Hint>,
    /// The Dialog the page last opened, until it closes.
    pub open_dialog: Option<AccountsDialog>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountColumns {
    pub name: String,
    pub institution: String,
    pub unit: String,
    pub actions: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountGroupView {
    /// The type's label, as written; the View sets the case.
    pub label: String,
    pub lines: Vec<AccountLine>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountLine {
    pub id: AccountId,
    pub name: String,
    /// The name of the Institution it names, as the Ledger holds it.
    pub institution: String,
    /// The Unit's code.
    pub unit: String,
}

pub struct AccountsPagePresenter {
    ports: SharedPorts,
    keymap: Keymap<AccountsPageEvent>,
    selected: Option<AccountId>,
    open_dialog: Option<AccountsDialog>,
    /// Where the row Delete last opened on stood among the rows, for the selection a delete leaves.
    delete_position: Option<usize>,
}

impl AccountsPagePresenter {
    /// `keymap` is the one the Client looks keys up in, so the legend shows the chords it binds.
    pub fn new(ports: SharedPorts, keymap: Keymap<AccountsPageEvent>) -> Self {
        Self {
            ports,
            keymap,
            selected: None,
            open_dialog: None,
            delete_position: None,
        }
    }

    pub fn handle(&mut self, event: AccountsPageEvent, router: &mut dyn AccountsRouter) {
        match event {
            AccountsPageEvent::Move(step) => {
                self.selected = step_id(&self.ids(), self.selected(), step);
            }
            AccountsPageEvent::OpenLedger => {
                if let Some(id) = self.selected() {
                    router.open_ledger(id);
                }
            }
            AccountsPageEvent::Open(id) => {
                self.select(id);
                router.open_ledger(id);
            }
            AccountsPageEvent::Reveal(id) => self.select(id),
            AccountsPageEvent::New => self.open_add(String::new(), router),
            AccountsPageEvent::Edit => {
                if let Some(id) = self.selected() {
                    self.open_edit(id, router);
                }
            }
            AccountsPageEvent::Delete => {
                if let Some(id) = self.selected() {
                    self.open_delete(id, router);
                }
            }
            AccountsPageEvent::EditRow(id) => {
                self.select(id);
                self.open_edit(id, router);
            }
            AccountsPageEvent::DeleteRow(id) => {
                self.select(id);
                self.open_delete(id, router);
            }
            AccountsPageEvent::DialogClosed(outcome) => {
                self.open_dialog = None;
                if let Some(AccountsOutcome { id }) = outcome {
                    self.select(id);
                }
            }
            AccountsPageEvent::Deleted => self.land_delete(),
            AccountsPageEvent::Command {
                command,
                verb,
                argument,
            } => self.command(&command, verb, &argument, router),
        }
    }

    pub fn view_model(&self) -> AccountsPageViewModel {
        let ports = self.ports.borrow();
        let accounts = ports.accounts().accounts();
        let institutions = ports.institutions().account_institutions();
        let units = ports.units().account_units();
        let unit_code = |id: UnitId| {
            units
                .iter()
                .find(|unit| unit.id == id)
                .map_or_else(String::new, |unit| unit.code.clone())
        };
        let groups: Vec<AccountGroupView> = group_accounts(&accounts)
            .into_iter()
            .map(|group| AccountGroupView {
                label: group.account_type.label(),
                lines: group
                    .indices
                    .into_iter()
                    .filter_map(|index| accounts.get(index))
                    .map(|account| AccountLine {
                        id: account.id,
                        name: account.name.clone(),
                        institution: institution_label(account.institution, &institutions),
                        unit: unit_code(account.unit),
                    })
                    .collect(),
            })
            .collect();
        let selected = resolve(self.ids_of(&accounts), self.selected);
        AccountsPageViewModel {
            scope: scope_note(&accounts, &units, ports.units().base_unit()),
            status: msg::accounts_count(count(accounts.len())),
            kicker: msg::accounts_kicker(),
            add_button: msg::accounts_add_button("+"),
            columns: AccountColumns {
                name: lib_locale::msg::column_name(),
                institution: lib_locale::msg::column_institution(),
                unit: lib_locale::msg::column_unit(),
                actions: lib_locale::msg::column_actions(),
            },
            groups,
            empty: accounts
                .is_empty()
                .then(|| msg::accounts_empty(self.chord(NEW))),
            row_edit: msg::accounts_row_edit(),
            row_delete: msg::accounts_row_delete(),
            selected,
            hints: self.hints(),
            open_dialog: self.open_dialog.clone(),
        }
    }

    /// The Account ids in the order the page lists them.
    fn ids(&self) -> Vec<AccountId> {
        let accounts = self.ports.borrow().accounts().accounts();
        self.ids_of(&accounts)
    }

    fn ids_of(&self, accounts: &[Account]) -> Vec<AccountId> {
        display_order(accounts)
            .into_iter()
            .filter_map(|index| accounts.get(index))
            .map(|account| account.id)
            .collect()
    }

    /// The selected Account: the stored id while it still exists, else the first row.
    fn selected(&self) -> Option<AccountId> {
        resolve(self.ids(), self.selected)
    }

    /// Selects Account `id`, if it still exists.
    fn select(&mut self, id: AccountId) {
        if self.ids().contains(&id) {
            self.selected = Some(id);
        }
    }

    fn open(&mut self, dialog: AccountsDialog, router: &mut dyn AccountsRouter) {
        if router.open_dialog(dialog.clone()) {
            self.open_dialog = Some(dialog);
        }
    }

    fn open_add(&mut self, name: String, router: &mut dyn AccountsRouter) {
        self.open(AccountsDialog::AddAccount { name }, router);
    }

    fn open_edit(&mut self, id: AccountId, router: &mut dyn AccountsRouter) {
        self.open(AccountsDialog::EditAccount(id), router);
    }

    /// Asks the Router for the Delete Dialog, remembering the row's place for the selection a
    /// confirmed delete leaves.
    fn open_delete(&mut self, id: AccountId, router: &mut dyn AccountsRouter) {
        let position = self.ids().iter().position(|row| *row == id);
        if router.open_delete(id) {
            self.delete_position = position;
        }
    }

    /// Selects the row that slid into the deleted one's place, or the last when it was last.
    fn land_delete(&mut self) {
        let ids = self.ids();
        let position = self.delete_position.take().unwrap_or(0);
        self.selected = ids.get(position.min(ids.len().saturating_sub(1))).copied();
    }

    /// `accounts new|edit|delete [<account name>]`. A name that fits nothing or several Accounts
    /// puts a status-line message naming the problem rather than guessing.
    fn command(
        &mut self,
        command: &str,
        verb: AccountsCommand,
        argument: &str,
        router: &mut dyn AccountsRouter,
    ) {
        if verb == AccountsCommand::New {
            self.open_add(argument.trim().to_string(), router);
            return;
        }
        let command = format!(":{command}");
        let accounts = self.ports.borrow().accounts().accounts();
        let found = if argument.is_empty() {
            self.selected()
                .ok_or_else(|| msg::accounts_status_none(&command))
        } else {
            match find_by_name(&accounts, argument) {
                NameLookup::Found(index) => Ok(accounts[index].id),
                NameLookup::NotFound => Err(msg::accounts_status_not_found(&command, argument)),
                NameLookup::Ambiguous(names) => Err(msg::accounts_status_ambiguous(
                    &command,
                    argument,
                    &names.join(", "),
                )),
            }
        };
        let id = match found {
            Ok(id) => id,
            Err(message) => {
                router.notify(Notice::Status(message));
                return;
            }
        };
        self.select(id);
        match verb {
            AccountsCommand::Edit => self.open_edit(id, router),
            AccountsCommand::Delete => self.open_delete(id, router),
            AccountsCommand::New => {}
        }
    }

    fn chord(&self, command: &str) -> &str {
        self.keymap.chord(command).unwrap_or_default()
    }

    fn hints(&self) -> Vec<Hint> {
        let hint = |chord: String, action: String| Hint { chord, action };
        vec![
            hint(
                format!("{}/{}", self.chord(MOVE_DOWN), self.chord(MOVE_UP)),
                msg::accounts_hint_row(),
            ),
            hint(
                self.chord(SELECT).to_string(),
                msg::accounts_hint_open_ledger(),
            ),
            hint(self.chord(EDIT).to_string(), msg::accounts_hint_edit()),
            hint(self.chord(DELETE).to_string(), msg::accounts_hint_delete()),
            hint(self.chord(NEW).to_string(), msg::accounts_hint_new()),
        ]
    }
}

/// The heading's meta: `7 accounts · net worth 83,995.87 aud · vas, btc held separately`. One net
/// figure in the base Unit only, then every other Unit held named rather than summed or dropped.
fn scope_note(accounts: &[Account], units: &[AccountUnit], base: Option<UnitId>) -> String {
    if accounts.is_empty() {
        return msg::accounts_summary_empty();
    }
    let net = net_worth(accounts, base);
    let mut parts = vec![msg::accounts_count(count(net.account_count))];
    if let Some(base) = net
        .base_unit
        .and_then(|id| units.iter().find(|unit| unit.id == id))
    {
        let figure = format_amount(&net.base_net, AmountStyle::fixed(u32::from(base.precision)));
        let sign = if figure.is_negative { "-" } else { "" };
        parts.push(format!(
            "{} {sign}{} {}",
            msg::accounts_net_worth(),
            figure.text,
            base.code
        ));
    }
    if !net.held_separately.is_empty() {
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
        parts.push(msg::accounts_held_separately(&codes.join(", ")));
    }
    parts.join(" \u{b7} ")
}

#[cfg(test)]
mod tests {
    use super::super::test_support::stub;
    use super::*;
    use crate::AccountRepository as _;
    use crate::fake::{RecordingRouter, RouterCall};
    use crate::keymap::accounts_page_keymap;
    use lib_config::KeyBindingConfig;

    struct Page {
        presenter: AccountsPagePresenter,
        router: RecordingRouter,
    }

    impl Page {
        fn new(shared: SharedPorts) -> Self {
            Self::with_keys(shared, &KeyBindingConfig::default())
        }

        fn with_keys(shared: SharedPorts, config: &KeyBindingConfig) -> Self {
            Self {
                presenter: AccountsPagePresenter::new(shared, accounts_page_keymap(config)),
                router: RecordingRouter::default(),
            }
        }

        fn send(&mut self, event: AccountsPageEvent) -> Vec<RouterCall> {
            self.presenter.handle(event, &mut self.router);
            self.router.take()
        }

        fn command(&mut self, verb: AccountsCommand, argument: &str) -> Vec<RouterCall> {
            self.send(AccountsPageEvent::Command {
                command: "accounts edit".into(),
                verb,
                argument: argument.into(),
            })
        }

        fn view(&self) -> AccountsPageViewModel {
            self.presenter.view_model()
        }

        fn selected_name(&self) -> Option<String> {
            let view = self.view();
            view.groups
                .into_iter()
                .flat_map(|group| group.lines)
                .find(|line| Some(line.id) == view.selected)
                .map(|line| line.name)
        }
    }

    fn open(dialog: AccountsDialog) -> Vec<RouterCall> {
        vec![RouterCall::OpenDialog(dialog)]
    }

    #[test]
    fn the_rows_are_grouped_by_type_in_the_fixed_order_with_their_institution_and_unit() {
        let (_, shared) = stub();
        let view = Page::new(shared).view();

        let labels: Vec<&str> = view
            .groups
            .iter()
            .map(|group| group.label.as_str())
            .collect();
        assert_eq!(
            labels,
            ["Cash", "Bank", "Credit card", "Loan", "Investment"]
        );
        let bank = &view.groups[1].lines;
        assert_eq!(bank[0].name, "ANZ Everyday");
        assert_eq!(bank[0].institution, "ANZ Banking Group");
        assert_eq!(bank[0].unit, "aud");
        assert_eq!(
            view.groups[0].lines[0].institution, "Personal",
            "Cash is at the Personal Institution"
        );
        assert_eq!(view.empty, None);
    }

    #[test]
    fn the_scope_reads_count_net_worth_in_the_base_unit_and_the_units_held_separately() {
        let (ports, shared) = stub();
        let page = Page::new(shared);

        assert_eq!(
            page.view().scope,
            "7 accounts \u{b7} net worth 83,995.87 aud \u{b7} vas, btc held separately"
        );
        assert_eq!(page.view().status, "7 accounts");

        ports.borrow_mut().accounts.accounts.truncate(1);
        let view = page.view();
        assert_eq!(view.status, "1 account");
        assert_eq!(view.scope, "1 account \u{b7} net worth 240.00 aud");
    }

    #[test]
    fn the_net_worth_shows_at_the_base_units_precision() {
        let (ports, shared) = stub();
        for unit in &mut ports.borrow_mut().units.units {
            if unit.id == UnitId::SEED_AUD {
                unit.precision = 0;
            }
        }

        assert_eq!(
            Page::new(shared).view().scope,
            "7 accounts \u{b7} net worth 83,996 aud \u{b7} vas, btc held separately"
        );
    }

    #[test]
    fn a_negative_net_worth_is_signed_and_no_accounts_say_so() {
        let (ports, shared) = stub();
        let page = Page::new(shared);
        ports.borrow_mut().accounts.accounts.retain(|account| {
            matches!(
                account.account_type,
                lib_core::AccountType::Loan | lib_core::AccountType::CreditCard
            )
        });
        assert!(page.view().scope.contains("net worth -"));

        ports.borrow_mut().accounts.accounts.clear();
        let view = page.view();
        assert_eq!(view.scope, "no accounts yet");
        assert_eq!(view.selected, None);
    }

    #[test]
    fn movement_walks_the_rows_in_the_order_they_are_listed_and_the_first_is_selected_to_start() {
        let (_, shared) = stub();
        let mut page = Page::new(shared);

        assert_eq!(page.selected_name().as_deref(), Some("Wallet"));
        page.send(AccountsPageEvent::Move(Step::Next));
        assert_eq!(page.selected_name().as_deref(), Some("ANZ Everyday"));
        page.send(AccountsPageEvent::Move(Step::Last));
        assert_eq!(page.selected_name().as_deref(), Some("Bitcoin"));
        page.send(AccountsPageEvent::Move(Step::First));
        assert_eq!(page.selected_name().as_deref(), Some("Wallet"));
    }

    #[test]
    fn enter_and_a_row_click_open_the_ledger_and_a_click_selects_the_row() {
        let (_, shared) = stub();
        let mut page = Page::new(shared);
        page.send(AccountsPageEvent::Move(Step::Next));

        assert_eq!(
            page.send(AccountsPageEvent::OpenLedger),
            [RouterCall::OpenLedger(AccountId::seed(2))]
        );
        assert_eq!(
            page.send(AccountsPageEvent::Open(AccountId::seed(5))),
            [RouterCall::OpenLedger(AccountId::seed(5))]
        );
        assert_eq!(page.selected_name().as_deref(), Some("Home Loan"));
    }

    #[test]
    fn reveal_selects_the_account_and_ignores_one_that_is_gone() {
        let (_, shared) = stub();
        let mut page = Page::new(shared);

        assert_eq!(page.send(AccountsPageEvent::Reveal(AccountId::seed(6))), []);
        assert_eq!(page.selected_name().as_deref(), Some("Vanguard VAS"));
        page.send(AccountsPageEvent::Reveal(AccountId::new()));
        assert_eq!(page.selected_name().as_deref(), Some("Vanguard VAS"));
    }

    #[test]
    fn new_edit_and_delete_ask_the_router_on_the_selected_account() {
        let (_, shared) = stub();
        let mut page = Page::new(shared);
        page.send(AccountsPageEvent::Move(Step::Next));
        let everyday = AccountId::seed(2);

        assert_eq!(
            page.send(AccountsPageEvent::New),
            open(AccountsDialog::AddAccount {
                name: String::new()
            })
        );
        assert!(page.view().open_dialog.is_some());
        page.send(AccountsPageEvent::DialogClosed(None));
        assert_eq!(page.view().open_dialog, None);
        assert_eq!(
            page.send(AccountsPageEvent::Edit),
            open(AccountsDialog::EditAccount(everyday))
        );
        assert_eq!(
            page.send(AccountsPageEvent::Delete),
            [RouterCall::OpenDelete(everyday)],
            "the Router opens lib-account-deletion's Dialog"
        );
        assert_eq!(
            page.send(AccountsPageEvent::DeleteRow(AccountId::seed(5))),
            [RouterCall::OpenDelete(AccountId::seed(5))]
        );
        assert_eq!(page.selected_name().as_deref(), Some("Home Loan"));
    }

    #[test]
    fn a_save_selects_its_account_and_a_deleted_one_leaves_the_row_that_slid_into_its_place() {
        let (ports, shared) = stub();
        let mut page = Page::new(shared);
        page.send(AccountsPageEvent::DialogClosed(Some(AccountsOutcome {
            id: AccountId::seed(6),
        })));
        assert_eq!(page.selected_name().as_deref(), Some("Vanguard VAS"));

        page.send(AccountsPageEvent::DeleteRow(AccountId::seed(2)));
        ports
            .borrow_mut()
            .accounts
            .delete_account(AccountId::seed(2));
        page.send(AccountsPageEvent::Deleted);
        assert_eq!(
            page.selected_name().as_deref(),
            Some("ANZ Offset"),
            "the next Bank row took its place"
        );

        page.send(AccountsPageEvent::DeleteRow(AccountId::seed(7)));
        ports
            .borrow_mut()
            .accounts
            .delete_account(AccountId::seed(7));
        page.send(AccountsPageEvent::Deleted);
        assert_eq!(page.selected_name().as_deref(), Some("Vanguard VAS"));
    }

    #[test]
    fn a_typed_new_opens_add_with_the_name_and_the_others_resolve_by_name() {
        let (_, shared) = stub();
        let mut page = Page::new(shared);

        assert_eq!(
            page.command(AccountsCommand::New, " Rainy Day "),
            open(AccountsDialog::AddAccount {
                name: "Rainy Day".into()
            })
        );
        page.send(AccountsPageEvent::DialogClosed(None));
        assert_eq!(
            page.command(AccountsCommand::Edit, "home loan"),
            open(AccountsDialog::EditAccount(AccountId::seed(5)))
        );
        assert_eq!(page.selected_name().as_deref(), Some("Home Loan"));
        assert_eq!(
            page.command(AccountsCommand::Delete, "bitc"),
            [RouterCall::OpenDelete(AccountId::seed(7))]
        );
        assert_eq!(
            page.command(AccountsCommand::Edit, ""),
            open(AccountsDialog::EditAccount(AccountId::seed(7))),
            "no name uses the selected row"
        );
    }

    #[test]
    fn a_name_that_fits_nothing_or_several_accounts_is_a_status_message_not_a_guess() {
        let (ports, shared) = stub();
        let mut page = Page::new(shared);
        let status = |text: &str| vec![RouterCall::Notice(Notice::Status(text.into()))];

        assert_eq!(
            page.command(AccountsCommand::Edit, "zzz"),
            status(":accounts edit \u{2014} no account named \"zzz\"")
        );
        assert_eq!(
            page.command(AccountsCommand::Delete, "anz"),
            status(":accounts edit \u{2014} \"anz\" matches ANZ Everyday, ANZ Offset")
        );
        ports.borrow_mut().accounts.accounts.clear();
        assert_eq!(
            page.command(AccountsCommand::Edit, ""),
            status(":accounts edit \u{2014} no accounts")
        );
    }

    #[test]
    fn an_empty_ledger_says_how_to_add_one_with_the_bound_chord_and_the_legend_follows_the_keys() {
        let (ports, shared) = stub();
        ports.borrow_mut().accounts.accounts.clear();
        let mut config = KeyBindingConfig::default();
        config.bindings.insert("new".into(), "a".into());
        config.bindings.insert("select".into(), "o".into());
        let view = Page::with_keys(shared, &config).view();

        assert_eq!(
            view.empty.as_deref(),
            Some("No accounts yet. Press a to add one.")
        );
        let hints: Vec<(&str, &str)> = view
            .hints
            .iter()
            .map(|hint| (hint.chord.as_str(), hint.action.as_str()))
            .collect();
        assert_eq!(
            hints,
            [
                ("j/k", "row"),
                ("o", "open ledger"),
                ("e", "edit"),
                ("d", "delete"),
                ("a", "new")
            ]
        );
    }
}
