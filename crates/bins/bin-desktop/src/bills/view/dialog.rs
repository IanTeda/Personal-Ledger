//! The Bills Dialogs' Views, each open one an Entity in the Dialog host (`shell::view::dialog_entity`).
//! [`BillsDialog`] holds a `lib-bills` Presenter and the widget state that is gpui's to keep (which
//! select's list is open and its highlight). Every key and click becomes a Presenter event; each
//! View draws the Presenter's view model and nothing else.
//!
//! Confirm and Cancel go to the Presenter, which closes its Dialog through its Router; a confirmed
//! outcome is handed to the page, and the host closes on [`DialogClosed`].

mod plan_form;
mod skip;

use gpui::{App, Context, Entity, EventEmitter, IntoElement, Render, Window};
use lib_bills::presenter::{PlanFormEvent, PlanFormPresenter, SkipEvent, SkipPresenter};
use lib_bills::{BillsOutcome, BillsRouter, EntryId, PayRequest};
use lib_core::{BillPlanId, TransactionId};
use lib_shell::DialogCommand;
use lib_toast::{Notice, Notify};
use lib_viper::keymap::{BACK, Keymap, NEXT_FIELD, SELECT};

use self::plan_form::PlanSelects;
use crate::bills::ports::{self, DesktopPorts};
use crate::dialog;
use crate::ledger_store::LedgerStore;
use crate::shell::view::dialog_entity::{DialogClicks, DialogClosed, HostedDialog};
use crate::shell::view::dialog_host::{DialogKey, DialogOutcome, DialogReadback};
use crate::shell::view::statusline::Hint;

/// The open Bills Dialog's clicks.
type Clicks = DialogClicks<BillsDialog>;

/// What a confirmed Bills Dialog hands the page that opened it.
pub(crate) type Landed = Box<dyn Fn(BillsOutcome, &mut App)>;

/// The open Bills Dialog: its Presenter and its widget state.
pub struct BillsDialog {
    ports: DesktopPorts,
    /// The Ledger read copy a confirm reads from and brings in line with what it wrote.
    ledger: Entity<LedgerStore>,
    view: View,
    landed: Landed,
}

enum View {
    /// Boxed: the form and its five selects' widget state would swell the Skip variant.
    PlanForm(Box<PlanFormPresenter>, Box<PlanSelects>),
    Skip(SkipPresenter),
}

impl View {
    /// Which Dialog is open, as the UI tests name it.
    fn kind(&self) -> &'static str {
        match self {
            Self::PlanForm(presenter, _) if presenter.view_model().active.is_some() => "Edit",
            Self::PlanForm(..) => "Add",
            Self::Skip(_) => "Skip",
        }
    }
}

impl BillsDialog {
    /// The Add or Edit bill plan Dialog on `presenter`. A confirm reads and refreshes `ledger` and
    /// hands its outcome to `landed`.
    pub(crate) fn plan_form(
        presenter: PlanFormPresenter,
        ports: &DesktopPorts,
        ledger: &Entity<LedgerStore>,
        landed: Landed,
    ) -> Self {
        Self::new(
            View::PlanForm(Box::new(presenter), Box::default()),
            ports,
            ledger,
            landed,
        )
    }

    /// The Skip this cycle Dialog on `presenter`.
    pub(crate) fn skip(
        presenter: SkipPresenter,
        ports: &DesktopPorts,
        ledger: &Entity<LedgerStore>,
        landed: Landed,
    ) -> Self {
        Self::new(View::Skip(presenter), ports, ledger, landed)
    }

    fn new(view: View, ports: &DesktopPorts, ledger: &Entity<LedgerStore>, landed: Landed) -> Self {
        Self {
            ports: ports.clone(),
            ledger: ledger.clone(),
            view,
            landed,
        }
    }

    /// A click's change to the Dialog, as one input.
    fn on_click(actions: &Clicks, change: impl Fn(&mut Self) + 'static) -> dialog::OnClick {
        actions.click(change)
    }

    /// As [`Self::on_click`], for a click on row `index` of an open list.
    fn on_option(
        actions: &Clicks,
        change: impl Fn(&mut Self, usize) + 'static,
    ) -> crate::form::select_field::OnOptionClick {
        actions.option(change)
    }

