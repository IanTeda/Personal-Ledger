//! The Documents page's Dialogs as one hosted Dialog Entity (`shell::view::dialog_entity`): Add
//! and Edit, Import, Edit extracted facts, Accept all and the link picker. [`DocumentsDialog`]
//! holds the open one's `lib-documents` Presenter and the widget state that is gpui's to keep
//! (whether a select's list is open and its highlight), turns the keys the Dialog host passes on
//! and its clicks into the Presenter's events, and draws its view model (`dialog/`).
//!
//! A Presenter closes its Dialog through the Router with what the page does next; the Dialog hands
//! that to the page that opened it ([`Landed`]), then emits [`DialogClosed`] for the Shell to close
//! the host. A Toast or status message the Presenter raises goes onto the bin's queue, which the
//! input's `send` drains.

mod accept_all;
mod document_form;
mod facts;
mod import;
mod picker;

use std::path::Path;

use gpui::{App, Context, EventEmitter, IntoElement, Render, Window};
use lib_documents::presenter::{
    AcceptAllEvent, AcceptAllPresenter, DialogPorts, DocumentFormEvent, DocumentFormPresenter,
    FactsFormEvent, FactsFormPresenter, ImportEvent, ImportPresenter, LinkPickerEvent,
    LinkPickerPresenter, Purpose,
};
use lib_documents::{DocumentLink, DocumentsDialog as Which, DocumentsOutcome, DocumentsRouter};
use lib_shell::{DialogCommand, Hint};
use lib_toast::{Notice, Notify};
use lib_viper::keymap::Keymap;

use crate::documents::view::snapshot::PickerSnapshot;
use crate::form::select::SelectState;
use crate::shell::view::dialog_entity::{DialogClicks, DialogClosed, HostedDialog};
use crate::shell::view::dialog_host::{DialogKey, DialogOutcome, DialogReadback};

/// The open Documents Dialog's clicks.
type Clicks = DialogClicks<DocumentsDialog>;

/// What a closed Documents Dialog hands the page that opened it: what the page does next, or
/// `None` when there is nothing more.
pub(crate) type Landed = Box<dyn Fn(Option<DocumentsOutcome>, &mut App)>;

/// The open Documents Dialog: its Presenter and its widget state.
pub struct DocumentsDialog {
    view: View,
    landed: Landed,
}

enum View {
    /// Add or Edit, with the Document type and Key date selects' open state.
    Form(DocumentFormPresenter, SelectState, SelectState),
    Import(ImportPresenter),
    /// Edit extracted facts, with the Document type select's open state.
    Facts(FactsFormPresenter, SelectState),
    AcceptAll(AcceptAllPresenter),
    Picker(LinkPickerPresenter),
}

impl DocumentsDialog {
    /// `which`'s Presenter on the page's `ports`, or `None` when the Document it names is gone.
    /// What its confirm asks of the page goes to `landed`.
    pub(crate) fn open(which: Which, ports: DialogPorts, landed: Landed) -> Option<Self> {
        let view = match which {
            Which::Add(scope) => View::Form(
                DocumentFormPresenter::add(ports, scope),
                SelectState::default(),
                SelectState::default(),
            ),
            Which::Edit(id) => View::Form(
                DocumentFormPresenter::edit(ports, id)?,
                SelectState::default(),
                SelectState::default(),
            ),
            Which::Import => View::Import(ImportPresenter::new(ports)),
            Which::Facts(id) => {
                View::Facts(FactsFormPresenter::new(ports, id)?, SelectState::default())
            }
            Which::AcceptAll(strong) => View::AcceptAll(AcceptAllPresenter::new(strong)),
            Which::Link(id) => View::Picker(LinkPickerPresenter::new(ports, Purpose::Link(id))?),
            Which::File(id) => View::Picker(LinkPickerPresenter::new(ports, Purpose::File(id))?),
            Which::Follow(id) => {
                View::Picker(LinkPickerPresenter::new(ports, Purpose::Follow(id))?)
            }
        };
        Some(Self { view, landed })
    }

