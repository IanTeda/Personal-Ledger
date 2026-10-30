//! The Accounts Settings page's View: the `lib-accounts` page Presenter in a gpui Entity. It looks
//! up each key the Shell passes on, as chord text, in the Accounts keymap, turns clicks into
//! events, draws the view model (`render.rs`), and keeps the Ledger read copy in line after every
//! event ([`ports::refresh`]). Each key and click is one input (`crate::input::send`), which
//! commits what it wrote. The Settings View hosts it as any other page.

mod render;

use gpui::{AnyElement, App, Context, Entity, EventEmitter, IntoElement as _, Subscription};
use lib_accounts::AccountsOutcome;
use lib_accounts::presenter::{
    AccountsCommand, AccountsPageEvent, AccountsPagePresenter, AccountsPageViewModel,
};
use lib_ledger::Domain;
use lib_viper::keymap::Keymap;

use crate::account_deletion::DeletionStores;
use crate::accounts::SettingsAccountsSnapshot;
use crate::accounts::ports::{self, DesktopPorts};
use crate::accounts::router::PageRouter;
use crate::settings::snapshot::PageSnapshot;
use crate::{
    ledger_store::{LedgerStore, notify_on},
    navigation::command::AccountsVerb,
    navigation::movement::Movement,
    settings::{SettingsPage, page::PageRowClicked, pages::Page},
    shell::view::statusline::Hint,
    view::event::{Reveal, ViewEvent},
};

impl EventEmitter<ViewEvent> for AccountsPage {}

/// The page has no rows, so it never emits a row click.
impl EventEmitter<PageRowClicked> for AccountsPage {}

pub struct AccountsPage {
    presenter: AccountsPagePresenter,
    ports: DesktopPorts,
    deletion: DeletionStores,
    ledger: Entity<LedgerStore>,
    clock: lib_core::Clock,
    keymap: Keymap<AccountsPageEvent>,
    /// Whether the page, not Settings' index, has keyboard focus, told by the Settings View as it
    /// draws the page.
    page_focused: bool,
    /// Repaints the page when the Accounts, or the Units and Institutions it names, change.
    _accounts_changed: Subscription,
    _units_changed: Subscription,
    _institutions_changed: Subscription,
}

impl AccountsPage {
    pub fn new(
        ports: DesktopPorts,
        deletion: DeletionStores,
        ledger: Entity<LedgerStore>,
        clock: lib_core::Clock,
        keymap: Keymap<AccountsPageEvent>,
        cx: &mut Context<'_, Self>,
    ) -> Self {
        ports::copy_from_ledger(&ports, ledger.read(cx).ledger());
        Self {
            presenter: AccountsPagePresenter::new(ports.clone(), keymap.clone()),
            ports,
            deletion,
            _accounts_changed: notify_on(&ledger, Domain::Accounts, cx),
            _units_changed: notify_on(&ledger, Domain::Units, cx),
            _institutions_changed: notify_on(&ledger, Domain::Institutions, cx),
            ledger,
            clock,
            keymap,
            page_focused: false,
        }
    }

    /// A key or click on the page: `event` as one input, committed once it is handled.
    pub(in crate::accounts) fn send(
        &mut self,
        event: AccountsPageEvent,
        cx: &mut Context<'_, Self>,
    ) {
        crate::input::send(cx, |cx| self.handle(event, cx));
    }

    /// Hands `event` to the Presenter with the Desktop's Router, then brings the Ledger read copy
    /// in line with whatever it wrote. Inside an input already, as [`Self::send`] or a Dialog's
    /// confirm.
    fn handle(&mut self, event: AccountsPageEvent, cx: &mut Context<'_, Self>) {
        ports::refresh(&self.ports, &self.ledger, cx);
        let mut router = PageRouter {
            ports: self.ports.clone(),
            ledger: self.ledger.clone(),
            deletion: self.deletion.clone(),
            clock: self.clock.clone(),
            cx,
        };
        self.presenter.handle(event, &mut router);
        ports::refresh(&self.ports, &self.ledger, cx);
        cx.notify();
    }