    /// Sends the open Presenter its Confirm or its Cancel, returning what its Router was asked.
    fn close_event(&mut self, confirm: bool) -> DialogRouter {
        match &mut self.view {
            View::PlanForm(presenter, _) => {
                let event = if confirm {
                    PlanFormEvent::Confirm
                } else {
                    PlanFormEvent::Cancel
                };
                send(&mut **presenter, event, PlanFormPresenter::handle)
            }
            View::Skip(presenter) => {
                let event = if confirm {
                    SkipEvent::Confirm
                } else {
                    SkipEvent::Cancel
                };
                send(presenter, event, SkipPresenter::handle)
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

/// Where a Bills Dialog's Presenter asks to go: closing, and Notices, which go onto the bin's
/// queue for `Root` to drain once the key or click returns. The page opens Dialogs and reports
/// outcomes.
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
    handle: fn(&mut P, E, &mut dyn BillsRouter),
) -> DialogRouter {
    let mut router = DialogRouter::default();
    handle(presenter, event, &mut router);
    router
}

impl EventEmitter<DialogClosed> for BillsDialog {}

impl HostedDialog for BillsDialog {
    /// Every key is the Bills Dialog's own: the Presenter holds every field.
    fn key(&mut self, key: DialogKey, _cx: &mut Context<'_, Self>) -> DialogOutcome {
        match &mut self.view {
            View::PlanForm(presenter, selects) => plan_form::key(presenter, selects, key),
            View::Skip(_) => match key {
                DialogKey::Enter => DialogOutcome::Confirm,
                _ => DialogOutcome::Handled,
            },
        }
    }

    fn can_confirm(&self) -> bool {
        match &self.view {
            View::PlanForm(presenter, _) => presenter.view_model().frame.can_confirm,
            View::Skip(presenter) => presenter.view_model().frame.can_confirm,
        }
    }

    /// Sends Confirm to the Presenter, which runs its Interactor. A refusal stays on the
    /// Presenter's view model and keeps the Dialog open.
    fn confirm(&mut self, cx: &mut Context<'_, Self>) {
        ports::copy_reads(&self.ports, self.ledger.read(cx).ledger());
        let router = self.close_event(true);
        ports::refresh(&self.ports, &self.ledger, cx);
        self.closed(router, cx);
    }

    /// A cancelled Bills Dialog has written nothing.
    fn cancel(&mut self, cx: &mut Context<'_, Self>) {
        let router = self.close_event(false);
        self.closed(router, cx);
    }

    fn close_open_select(&mut self) -> bool {
        match &mut self.view {
            View::PlanForm(_, selects) => selects.close(),
            View::Skip(_) => false,
        }
    }

    fn hints(&self, keymap: &Keymap<DialogCommand>) -> Option<Vec<Hint>> {
        let key = |command| keymap.chord(command).unwrap_or_default().to_string();
        let mut hints = vec![
            (key(BACK), crate::msg::desktop_hint_cancel()),
            (key(SELECT), crate::msg::desktop_hint_confirm()),
        ];
        if matches!(self.view, View::PlanForm(..)) {
            hints.push((key(NEXT_FIELD), crate::msg::desktop_hint_next_field()));
        }
        Some(hints)
    }

    /// The open Dialog's title, the fields it draws and whether it would confirm, for a test.
    fn readback(&self, _cx: &App) -> DialogReadback {
        let (frame, mut values) = match &self.view {
            View::PlanForm(presenter, selects) => {
                let view = presenter.view_model();
                (view.frame.clone(), plan_form::readback(&view, selects))
            }
            View::Skip(presenter) => {
                let view = presenter.view_model();
                (view.frame.clone(), vec![("body", view.body.clone())])
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

impl Render for BillsDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let actions = Clicks::new(cx);
        match &self.view {
            View::PlanForm(presenter, selects) => {
                plan_form::render(&presenter.view_model(), selects, &actions, cx)
            }
            View::Skip(presenter) => skip::render(&presenter.view_model(), &actions, cx),
        }
    }
}