    /// Which Dialog is open, as the UI tests name it.
    pub fn kind(&self) -> &'static str {
        match &self.view {
            View::Form(presenter, ..) if presenter.view_model().editing => "edit",
            View::Form(..) => "add",
            View::Import(_) => "import",
            View::Facts(..) => "facts",
            View::AcceptAll(_) => "accept-all",
            View::Picker(presenter) => match presenter.purpose() {
                Purpose::Link(_) => "picker-link",
                Purpose::File(_) => "picker-file",
                Purpose::Follow(_) => "picker-follow",
            },
        }
    }

    /// The open link picker as a test reads it.
    pub fn picker(&self) -> Option<PickerSnapshot> {
        match &self.view {
            View::Picker(presenter) => Some(picker::snapshot(&presenter.view_model())),
            _ => None,
        }
    }

    /// Runs `handle` on the open Presenter and, once it has closed the Dialog, hands what it asks
    /// of the page over and tells the host.
    fn run(
        &mut self,
        cx: &mut Context<'_, Self>,
        handle: impl FnOnce(&mut View, &mut DialogRouter),
    ) {
        let mut router = DialogRouter::default();
        handle(&mut self.view, &mut router);
        if let Some(outcome) = router.closed {
            (self.landed)(outcome, cx);
            cx.emit(DialogClosed);
        }
        cx.notify();
    }

    /// A click on the picker's row `index`, which picks it.
    fn pick_row(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        self.run(cx, |view, router| {
            if let View::Picker(presenter) = view {
                presenter.handle(LinkPickerEvent::PickRow(index), router);
            }
        });
    }
}

/// Where a Documents Dialog's Presenter asks to go: closing, and Notices, which go onto the bin's
/// queue for `Root` to drain once the key or click returns. The page opens Dialogs and reveals.
#[derive(Default)]
struct DialogRouter {
    /// `Some` once the Presenter closed its Dialog, with what it asks of the page.
    closed: Option<Option<DocumentsOutcome>>,
}

impl DocumentsRouter for DialogRouter {
    fn reveal(&mut self, _link: DocumentLink) {}

    fn open_dialog(&mut self, _dialog: Which) -> bool {
        false
    }

    fn open_file(&mut self, _path: &Path, _in_folder: bool) {}

    fn close_dialog(&mut self, outcome: Option<DocumentsOutcome>) {
        self.closed = Some(outcome);
    }
}

impl Notify for DialogRouter {
    fn notify(&mut self, notice: Notice) {
        crate::notices::Queue.notify(notice);
    }
}

/// Sends `event` to `presenter` where it can't close the Dialog: a field changing.
fn send<P, E>(presenter: &mut P, event: E, handle: fn(&mut P, E, &mut dyn DocumentsRouter)) {
    handle(presenter, event, &mut DialogRouter::default());
}

impl EventEmitter<DialogClosed> for DocumentsDialog {}

impl HostedDialog for DocumentsDialog {
    fn key(&mut self, key: DialogKey, _cx: &mut Context<'_, Self>) -> DialogOutcome {
        match &mut self.view {
            View::Form(presenter, doc_type, key_kind) => {
                document_form::key(presenter, doc_type, key_kind, key)
            }
            View::Import(presenter) => import::key(presenter, key),
            View::Facts(presenter, doc_type) => facts::key(presenter, doc_type, key),
            View::AcceptAll(_) => accept_all::key(key),
            View::Picker(presenter) => picker::key(presenter, key),
        }
    }

    fn can_confirm(&self) -> bool {
        match &self.view {
            View::Form(presenter, ..) => presenter.view_model().frame.can_confirm,
            View::Import(presenter) => presenter.view_model().frame.can_confirm,
            View::Facts(presenter, _) => presenter.view_model().frame.can_confirm,
            View::AcceptAll(_) | View::Picker(_) => true,
        }
    }

    /// Sends the Presenter its Confirm, the picker's being a pick of the highlighted row. A refusal
    /// stays on the view model and keeps the Dialog open.
    fn confirm(&mut self, cx: &mut Context<'_, Self>) {
        self.run(cx, |view, router| match view {
            View::Form(presenter, ..) => presenter.handle(DocumentFormEvent::Confirm, router),
            View::Import(presenter) => presenter.handle(ImportEvent::Confirm, router),
            View::Facts(presenter, _) => presenter.handle(FactsFormEvent::Confirm, router),
            View::AcceptAll(presenter) => presenter.handle(AcceptAllEvent::Confirm, router),
            View::Picker(presenter) => presenter.handle(LinkPickerEvent::Pick, router),
        });
    }

