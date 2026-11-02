//! The Document Types Dialogs' Views, each open one an Entity in the Dialog host
//! (`shell::view::dialog_entity`). [`DocumentTypesDialog`] holds a `lib-documents` Presenter and the
//! widget state that is gpui's to keep (whether the Remove destination list is open and its
//! highlight). Every key and click becomes a Presenter event; each View draws the Presenter's view
//! model and nothing else.
//!
//! Confirm and Cancel go to the Presenter, which closes its Dialog through its Router; a confirmed
//! outcome is handed to the page, and the host closes on [`DialogClosed`].

mod default_notice;
mod remove_type;
mod type_form;

use gpui::{App, Context, Entity, EventEmitter, IntoElement, Render, Window};
use lib_documents::presenter::{
    DefaultNoticeEvent, DefaultNoticePresenter, RemoveTypeEvent, RemoveTypePresenter,
    TypeFormEvent, TypeFormPresenter,
};
use lib_documents::{
    DocumentTypesDialog as Which, DocumentTypesOutcome, DocumentTypesRouter, SharedRepository,
};
use lib_shell::{DialogCommand, Hint};
use lib_toast::{Notice, Notify};
use lib_viper::keymap::{BACK, Keymap, NEXT_FIELD, SELECT};

use crate::document_types::ports;
use crate::form::select::SelectState;
use crate::ledger_store::LedgerStore;
use crate::shell::view::dialog_entity::{DialogClicks, DialogClosed, HostedDialog};
use crate::shell::view::dialog_host::{DialogKey, DialogOutcome, DialogReadback};

/// The open Document Types Dialog's clicks.
type Clicks = DialogClicks<DocumentTypesDialog>;

/// What a confirmed Document Types Dialog hands the page that opened it.
pub(crate) type Landed = Box<dyn Fn(DocumentTypesOutcome, &mut App)>;

/// The open Document Types Dialog: its Presenter and its widget state.
pub struct DocumentTypesDialog {
    /// The Ledger store a confirm announces what it wrote through.
    ledger: Entity<LedgerStore>,
    view: View,
    landed: Landed,
}

enum View {
    /// Add or Edit.
    Form(TypeFormPresenter),
    /// Remove, with the destination select's open state.
    Remove(RemoveTypePresenter, SelectState),
    /// What Other, the Default, gets in place of Remove.
    Notice(DefaultNoticePresenter),
}

impl View {
    /// Which Dialog is open, as the UI tests name it.
    fn kind(&self) -> &'static str {
        match self {
            Self::Form(presenter) if presenter.view_model().editing => "EditDocumentType",
            Self::Form(_) => "AddDocumentType",
            Self::Remove(..) => "RemoveDocumentType",
            Self::Notice(_) => "DefaultNotice",
        }
    }
}

impl DocumentTypesDialog {
    /// `which`'s Presenter on `documents`, or `None` when the type it names is gone. A confirm
    /// brings `ledger` in line and hands its outcome to `landed`.
    pub(crate) fn open(
        which: Which,
        documents: &SharedRepository,
        ledger: &Entity<LedgerStore>,
        landed: Landed,
    ) -> Option<Self> {
        let shared = documents.clone();
        let view = match which {
            Which::Add => View::Form(TypeFormPresenter::add(shared)),
            Which::Edit(id) => View::Form(TypeFormPresenter::edit(shared, id)?),
            Which::Remove(id) => View::Remove(
                RemoveTypePresenter::new(shared, id)?,
                SelectState::default(),
            ),
            Which::DefaultNotice => View::Notice(DefaultNoticePresenter),
        };
        Some(Self {
            ledger: ledger.clone(),
            view,
            landed,
        })
    }