    /// A confirmed Dialog's outcome, handed back by the Dialog host inside the confirm's input.
    pub(in crate::accounts) fn dialog_landed(
        &mut self,
        outcome: AccountsOutcome,
        cx: &mut Context<'_, Self>,
    ) {
        self.handle(AccountsPageEvent::DialogClosed(Some(outcome)), cx);
    }

    /// The Delete Dialog landed, inside the confirm's input.
    pub(in crate::accounts) fn deleted(&mut self, cx: &mut Context<'_, Self>) {
        self.handle(AccountsPageEvent::Deleted, cx);
    }

    /// The page as drawn, with the Units, Institutions and Budgets as the Ledger has them now.
    pub fn view_model(&self, cx: &App) -> AccountsPageViewModel {
        ports::copy_from_ledger(&self.ports, self.ledger.read(cx).ledger());
        self.presenter.view_model()
    }

    /// `accounts new|edit|delete [<account name>]`, through [`crate::accounts::router::run_command`]:
    /// the same Dialog the page's `n`/`e`/`d` and buttons open. A name that fits nothing or several
    /// Accounts puts a status-line message naming the problem.
    pub(in crate::accounts) fn run_command(
        &mut self,
        command_name: &str,
        verb: AccountsVerb,
        argument: &str,
        cx: &mut Context<'_, Self>,
    ) {
        let verb = match verb {
            AccountsVerb::New => AccountsCommand::New,
            AccountsVerb::Edit => AccountsCommand::Edit,
            AccountsVerb::Delete => AccountsCommand::Delete,
        };
        self.send(
            AccountsPageEvent::Command {
                command: command_name.to_string(),
                verb,
                argument: argument.to_string(),
            },
            cx,
        );
    }
}

impl PageSnapshot for SettingsAccountsSnapshot {
    type Page = AccountsPage;

    fn read(page: &AccountsPage, cx: &App) -> Self {
        let view = page.view_model(cx);
        let lines = || view.groups.iter().flat_map(|group| &group.lines);
        SettingsAccountsSnapshot {
            selected_account: lines()
                .find(|line| Some(line.id) == view.selected)
                .map(|line| line.name.clone()),
            account_names: lines().map(|line| line.name.clone()).collect(),
        }
    }
}

impl Page for AccountsPage {
    const PAGE: SettingsPage = SettingsPage::Accounts;

    /// Every chord the Accounts keymap binds, the arrows, `g g` and `enter` among them. The Shell
    /// has already taken its own keys.
    fn module_key(&mut self, chord: &str, cx: &mut Context<'_, Self>) -> bool {
        let Some(event) = self.keymap.event(chord) else {
            return false;
        };
        self.send(event, cx);
        true
    }

    /// A movement the keymap doesn't bind does nothing on the page, and Settings leaves the page
    /// where it is.
    fn apply_movement(&mut self, _movement: Movement, _cx: &mut Context<'_, Self>) -> bool {
        true
    }

    /// Selects an Account another View handed off.
    fn reveal(&mut self, reveal: Reveal, cx: &mut Context<'_, Self>) {
        if let Reveal::Account(id) = reveal {
            self.send(AccountsPageEvent::Reveal(id), cx);
        }
    }

    /// The Presenter's legend, rebound keys included, which the `?` cheat sheet reads.
    fn hints(&self, cx: &App) -> Vec<Hint> {
        legend(self.view_model(cx).hints)
    }

    /// The page's account count.
    fn status_note(&self, cx: &App) -> String {
        self.view_model(cx).status
    }

    fn set_page_focused(&mut self, focused: bool, _cx: &mut Context<'_, Self>) {
        self.page_focused = focused;
    }

    fn heading_meta(&self, cx: &App) -> AnyElement {
        self.view_model(cx).scope.into_any_element()
    }
}

/// The Presenter's hints as the status line and the `?` cheat sheet take them.
fn legend(hints: Vec<lib_accounts::presenter::Hint>) -> Vec<Hint> {
    hints
        .into_iter()
        .map(|hint| (hint.chord, hint.action))
        .collect()
}