    /// A cancelled Documents Dialog has written nothing.
    fn cancel(&mut self, cx: &mut Context<'_, Self>) {
        self.run(cx, |view, router| match view {
            View::Form(presenter, ..) => presenter.handle(DocumentFormEvent::Cancel, router),
            View::Import(presenter) => presenter.handle(ImportEvent::Cancel, router),
            View::Facts(presenter, _) => presenter.handle(FactsFormEvent::Cancel, router),
            View::AcceptAll(presenter) => presenter.handle(AcceptAllEvent::Cancel, router),
            View::Picker(presenter) => presenter.handle(LinkPickerEvent::Cancel, router),
        });
    }

    fn close_open_select(&mut self) -> bool {
        let close = |state: &mut SelectState| {
            let open = state.is_open();
            state.cancel();
            open
        };
        match &mut self.view {
            View::Form(_, doc_type, key_kind) => close(doc_type) | close(key_kind),
            View::Facts(_, doc_type) => close(doc_type),
            View::Import(_) | View::AcceptAll(_) | View::Picker(_) => false,
        }
    }

    fn hints(&self, keymap: &Keymap<DialogCommand>) -> Option<Vec<Hint>> {
        Some(match &self.view {
            View::Form(presenter, ..) => presenter.legend(keymap),
            View::Import(presenter) => presenter.legend(keymap),
            View::Facts(presenter, _) => presenter.legend(keymap),
            View::AcceptAll(presenter) => presenter.legend(keymap),
            View::Picker(presenter) => presenter.legend(keymap),
        })
    }

    /// The open Dialog's title, what it shows and whether it would confirm, for a test.
    fn readback(&self, _cx: &App) -> DialogReadback {
        let mut values: Vec<(&'static str, String)> = Vec::new();
        match &self.view {
            View::Form(presenter, ..) => {
                let view = presenter.view_model();
                values.push(("title", view.frame.title));
                if let Some(path) = view.path {
                    values.push(("path", path.text));
                }
                values.push(("document_title", view.title.text));
                values.push(("date", view.date.text));
                values.push((
                    "error",
                    view.path_error.or(view.date_error).unwrap_or_default(),
                ));
                values.push(("can_confirm", view.frame.can_confirm.to_string()));
            }
            View::Import(presenter) => {
                let view = presenter.view_model();
                values.push(("title", view.frame.title));
                values.push(("paths", view.paths));
                values.push(("notes", view.notes.join("\n")));
                values.push(("can_confirm", view.frame.can_confirm.to_string()));
            }
            View::Facts(presenter, _) => {
                let view = presenter.view_model();
                values.push(("title", view.frame.title));
                values.push(("merchant", view.merchant.text));
                values.push(("total", view.total.text));
                values.push((
                    "error",
                    view.total_error.or(view.date_error).unwrap_or_default(),
                ));
                values.push(("can_confirm", view.frame.can_confirm.to_string()));
            }
            View::AcceptAll(presenter) => {
                let view = presenter.view_model();
                values.push(("title", view.frame.title));
                values.push(("body", view.body));
            }
            View::Picker(presenter) => values.push(("title", presenter.view_model().title)),
        }
        DialogReadback {
            kind: self.kind(),
            values,
        }
    }
}

impl Render for DocumentsDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let actions = Clicks::new(cx);
        match &self.view {
            View::Form(presenter, doc_type, key_kind) => {
                document_form::render(&presenter.view_model(), doc_type, key_kind, &actions, cx)
            }
            View::Import(presenter) => import::render(&presenter.view_model(), &actions, cx),
            View::Facts(presenter, doc_type) => {
                facts::render(&presenter.view_model(), doc_type, &actions, cx)
            }
            View::AcceptAll(presenter) => accept_all::render(&presenter.view_model(), &actions, cx),
            View::Picker(presenter) => picker::render(&presenter.view_model(), &actions, cx),
        }
    }
}