    /// Sends the open Presenter its Confirm or its Cancel, returning what its Router was asked.
    fn close_event(&mut self, confirm: bool) -> DialogRouter {
        match &mut self.view {
            View::Form(presenter) => {
                let event = if confirm {
                    TypeFormEvent::Confirm
                } else {
                    TypeFormEvent::Cancel
                };
                send(presenter, event, TypeFormPresenter::handle)
            }
            View::Remove(presenter, _) => {
                let event = if confirm {
                    RemoveTypeEvent::Confirm
                } else {
                    RemoveTypeEvent::Cancel
                };
                send(presenter, event, RemoveTypePresenter::handle)
            }
            View::Notice(presenter) => {
                let event = if confirm {
                    DefaultNoticeEvent::Confirm
                } else {
                    DefaultNoticeEvent::Cancel
                };
                send(presenter, event, DefaultNoticePresenter::handle)
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

/// Where a Document Types Dialog's Presenter asks to go: closing, and Notices, which go onto the
/// bin's queue for `Root` to drain once the key or click returns. The page opens Dialogs.
#[derive(Default)]
struct DialogRouter {
    /// `Some` once the Presenter closed its Dialog, with what its confirm did.
    closed: Option<Option<DocumentTypesOutcome>>,
}

impl DocumentTypesRouter for DialogRouter {
    fn open_dialog(&mut self, _dialog: Which) -> bool {
        false
    }

    fn close_dialog(&mut self, outcome: Option<DocumentTypesOutcome>) {
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
    handle: fn(&mut P, E, &mut dyn DocumentTypesRouter),
) -> DialogRouter {
    let mut router = DialogRouter::default();
    handle(presenter, event, &mut router);
    router
}

impl EventEmitter<DialogClosed> for DocumentTypesDialog {}

impl HostedDialog for DocumentTypesDialog {
    /// Every key is the Dialog's own: the Presenter holds every field.
    fn key(&mut self, key: DialogKey, _cx: &mut Context<'_, Self>) -> DialogOutcome {
        match &mut self.view {
            View::Form(presenter) => type_form::key(presenter, key),
            View::Remove(presenter, select) => remove_type::key(presenter, select, key),
            View::Notice(_) => default_notice::key(key),
        }
    }

    fn can_confirm(&self) -> bool {
        match &self.view {
            View::Form(presenter) => presenter.view_model().frame.can_confirm,
            View::Remove(presenter, _) => presenter.view_model().frame.can_confirm,
            View::Notice(presenter) => presenter.view_model().frame.can_confirm,
        }
    }

    /// Sends Confirm to the Presenter, which runs its Interactor, then announces what it wrote. A
    /// refusal stays on the Presenter's view model and keeps the Dialog open.
    fn confirm(&mut self, cx: &mut Context<'_, Self>) {
        let router = self.close_event(true);
        ports::announce(&self.ledger, cx);
        self.closed(router, cx);
    }

    /// A cancelled Document Types Dialog has written nothing.
    fn cancel(&mut self, cx: &mut Context<'_, Self>) {
        let router = self.close_event(false);
        self.closed(router, cx);
    }

    fn close_open_select(&mut self) -> bool {
        match &mut self.view {
            View::Remove(_, select) => {
                let open = select.is_open();
                select.cancel();
                open
            }
            View::Form(_) | View::Notice(_) => false,
        }
    }

    /// Cancel and confirm, and on the form, the key that moves field.
    fn hints(&self, keymap: &Keymap<DialogCommand>) -> Option<Vec<Hint>> {
        let key = |command| keymap.chord(command).unwrap_or_default().to_string();
        let mut hints = vec![
            Hint::new(key(BACK), crate::msg::desktop_hint_cancel()),
            Hint::new(key(SELECT), crate::msg::desktop_hint_confirm()),
        ];
        if matches!(self.view, View::Form(_)) {
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
            View::Form(presenter) => {
                let view = presenter.view_model();
                (view.frame.clone(), type_form::readback(&view))
            }
            View::Remove(presenter, _) => {
                let view = presenter.view_model();
                (view.frame.clone(), remove_type::readback(&view))
            }
            View::Notice(presenter) => {
                let view = presenter.view_model();
                (view.frame.clone(), default_notice::readback(&view))
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

impl Render for DocumentTypesDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let actions = Clicks::new(cx);
        match &self.view {
            View::Form(presenter) => type_form::render(&presenter.view_model(), &actions, cx),
            View::Remove(presenter, select) => {
                remove_type::render(&presenter.view_model(), select, &actions, cx)
            }
            View::Notice(presenter) => {
                default_notice::render(&presenter.view_model(), &actions, cx)
            }
        }
    }
}
