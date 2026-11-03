//! The Add and Edit account Dialog's View, an Entity in the Dialog host
//! (`shell::view::dialog_entity`). [`AccountsDialog`] holds the `lib-accounts` form Presenter and
//! the widget state that is gpui's to keep (each select's list scroll window). Every Dialog command
//! and typed key reaches the Presenter as a `lib-viper` `DialogInput`, which it reads itself, and
//! every click becomes a Presenter event; the View draws the Presenter's view model and nothing
//! else.
//!
//! Confirm and Cancel go to the Presenter, which closes its Dialog through the host's
//! [`HostRouter`]; a confirmed
//! outcome is handed to the page, and the host closes on [`DialogClosed`]. Delete account is
//! `lib-account-deletion`'s Dialog, opened through the page's Router.

mod account_form;

use gpui::{App, Context, Entity, EventEmitter, IntoElement, Render, Window};
use lib_accounts::presenter::{AccountFormEvent, AccountFormPresenter};
use lib_accounts::{AccountsDialog as Which, AccountsOutcome};
use lib_shell::DialogCommand;
use lib_viper::dialog::DialogInput;
use lib_viper::keymap::{BACK, Keymap, NEXT_FIELD, SELECT};

use crate::accounts::ports::{self, DesktopPorts};
use crate::ledger_store::LedgerStore;
use crate::shell::view::dialog_entity::{DialogClicks, DialogClosed, HostRouter, HostedDialog};
use crate::shell::view::dialog_host::DialogReadback;
use lib_shell::Hint;

/// The open Dialog's clicks.
type Clicks = DialogClicks<AccountsDialog>;

/// What a confirmed Dialog hands the page that opened it.
pub(crate) type Landed = Box<dyn Fn(AccountsOutcome, &mut App)>;

/// The open Add or Edit account Dialog: its Presenter and its widget state.
pub struct AccountsDialog {
    ports: DesktopPorts,
    /// The Ledger read copy a confirm brings in line with what it wrote.
    ledger: Entity<LedgerStore>,
    presenter: AccountFormPresenter,
    /// The top row of each select's list window (Institution, Type, Unit), which the View keeps
    /// between draws.
    scrolls: [usize; 3],
    landed: Landed,
}

impl AccountsDialog {
    /// `which`'s Presenter on `ports`, or `None` when the Account it names is gone. A confirm
    /// brings `ledger` in line and hands its outcome to `landed`. An Account added opens `today`.
    pub(crate) fn open(
        which: Which,
        ports: &DesktopPorts,
        ledger: &Entity<LedgerStore>,
        today: chrono::NaiveDate,
        landed: Landed,
    ) -> Option<Self> {
        let shared = ports.clone();
        let presenter = match which {
            Which::AddAccount { name } => AccountFormPresenter::add(shared, &name, today),
            Which::EditAccount(id) => AccountFormPresenter::edit(shared, id, today)?,
        };
        Some(Self {
            ports: ports.clone(),
            ledger: ledger.clone(),
            presenter,
            scrolls: [0; 3],
            landed,
        })
    }

    /// Sends the Presenter `event`, returning what its Router was asked.
    fn send(&mut self, event: AccountFormEvent) -> Router {
        let mut router = Router::default();
        self.presenter.handle(event, &mut router);
        router
    }

    /// Hands a confirmed outcome to the page and tells the host, once the Presenter has closed
    /// the Dialog.
    fn closed(&self, router: Router, cx: &mut Context<'_, Self>) {
        let Some(outcome) = router.closed() else {
            return;
        };
        if let Some(outcome) = outcome {
            (self.landed)(outcome, cx);
        }
        cx.emit(DialogClosed);
    }

    /// The Dialog's kind, as the UI tests name it.
    fn kind(&self) -> &'static str {
        if self.presenter.view_model().editing {
            "EditAccount"
        } else {
            "AddAccount"
        }
    }
}

/// The host's Router, as the Dialog's Presenter closes through it.
type Router = HostRouter<AccountsOutcome>;

impl EventEmitter<DialogClosed> for AccountsDialog {}

impl HostedDialog for AccountsDialog {
    /// Every input is the Presenter's to read. One that may confirm (`select`, `submit`) runs with
    /// the Ledger read copy in line before it and after it.
    fn input(&mut self, input: DialogInput, cx: &mut Context<'_, Self>) -> bool {
        let may_confirm = matches!(input, DialogInput::Select | DialogInput::Submit);
        if may_confirm {
            ports::refresh(&self.ports, &self.ledger, cx);
        }
        let router = self.send(AccountFormEvent::Input(input));
        if may_confirm {
            ports::refresh(&self.ports, &self.ledger, cx);
        }
        self.closed(router, cx);
        true
    }

    fn can_confirm(&self) -> bool {
        self.presenter.view_model().frame.can_confirm
    }

    /// Sends Confirm to the Presenter, which runs its Interactor, with the Ledger read copy in
    /// line either side. A refusal stays on the Presenter's view model and keeps the Dialog open.
    fn confirm(&mut self, cx: &mut Context<'_, Self>) {
        ports::refresh(&self.ports, &self.ledger, cx);
        let router = self.send(AccountFormEvent::Confirm);
        ports::refresh(&self.ports, &self.ledger, cx);
        self.closed(router, cx);
    }

    /// A cancelled Dialog has written nothing.
    fn cancel(&mut self, cx: &mut Context<'_, Self>) {
        let router = self.send(AccountFormEvent::Cancel);
        self.closed(router, cx);
    }

    fn close_open_select(&mut self) -> bool {
        let open = self.presenter.view_model().list.is_open();
        if open {
            self.send(AccountFormEvent::CloseList);
        }
        open
    }

    /// Cancel and confirm, and the key that moves field.
    fn hints(&self, keymap: &Keymap<DialogCommand>) -> Option<Vec<Hint>> {
        let key = |command| keymap.chord(command).unwrap_or_default().to_string();
        Some(vec![
            Hint::new(key(BACK), crate::msg::desktop_hint_cancel()),
            Hint::new(key(SELECT), crate::msg::desktop_hint_confirm()),
            Hint::new(key(NEXT_FIELD), crate::msg::desktop_hint_next_field()),
        ])
    }

    /// The focused field, the drafts, the selects, whether a list is open and whether it would
    /// confirm; Edit also reads back the `transactions` and `budgets` counts it quotes.
    fn readback(&self, _cx: &App) -> DialogReadback {
        let view = self.presenter.view_model();
        let mut values = account_form::readback(&view);
        values.insert(0, ("title", view.frame.title.clone()));
        values.push(("can_confirm", view.frame.can_confirm.to_string()));
        DialogReadback {
            kind: self.kind(),
            values,
        }
    }
}

impl Render for AccountsDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let actions = Clicks::new(cx);
        account_form::render(
            &self.presenter.view_model(),
            &mut self.scrolls,
            &actions,
            cx,
        )
    }
}
