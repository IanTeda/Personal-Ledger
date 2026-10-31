//! The Add and Edit account Dialog's View, an Entity in the Dialog host
//! (`shell::view::dialog_entity`). [`AccountsDialog`] holds the `lib-accounts` form Presenter and
//! the widget state that is gpui's to keep (whether each select's list is open and its highlight).
//! Every key and click becomes a Presenter event; the View draws the Presenter's view model and
//! nothing else.
//!
//! Confirm and Cancel go to the Presenter, which closes its Dialog through its Router; a confirmed
//! outcome is handed to the page, and the host closes on [`DialogClosed`]. Delete account is
//! `lib-account-deletion`'s Dialog, opened through the page's Router.

mod account_form;

use gpui::{App, Context, Entity, EventEmitter, IntoElement, Render, Window};
use lib_accounts::presenter::{AccountField, AccountFormEvent, AccountFormPresenter};
use lib_accounts::{AccountsDialog as Which, AccountsOutcome, AccountsRouter};
use lib_core::AccountId;
use lib_shell::DialogCommand;
use lib_toast::{Notice, Notify};
use lib_viper::keymap::{BACK, Keymap, NEXT_FIELD, SELECT};

use crate::accounts::ports::{self, DesktopPorts};
use crate::form::select::SelectState;
use crate::ledger_store::LedgerStore;
use crate::shell::view::dialog_entity::{DialogClicks, DialogClosed, HostedDialog};
use crate::shell::view::dialog_host::{DialogKey, DialogOutcome, DialogReadback};
use lib_shell::Hint;

/// The open Dialog's clicks.
type Clicks = DialogClicks<AccountsDialog>;

/// What a confirmed Dialog hands the page that opened it.
pub(crate) type Landed = Box<dyn Fn(AccountsOutcome, &mut App)>;

/// The three selects' open state and highlight.
#[derive(Default)]
struct Selects {
    institution: SelectState,
    account_type: SelectState,
    unit: SelectState,
}

impl Selects {
    fn any_open(&self) -> bool {
        self.institution.is_open() || self.account_type.is_open() || self.unit.is_open()
    }

    /// Closes every list, discarding its highlight. Returns whether one was open.
    fn close_all(&mut self) -> bool {
        let was_open = self.any_open();
        self.institution.cancel();
        self.account_type.cancel();
        self.unit.cancel();
        was_open
    }

    /// Closes every list but `field`'s.
    fn close_except(&mut self, field: AccountField) {
        if field != AccountField::Institution {
            self.institution.cancel();
        }
        if field != AccountField::Type {
            self.account_type.cancel();
        }
        if field != AccountField::Unit {
            self.unit.cancel();
        }
    }
}

/// The open Add or Edit account Dialog: its Presenter and its widget state.
pub struct AccountsDialog {
    ports: DesktopPorts,
    /// The Ledger read copy a confirm brings in line with what it wrote.
    ledger: Entity<LedgerStore>,
    presenter: AccountFormPresenter,
    selects: Selects,
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
            selects: Selects::default(),
            landed,
        })
    }

    /// Sends the Presenter `event`, returning what its Router was asked.
    fn send(&mut self, event: AccountFormEvent) -> DialogRouter {
        let mut router = DialogRouter::default();
        self.presenter.handle(event, &mut router);
        router
    }

    /// Hands a confirmed outcome to the page and tells the host, once the Presenter has closed
    /// the Dialog.
    fn closed(&self, router: DialogRouter, cx: &mut Context<'_, Self>) {
        let Some(outcome) = router.closed else {
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

/// Where the Dialog's Presenter asks to go: closing, and Notices, which go onto the bin's queue
/// for `Root` to drain once the key or click returns. The page opens Dialogs.
#[derive(Default)]
struct DialogRouter {
    /// `Some` once the Presenter closed its Dialog, with what its confirm did.
    closed: Option<Option<AccountsOutcome>>,
}

impl AccountsRouter for DialogRouter {
    fn open_dialog(&mut self, _dialog: Which) -> bool {
        false
    }

    fn open_delete(&mut self, _id: AccountId) -> bool {
        false
    }

    fn open_ledger(&mut self, _id: AccountId) {}

    fn close_dialog(&mut self, outcome: Option<AccountsOutcome>) {
        self.closed = Some(outcome);
    }
}

impl Notify for DialogRouter {
    fn notify(&mut self, notice: Notice) {
        crate::notices::Queue.notify(notice);
    }
}

impl EventEmitter<DialogClosed> for AccountsDialog {}

impl HostedDialog for AccountsDialog {
    /// Every key is the Dialog's own: the Presenter holds every field.
    fn key(&mut self, key: DialogKey, _cx: &mut Context<'_, Self>) -> DialogOutcome {
        account_form::key(self, key)
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
        self.selects.close_all()
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
        let mut values = account_form::readback(&view, self.selects.any_open());
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
        account_form::render(&self.presenter.view_model(), &self.selects, &actions, cx)
    }
}
