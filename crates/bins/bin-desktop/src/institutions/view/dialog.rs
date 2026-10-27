//! The Institutions Add Dialog's View, an Entity in the Dialog host (`shell::view::dialog_entity`).
//! [`InstitutionsDialog`] holds the `lib-institutions` form Presenter. Every key and click becomes
//! a Presenter event; the View draws the Presenter's view model and nothing else.
//!
//! Confirm and Cancel go to the Presenter, which closes its Dialog through its Router; a confirmed
//! outcome is handed to the page, and the host closes on [`DialogClosed`]. The Toast the Presenter
//! raises goes onto the bin's queue, which the input's `send` drains.

mod institution_form;

use gpui::{App, Context, Entity, EventEmitter, IntoElement, Render, Window};
use lib_institutions::presenter::{InstitutionFormEvent, InstitutionFormPresenter};
use lib_institutions::{InstitutionsDialog as Which, InstitutionsOutcome, InstitutionsRouter};
use lib_shell::DialogCommand;
use lib_toast::{Notice, Notify};
use lib_viper::keymap::{BACK, Keymap, SELECT};

use crate::institutions::ports::{self, DesktopPorts};
use crate::ledger_store::LedgerStore;
use crate::shell::view::dialog_entity::{DialogClicks, DialogClosed, HostedDialog};
use crate::shell::view::dialog_host::{DialogKey, DialogOutcome, DialogReadback};
use crate::shell::view::statusline::Hint;

/// The open Dialog's clicks.
type Clicks = DialogClicks<InstitutionsDialog>;

/// What a confirmed Dialog hands the page that opened it.
pub(crate) type Landed = Box<dyn Fn(InstitutionsOutcome, &mut App)>;

/// The open Institutions Dialog: its Presenter.
pub struct InstitutionsDialog {
    ports: DesktopPorts,
    /// The Ledger read copy a confirm brings in line with what it wrote.
    ledger: Entity<LedgerStore>,
    presenter: InstitutionFormPresenter,
    landed: Landed,
}

impl InstitutionsDialog {
    /// `which`'s Presenter on `ports`. A confirm brings `ledger` in line and hands its outcome to
    /// `landed`.
    pub(crate) fn open(
        which: Which,
        ports: &DesktopPorts,
        ledger: &Entity<LedgerStore>,
        landed: Landed,
    ) -> Self {
        let presenter = match which {
            Which::AddInstitution => InstitutionFormPresenter::add(ports.clone()),
        };
        Self {
            ports: ports.clone(),
            ledger: ledger.clone(),
            presenter,
            landed,
        }
    }

    /// Sends the Presenter `event`, returning what its Router was asked.
    fn send(&mut self, event: InstitutionFormEvent) -> DialogRouter {
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
}

/// Where the Dialog's Presenter asks to go: closing, and Notices, which go onto the bin's queue
/// for `Root` to drain once the key or click returns. The page opens Dialogs.
#[derive(Default)]
struct DialogRouter {
    /// `Some` once the Presenter closed its Dialog, with what its confirm did.
    closed: Option<Option<InstitutionsOutcome>>,
}

impl InstitutionsRouter for DialogRouter {
    fn open_dialog(&mut self, _dialog: Which) -> bool {
        false
    }

    fn close_dialog(&mut self, outcome: Option<InstitutionsOutcome>) {
        self.closed = Some(outcome);
    }
}

impl Notify for DialogRouter {
    fn notify(&mut self, notice: Notice) {
        crate::notices::Queue.notify(notice);
    }
}

impl EventEmitter<DialogClosed> for InstitutionsDialog {}

impl HostedDialog for InstitutionsDialog {
    /// Every key is the Dialog's own: the Presenter holds every field.
    fn key(&mut self, key: DialogKey, _cx: &mut Context<'_, Self>) -> DialogOutcome {
        institution_form::key(self, key)
    }

    fn can_confirm(&self) -> bool {
        self.presenter.view_model().frame.can_confirm
    }

    /// Sends Confirm to the Presenter, which runs its Interactor, with the Ledger read copy in
    /// line either side. A refusal stays on the Presenter's view model and keeps the Dialog open.
    fn confirm(&mut self, cx: &mut Context<'_, Self>) {
        ports::refresh(&self.ledger, cx);
        let router = self.send(InstitutionFormEvent::Confirm);
        ports::refresh(&self.ledger, cx);
        self.closed(router, cx);
    }

    /// A cancelled Dialog has written nothing.
    fn cancel(&mut self, cx: &mut Context<'_, Self>) {
        let router = self.send(InstitutionFormEvent::Cancel);
        self.closed(router, cx);
    }

    /// There is no select list to close.
    fn close_open_select(&mut self) -> bool {
        false
    }

    /// Cancel and confirm: there is no other field to move to.
    fn hints(&self, keymap: &Keymap<DialogCommand>) -> Option<Vec<Hint>> {
        let key = |command| keymap.chord(command).unwrap_or_default().to_string();
        Some(vec![
            (key(BACK), crate::msg::desktop_hint_cancel()),
            (key(SELECT), crate::msg::desktop_hint_confirm()),
        ])
    }

    /// The Name draft, the checked products (comma-joined variant names) and the chosen Default
    /// unit code, for a test.
    fn readback(&self, _cx: &App) -> DialogReadback {
        let view = self.presenter.view_model();
        let mut values = institution_form::readback(&view);
        values.insert(0, ("title", view.frame.title));
        values.push(("can_confirm", view.frame.can_confirm.to_string()));
        DialogReadback {
            kind: "AddInstitution",
            values,
        }
    }
}

impl Render for InstitutionsDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let actions = Clicks::new(cx);
        institution_form::render(&self.presenter.view_model(), &actions, cx)
    }
}
