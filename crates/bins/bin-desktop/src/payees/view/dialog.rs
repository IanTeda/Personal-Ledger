//! The Payees Dialogs' Views, each open one an Entity in the Dialog host
//! (`shell::view::dialog_entity`). [`PayeesDialog`] holds a `lib-payees` Presenter and the widget
//! state that is gpui's to keep (whether the Default category list is open and its highlight).
//! Every key and click becomes a Presenter event; each View draws the Presenter's view model and
//! nothing else.
//!
//! Confirm and Cancel go to the Presenter, which closes its Dialog through its Router; a confirmed
//! outcome is handed to the page, and the host closes on [`DialogClosed`]. A Toast the Presenter
//! raises goes onto the bin's queue, which the input's `send` drains.

mod delete_payee;
mod payee_form;
mod rules_field;

use gpui::{App, Context, Entity, EventEmitter, IntoElement, Render, Window};
use lib_payees::presenter::{
    DeletePayeeEvent, DeletePayeePresenter, PayeeFormEvent, PayeeFormPresenter,
};
use lib_payees::{PayeesDialog as Which, PayeesOutcome, PayeesRouter};
use lib_shell::DialogCommand;
use lib_toast::{Notice, Notify};
use lib_viper::keymap::{BACK, Keymap, NEXT_FIELD, SELECT};

use crate::form::select::SelectState;
use crate::ledger_store::LedgerStore;
use crate::payees::ports::{self, DesktopPorts};
use crate::shell::view::dialog_entity::{DialogClicks, DialogClosed, HostedDialog};
use crate::shell::view::dialog_host::{DialogKey, DialogOutcome, DialogReadback};
use lib_shell::Hint;

/// The open Payees Dialog's clicks.
type Clicks = DialogClicks<PayeesDialog>;

/// What a confirmed Payees Dialog hands the page that opened it.
pub(crate) type Landed = Box<dyn Fn(PayeesOutcome, &mut App)>;

/// The open Payees Dialog: its Presenter and its widget state.
pub struct PayeesDialog {
    ports: DesktopPorts,
    /// The Ledger read copy a confirm brings in line with what it wrote.
    ledger: Entity<LedgerStore>,
    view: View,
    landed: Landed,
}

enum View {
    /// Add or Edit, with the Default category select's open state.
    Form(PayeeFormPresenter, SelectState),
    Delete(DeletePayeePresenter),
}

impl View {
    /// Which Dialog is open, as the UI tests name it.
    fn kind(&self) -> &'static str {
        match self {
            Self::Form(presenter, _) if presenter.view_model().editing => "EditPayee",
            Self::Form(..) => "AddPayee",
            Self::Delete(_) => "DeletePayee",
        }
    }
}

impl PayeesDialog {
    /// `which`'s Presenter on `ports`, or `None` when the Payee it names is gone. A confirm brings
    /// `ledger` in line and hands its outcome to `landed`.
    pub(crate) fn open(
        which: Which,
        ports: &DesktopPorts,
        ledger: &Entity<LedgerStore>,
        landed: Landed,
    ) -> Option<Self> {
        let shared = ports.clone();
        let view = match which {
            Which::AddPayee => View::Form(PayeeFormPresenter::add(shared), SelectState::default()),
            Which::EditPayee(id) => View::Form(
                PayeeFormPresenter::edit(shared, id)?,
                SelectState::default(),
            ),
            Which::DeletePayee(id) => View::Delete(DeletePayeePresenter::new(shared, id)?),
        };
        Some(Self {
            ports: ports.clone(),
            ledger: ledger.clone(),
            view,
            landed,
        })
    }

    /// Sends the open Presenter its Confirm or its Cancel, returning what its Router was asked.
    fn close_event(&mut self, confirm: bool) -> DialogRouter {
        match &mut self.view {
            View::Form(presenter, _) => {
                let event = if confirm {
                    PayeeFormEvent::Confirm
                } else {
                    PayeeFormEvent::Cancel
                };
                send(presenter, event, PayeeFormPresenter::handle)
            }
            View::Delete(presenter) => {
                let event = if confirm {
                    DeletePayeeEvent::Confirm
                } else {
                    DeletePayeeEvent::Cancel
                };
                send(presenter, event, DeletePayeePresenter::handle)
            }
        }
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
}

/// Where a Payees Dialog's Presenter asks to go: closing, and Notices, which go onto the bin's
/// queue for `Root` to drain once the key or click returns. The page opens Dialogs.
#[derive(Default)]
struct DialogRouter {
    /// `Some` once the Presenter closed its Dialog, with what its confirm did.
    closed: Option<Option<PayeesOutcome>>,
}

impl PayeesRouter for DialogRouter {
    fn open_dialog(&mut self, _dialog: Which) -> bool {
        false
    }

