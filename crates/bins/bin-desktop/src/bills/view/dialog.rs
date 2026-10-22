//! The Bills Dialogs' Views, hosted by today's Dialog host. [`BillsDialog`] is the value the host
//! holds while one is open (`OpenDialog::Bills`): a `lib-bills` Presenter and the widget state that
//! is gpui's to keep (which select's list is open and its highlight). Every key and click becomes a
//! Presenter event; each View draws the Presenter's view model and nothing else.
//!
//! The Presenter sits in an `Rc<RefCell<_>>`, not an Entity: the host's `Dialog` methods take no
//! gpui context to update one through, and the host repaints after every key and click.
//!
//! Confirm sends the Presenter its Confirm event ([`ConfirmDialog::confirm`]); a Dialog that closed
//! through its Router hands the outcome to the page. `Esc` and Cancel close through the host, which
//! tells the Presenter nothing, since a cancelled Bills Dialog wrote nothing.

mod plan_form;
mod skip;

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use gpui::{AnyElement, App, Window};
use lib_bills::presenter::{PlanFormEvent, PlanFormPresenter, SkipEvent, SkipPresenter};
use lib_bills::{BillsOutcome, BillsRouter, EntryId, PayRequest};
use lib_core::{BillPlanId, TransactionId};
use lib_toast::ToastKind;

use self::plan_form::PlanSelects;
use crate::bills::ports::{self, DesktopPorts};
use crate::chrome::dialog_host::{
    ConfirmDialog, Dialog, DialogActions, DialogKey, DialogOutcome, DialogReadback, DialogScope,
    Landed,
};
use crate::dialog;

type Shared<P> = Rc<RefCell<P>>;

/// The open Bills Dialog: its Presenter and its widget state.
#[derive(Clone)]
pub struct BillsDialog {
    ports: DesktopPorts,
    view: View,
}

#[derive(Clone)]
enum View {
    /// Boxed: five selects' widget state would swell the Skip variant.
    PlanForm(Shared<PlanFormPresenter>, Box<PlanSelects>),
    Skip(Shared<SkipPresenter>),
}

impl View {
    /// The Presenter's address, which tells two open Dialogs apart.
    fn presenter(&self) -> *const () {
        match self {
            Self::PlanForm(presenter, _) => Rc::as_ptr(presenter).cast(),
            Self::Skip(presenter) => Rc::as_ptr(presenter).cast(),
        }
    }

    /// Which Dialog is open, as the UI tests name it.
    fn kind(&self) -> &'static str {
        match self {
            Self::PlanForm(presenter, _) if presenter.borrow().view_model().active.is_some() => {
                "Edit"
            }
            Self::PlanForm(..) => "Add",
            Self::Skip(_) => "Skip",
        }
    }
}

impl fmt::Debug for BillsDialog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("BillsDialog")
            .field(&self.view.kind())
            .finish()
    }
}

/// Two values are the same Dialog when they hold the same Presenter.
impl PartialEq for BillsDialog {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.view.presenter(), other.view.presenter())
    }
}

impl Eq for BillsDialog {}

impl BillsDialog {
    /// The Add or Edit bill plan Dialog on `presenter`.
    pub fn plan_form(presenter: PlanFormPresenter, ports: &DesktopPorts) -> Self {
        Self {
            ports: ports.clone(),
            view: View::PlanForm(Rc::new(RefCell::new(presenter)), Box::default()),
        }
    }

    /// The Skip this cycle Dialog on `presenter`.
    pub fn skip(presenter: SkipPresenter, ports: &DesktopPorts) -> Self {
        Self {
            ports: ports.clone(),
            view: View::Skip(Rc::new(RefCell::new(presenter))),
        }
    }

    /// A click's change, sent through the host's edit callback so the Dialog repaints.
    fn on_click(actions: &DialogActions, change: impl Fn(&mut Self) + 'static) -> dialog::OnClick {
        let actions = actions.clone();
        let change = Rc::new(change);
        Rc::new(move |_window: &mut Window, cx: &mut App| {
            let change = change.clone();
            actions.edit(cx, move |dialog: &mut Self, _| {
                change(dialog);
                None
            });
        })
    }

    /// As [`Self::on_click`], for a click on row `index` of an open list.
    fn on_option(
        actions: &DialogActions,
        change: impl Fn(&mut Self, usize) + 'static,
    ) -> crate::form::select_field::OnOptionClick {
        let actions = actions.clone();
        let change = Rc::new(change);
        Rc::new(move |index, _window: &mut Window, cx: &mut App| {
            let change = change.clone();
            actions.edit(cx, move |dialog: &mut Self, _| {
                change(dialog, index);
                None
            });
        })
    }
}

