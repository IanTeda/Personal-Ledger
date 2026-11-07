//! The Payees Dialogs' Views, each open one an Entity in the Dialog host
//! (`shell::view::dialog_entity`). [`PayeesDialog`] holds a `lib-payees` Presenter and the widget
//! state that is gpui's to keep (the Default category list's scroll window). Every Dialog command
//! and typed key reaches the Presenter as a `lib-viper` `DialogInput`, which it reads itself, and
//! every click becomes a Presenter event; each View draws the Presenter's view model and nothing
//! else.
//!
//! The Presenter closes its Dialog through the host's [`HostRouter`]; a confirmed outcome is
//! handed to the page, and the host closes on [`DialogClosed`]. A Toast the Presenter raises goes
//! onto the bin's queue, which the input's `send` drains.

mod delete_payee;
mod payee_form;
mod rules_field;

use gpui::{App, Context, EventEmitter, IntoElement, Render, Window};
use lib_payees::presenter::{
    DeletePayeeEvent, DeletePayeePresenter, PayeeFormEvent, PayeeFormPresenter,
};
use lib_payees::{PayeesDialog as Which, PayeesOutcome};
use lib_shell::DialogCommand;
use lib_viper::dialog::{DialogInput, DialogRouter};
use lib_viper::keymap::{BACK, Keymap, NEXT_FIELD, SELECT};

use crate::shell::view::dialog_entity::DialogReadback;
use crate::shell::view::dialog_entity::{DialogClicks, DialogClosed, HostRouter, HostedDialog};
use lib_shell::Hint;

/// The open Payees Dialog's clicks.
type Clicks = DialogClicks<PayeesDialog>;

/// What a confirmed Payees Dialog hands the page that opened it.
pub(crate) type Landed = Box<dyn Fn(PayeesOutcome, &mut App)>;

/// The open Payees Dialog: its Presenter and its widget state.
pub struct PayeesDialog {
    ports: lib_payees::SharedPorts,
    view: View,
    landed: Landed,
}

enum View {
    /// Add or Edit, with the top row of the Default category list's window.
    Form(PayeeFormPresenter, usize),
    Delete(DeletePayeePresenter),
}

impl View {
    /// Which Dialog is open, as the UI tests name it.
    fn kind(&self) -> &'static str {
        match self {
            Self::Form(presenter, _) if presenter.is_editing() => "EditPayee",
            Self::Form(..) => "AddPayee",
            Self::Delete(_) => "DeletePayee",
        }
    }
}

impl PayeesDialog {
    /// `which`'s Presenter on `ports`, or `None` when the Payee it names is gone. A confirm
    /// hands its outcome to `landed`.
    pub(crate) fn open(
        which: Which,
        ports: &lib_payees::SharedPorts,
        landed: Landed,
    ) -> Option<Self> {
        let shared = ports.clone();
        let view = match which {
            Which::AddPayee => View::Form(PayeeFormPresenter::add(shared), 0),
            Which::EditPayee(id) => View::Form(PayeeFormPresenter::edit(shared, id)?, 0),
            Which::DeletePayee(id) => View::Delete(DeletePayeePresenter::new(shared, id)?),
        };
        Some(Self {
            ports: ports.clone(),
            view,
            landed,
        })
    }

    /// Sends the open Presenter its Confirm or its Cancel, returning what its Router was asked.
    fn close_event(&mut self, confirm: bool) -> Router {
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

    /// Sends the open Presenter `input`, returning what its Router was asked.
    fn input_event(&mut self, input: DialogInput) -> Router {
        match &mut self.view {
            View::Form(presenter, _) => send(
                presenter,
                PayeeFormEvent::Input(input),
                PayeeFormPresenter::handle,
            ),
            View::Delete(presenter) => send(
                presenter,
                DeletePayeeEvent::Input(input),
                DeletePayeePresenter::handle,
            ),
        }
    }

    /// Once the Presenter has closed the Dialog, hands its outcome to the page and tells the host.
    fn closed(&self, router: Router, cx: &mut Context<'_, Self>) {
        let Some(outcome) = router.closed() else {
            return;
        };
        if let Some(outcome) = outcome {
            (self.landed)(outcome, cx);
        }
        cx.emit(DialogClosed);
    }
}

/// The host's Router, as a Payees Dialog's Presenter closes through it.
type Router = HostRouter<PayeesOutcome>;

/// Sends `event` to `presenter`, returning what its Router was asked.
fn send<P, E>(
    presenter: &mut P,
    event: E,
    handle: fn(&mut P, E, &mut dyn DialogRouter<PayeesOutcome>),
) -> Router {
    let mut router = Router::default();
    handle(presenter, event, &mut router);
    router
}

impl EventEmitter<DialogClosed> for PayeesDialog {}

impl HostedDialog for PayeesDialog {
    /// Every input is the Presenter's to read.
    fn input(&mut self, input: DialogInput, cx: &mut Context<'_, Self>) {
        let router = self.input_event(input);
        self.closed(router, cx);
    }

    fn can_confirm(&self) -> bool {
        match &self.view {
            View::Form(presenter, _) => presenter.can_confirm(),
            View::Delete(presenter) => presenter.can_confirm(),
        }
    }

    /// Sends Confirm to the Presenter, which runs its Interactor. A refusal stays on the
    /// Presenter's view model and keeps the Dialog open.
    fn confirm(&mut self, cx: &mut Context<'_, Self>) {
        let router = self.close_event(true);
        self.closed(router, cx);
    }

    /// A cancelled Payees Dialog has written nothing.
    fn cancel(&mut self, cx: &mut Context<'_, Self>) {
        let router = self.close_event(false);
        self.closed(router, cx);
    }

    fn close_open_select(&mut self) -> bool {
        match &mut self.view {
            View::Form(presenter, _) => {
                let open = presenter.is_category_list_open();
                if open {
                    send(
                        presenter,
                        PayeeFormEvent::CloseCategories,
                        PayeeFormPresenter::handle,
                    );
                }
                open
            }
            View::Delete(_) => false,
        }
    }

    /// Cancel and confirm, and on the form, the key that moves field.
    fn hints(&self, keymap: &Keymap<DialogCommand>) -> Option<Vec<Hint>> {
        let key = |command| keymap.chord(command).unwrap_or_default().to_string();
        let mut hints = vec![
            Hint::new(key(BACK), lib_locale::msg::dialog_hint_cancel()),
            Hint::new(key(SELECT), lib_locale::msg::dialog_hint_confirm()),
        ];
        if matches!(self.view, View::Form(..)) {
            hints.push(Hint::new(
                key(NEXT_FIELD),
                lib_locale::msg::dialog_hint_next_field(),
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
        match &mut self.view {
            View::Form(presenter, scroll) => {
                payee_form::render(&presenter.view_model(), scroll, &actions, cx)
            }
            View::Delete(presenter) => delete_payee::render(&presenter.view_model(), &actions, cx),
        }
    }
}