    fn open_transactions(&mut self, _id: lib_core::PayeeId) {}

    fn close_dialog(&mut self, outcome: Option<PayeesOutcome>) {
        self.closed = Some(outcome);
    }
}

impl Notify for DialogRouter {
    fn notify(&mut self, notice: Notice) {
        crate::notices::Queue.notify(notice);
    }
}

/// Sends `event` to `presenter`, returning what its Router was asked.
fn send<P, E>(
    presenter: &mut P,
    event: E,
    handle: fn(&mut P, E, &mut dyn PayeesRouter),
) -> DialogRouter {
    let mut router = DialogRouter::default();
    handle(presenter, event, &mut router);
    router
}

impl EventEmitter<DialogClosed> for PayeesDialog {}

impl HostedDialog for PayeesDialog {
    /// Every key is the Payees Dialog's own: the Presenter holds every field.
    fn key(&mut self, key: DialogKey, _cx: &mut Context<'_, Self>) -> DialogOutcome {
        match &mut self.view {
            View::Form(presenter, select) => payee_form::key(presenter, select, key),
            View::Delete(presenter) => delete_payee::key(presenter, key),
        }
    }

    fn can_confirm(&self) -> bool {
        match &self.view {
            View::Form(presenter, _) => presenter.view_model().frame.can_confirm,
            View::Delete(presenter) => presenter.view_model().frame.can_confirm,
        }
    }

    /// Sends Confirm to the Presenter, which runs its Interactor, with the Ledger read copy in
    /// line either side. A refusal stays on the Presenter's view model and keeps the Dialog open.
    fn confirm(&mut self, cx: &mut Context<'_, Self>) {
        ports::refresh(&self.ports, &self.ledger, cx);
        let router = self.close_event(true);
        ports::refresh(&self.ports, &self.ledger, cx);
        self.closed(router, cx);
    }

    /// A cancelled Payees Dialog has written nothing.
    fn cancel(&mut self, cx: &mut Context<'_, Self>) {
        let router = self.close_event(false);
        self.closed(router, cx);
    }

    fn close_open_select(&mut self) -> bool {
        match &mut self.view {
            View::Form(_, select) => {
                let open = select.is_open();
                select.cancel();
                open
            }
            View::Delete(_) => false,
        }
    }

    /// Cancel and confirm, and on the form, the key that moves field.
    fn hints(&self, keymap: &Keymap<DialogCommand>) -> Option<Vec<Hint>> {
        let key = |command| keymap.chord(command).unwrap_or_default().to_string();
        let mut hints = vec![
            Hint::new(key(BACK), crate::msg::desktop_hint_cancel()),
            Hint::new(key(SELECT), crate::msg::desktop_hint_confirm()),
        ];
        if matches!(self.view, View::Form(..)) {
            hints.push(Hint::new(
                key(NEXT_FIELD),
                crate::msg::desktop_hint_next_field(),
            ));
        }
        Some(hints)
    }

    /// The open Dialog's title, the fields it draws and whether it would confirm, for a test.
    fn readback(&self, _cx: &App) -> DialogReadback {
        let (frame, mut values) = match &self.view {
            View::Form(presenter, _) => {
                let view = presenter.view_model();
                (view.frame.clone(), payee_form::readback(&view))
            }
            View::Delete(presenter) => {
                let view = presenter.view_model();
                (view.frame.clone(), delete_payee::readback(&view))
            }
        };
        values.insert(0, ("title", frame.title));
        values.push(("can_confirm", frame.can_confirm.to_string()));
        DialogReadback {
            kind: self.view.kind(),
            values,
        }
    }
}

impl Render for PayeesDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let actions = Clicks::new(cx);
        match &self.view {
            View::Form(presenter, select) => {
                payee_form::render(&presenter.view_model(), select, &actions, cx)
            }
            View::Delete(presenter) => delete_payee::render(&presenter.view_model(), &actions, cx),
        }
    }
}