/// Where a Bills Dialog's Presenter asks to go. Only closing reaches the Desktop from a Dialog;
/// the page opens Dialogs and reports outcomes.
#[derive(Default)]
struct DialogRouter {
    /// `Some` once the Presenter closed its Dialog, with what its confirm did.
    closed: Option<Option<BillsOutcome>>,
}

impl BillsRouter for DialogRouter {
    fn open_plan_form(&mut self, _plan: Option<BillPlanId>) -> bool {
        false
    }

    fn open_skip(&mut self, _entry: EntryId) -> bool {
        false
    }

    fn open_pay(&mut self, _request: PayRequest) -> bool {
        false
    }

    fn close_dialog(&mut self, outcome: Option<BillsOutcome>) {
        self.closed = Some(outcome);
    }

    fn show_transaction(&mut self, _id: TransactionId) {}

    fn raise_toast(&mut self, _kind: ToastKind, _text: String) {}

    fn set_status(&mut self, _text: Option<String>) {}
}

/// Sends `event` to `presenter`, returning what its Router was asked.
fn send<P, E>(
    presenter: &Shared<P>,
    event: E,
    handle: fn(&mut P, E, &mut dyn BillsRouter),
) -> DialogRouter {
    let mut router = DialogRouter::default();
    handle(&mut presenter.borrow_mut(), event, &mut router);
    router
}

impl Dialog for BillsDialog {
    /// Every key is the Bills Dialog's own: the host's typing and `Tab` would edit a `TextField`
    /// the Presenter doesn't hold.
    fn handle_own_key(&mut self, key: DialogKey) -> Option<DialogOutcome> {
        Some(match &mut self.view {
            View::PlanForm(presenter, selects) => plan_form::key(presenter, selects, key),
            View::Skip(_) => match key {
                DialogKey::Enter | DialogKey::CtrlEnter => DialogOutcome::Confirm,
                _ => DialogOutcome::Handled,
            },
        })
    }

    fn is_valid(&self) -> bool {
        match &self.view {
            View::PlanForm(presenter, _) => presenter.borrow().view_model().frame.can_confirm,
            View::Skip(presenter) => presenter.borrow().view_model().frame.can_confirm,
        }
    }

    fn close_open_select(&mut self) -> bool {
        match &mut self.view {
            View::PlanForm(_, selects) => selects.close(),
            View::Skip(_) => false,
        }
    }

    fn hints(&self) -> Option<Vec<(&'static str, String)>> {
        Some(match &self.view {
            View::PlanForm(..) => crate::settings::hints::payee_dialog_hints(),
            View::Skip(_) => vec![
                ("esc", crate::msg::desktop_hint_cancel()),
                ("enter", crate::msg::desktop_hint_confirm()),
            ],
        })
    }

    fn render(
        &self,
        _scope: &DialogScope,
        actions: &DialogActions,
        cx: &App,
    ) -> Option<AnyElement> {
        Some(match &self.view {
            View::PlanForm(presenter, selects) => {
                plan_form::render(&presenter.borrow().view_model(), selects, actions, cx)
            }
            View::Skip(presenter) => skip::render(&presenter.borrow().view_model(), actions, cx),
        })
    }

    /// The open Dialog's title, the fields it draws and whether it would confirm, for a test.
    fn readback(&self, _scope: &DialogScope, _cx: &App) -> Option<DialogReadback> {
        let (frame, mut values) = match &self.view {
            View::PlanForm(presenter, _) => {
                let view = presenter.borrow().view_model();
                (view.frame.clone(), plan_form::readback(&view))
            }
            View::Skip(presenter) => {
                let view = presenter.borrow().view_model();
                (view.frame.clone(), vec![("body", view.body.clone())])
            }
        };
        values.insert(0, ("title", frame.title));
        values.push(("can_confirm", frame.can_confirm.to_string()));
        Some(DialogReadback {
            kind: self.view.kind(),
            values,
        })
    }
}

impl ConfirmDialog for BillsDialog {
    type Outcome = BillsOutcome;

    /// Sends Confirm to the Presenter, which runs its Interactor. A refusal stays on the
    /// Presenter's view model and keeps the Dialog open.
    fn confirm(&mut self, scope: &DialogScope, cx: &mut App) -> Option<Landed<BillsOutcome>> {
        ports::copy_reads(&self.ports, scope.ledger.read(cx).ledger());
        let router = match &self.view {
            View::PlanForm(presenter, _) => {
                send(presenter, PlanFormEvent::Confirm, PlanFormPresenter::handle)
            }
            View::Skip(presenter) => send(presenter, SkipEvent::Confirm, SkipPresenter::handle),
        };
        ports::sync(&self.ports, &scope.ledger, cx);
        let outcome = router.closed.flatten()?;
        Some(Landed {
            toast: None,
            outcome,
        })
    }
}
