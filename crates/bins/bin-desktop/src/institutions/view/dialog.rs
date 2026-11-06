//! The Institutions Add and Edit Dialog's View, an Entity in the Dialog host (`shell::view::dialog_entity`).
//! [`InstitutionsDialog`] holds the `lib-institutions` form Presenter. Every key and click becomes
//! a Presenter event, or, for a Dialog command or typed key, a `lib-viper` `DialogInput` the
//! Presenter reads itself; the View draws the Presenter's view model and nothing else.
//!
//! Confirm and Cancel go to the Presenter, which closes its Dialog through the host's
//! [`HostRouter`]; a confirmed
//! outcome is handed to the page, and the host closes on [`DialogClosed`]. The Toast the Presenter
//! raises goes onto the bin's queue, which the input's `send` drains.

mod delete_institution;
mod institution_form;

pub(crate) use delete_institution::DeleteInstitutionDialog;

use gpui::{App, Context, Entity, EventEmitter, IntoElement, Render, Window};
use lib_institutions::presenter::{InstitutionFormEvent, InstitutionFormPresenter};
use lib_institutions::{InstitutionsDialog as Which, InstitutionsOutcome};
use lib_shell::DialogCommand;
use lib_viper::dialog::DialogInput;
use lib_viper::keymap::{BACK, Keymap, NEXT_FIELD, SELECT};

use crate::announcer::Announcer;
use crate::institutions::ports::{self, DesktopPorts};
use crate::shell::view::dialog_entity::DialogReadback;
use crate::shell::view::dialog_entity::{DialogClicks, DialogClosed, HostRouter, HostedDialog};
use lib_shell::Hint;

/// The open Dialog's clicks.
type Clicks = DialogClicks<InstitutionsDialog>;

/// What a confirmed Dialog hands the page that opened it.
pub(crate) type Landed = Box<dyn Fn(InstitutionsOutcome, &mut App)>;

/// The open Institutions Dialog: its Presenter.
pub struct InstitutionsDialog {
    ports: DesktopPorts,
    /// The Ledger a confirm announces its write through.
    announcer: Entity<Announcer>,
    presenter: InstitutionFormPresenter,
    /// The top row of the Usual unit list's window, which the View keeps between draws.
    scroll: usize,
    /// Which Dialog this is, for the test readback.
    which: Which,
    landed: Landed,
}

impl InstitutionsDialog {
    /// `which`'s Presenter on `ports`, or `None` when it is an Edit of an Institution that has
    /// gone. A confirm announces what it wrote and hands its outcome to `landed`.
    pub(crate) fn open(
        which: Which,
        ports: &DesktopPorts,
        announcer: &Entity<Announcer>,
        landed: Landed,
    ) -> Option<Self> {
        let presenter = match which {
            Which::AddInstitution => InstitutionFormPresenter::add(ports.clone()),
            Which::EditInstitution(id) => InstitutionFormPresenter::edit(ports.clone(), id)?,
            // The Delete Dialog is its own Entity (`DeleteInstitutionDialog`).
            Which::DeleteInstitution(_) => return None,
        };
        Some(Self {
            ports: ports.clone(),
            announcer: announcer.clone(),
            presenter,
            scroll: 0,
            which,
            landed,
        })
    }

    /// Sends the Presenter `event`, returning what its Router was asked.
    fn send(&mut self, event: InstitutionFormEvent) -> Router {
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
}

/// The host's Router, as an Institutions Dialog's Presenter closes through it.
type Router = HostRouter<InstitutionsOutcome>;

impl EventEmitter<DialogClosed> for InstitutionsDialog {}

impl HostedDialog for InstitutionsDialog {
    /// Every input is the Presenter's to read. One that may confirm (`select`, `submit`) announces
    /// the Institutions written before it and after it.
    fn input(&mut self, input: DialogInput, cx: &mut Context<'_, Self>) {
        let may_confirm = matches!(input, DialogInput::Select | DialogInput::Submit);
        if may_confirm {
            ports::refresh(&self.ports, &self.announcer, cx);
        }
        let router = self.send(InstitutionFormEvent::Input(input));
        if may_confirm {
            ports::refresh(&self.ports, &self.announcer, cx);
        }
        self.closed(router, cx);
    }

    fn can_confirm(&self) -> bool {
        self.presenter.view_model().frame.can_confirm
    }

    /// Sends Confirm to the Presenter, which runs its Interactor, announcing the
    /// Institutions written either side. A refusal stays on the Presenter's view model and keeps the Dialog open.
    fn confirm(&mut self, cx: &mut Context<'_, Self>) {
        ports::refresh(&self.ports, &self.announcer, cx);
        let router = self.send(InstitutionFormEvent::Confirm);
        ports::refresh(&self.ports, &self.announcer, cx);
        self.closed(router, cx);
    }

    /// A cancelled Dialog has written nothing.
    fn cancel(&mut self, cx: &mut Context<'_, Self>) {
        let router = self.send(InstitutionFormEvent::Cancel);
        self.closed(router, cx);
    }

    fn close_open_select(&mut self) -> bool {
        let open = self.presenter.view_model().usual_unit_list.is_open();
        if open {
            self.send(InstitutionFormEvent::CloseUnits);
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

    /// The Name draft, the checked products (comma-joined variant names) and the chosen Usual
    /// unit code, for a test.
    fn readback(&self, _cx: &App) -> DialogReadback {
        let view = self.presenter.view_model();
        let mut values = institution_form::readback(&view);
        values.insert(0, ("title", view.frame.title));
        values.push(("can_confirm", view.frame.can_confirm.to_string()));
        DialogReadback {
            kind: match self.which {
                Which::AddInstitution => "AddInstitution",
                Which::EditInstitution(_) => "EditInstitution",
                Which::DeleteInstitution(_) => "DeleteInstitution",
            },
            values,
        }
    }
}

impl Render for InstitutionsDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let actions = Clicks::new(cx);
        institution_form::render(&self.presenter.view_model(), &mut self.scroll, &actions, cx)
    }
}
