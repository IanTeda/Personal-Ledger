//! The Document Types Settings page's View: the `lib-documents` page Presenter in a gpui Entity. It
//! looks up each key the Shell passes on, as chord text, in the Document Types keymap, turns clicks
//! into events, draws the view model (`render.rs`), and announces the Documents after every event
//! so the Documents View redraws ([`ports::announce`]). Each key and click is one input (`crate::input::send`), which
//! commits what it wrote. The Settings View hosts it as any other page.

mod render;

use gpui::{AnyElement, App, Context, Entity, EventEmitter, IntoElement as _, Subscription};
use lib_documents::presenter::{
    DocumentTypesPageEvent, DocumentTypesPagePresenter, DocumentTypesPageViewModel,
};
use lib_documents::{DocumentTypesOutcome, SharedRepository};
use lib_ledger::Domain;
use lib_shell::Hint;
use lib_viper::keymap::Keymap;

use crate::document_types::router::PageRouter;
use crate::document_types::{DocumentTypesSetup, SettingsDocumentTypesSnapshot, ports};
use crate::settings::snapshot::PageSnapshot;
use crate::{
    ledger_store::{LedgerStore, notify_on},
    navigation::movement::Movement,
    settings::{SettingsPage, page::PageRowClicked, pages::Page},
    view::event::ViewEvent,
};

impl EventEmitter<ViewEvent> for DocumentTypesPage {}
impl EventEmitter<PageRowClicked> for DocumentTypesPage {}

pub struct DocumentTypesPage {
    presenter: DocumentTypesPagePresenter,
    documents: SharedRepository,
    ledger: Entity<LedgerStore>,
    keymap: Keymap<DocumentTypesPageEvent>,
    /// Whether the page, not Settings' index, has keyboard focus, told by the Settings View as it
    /// draws the page.
    page_focused: bool,
    /// Repaints the page when the Document Types its rows read change.
    _types_changed: Subscription,
}

impl DocumentTypesPage {
    pub fn new(
        setup: DocumentTypesSetup,
        ledger: Entity<LedgerStore>,
        cx: &mut Context<'_, Self>,
    ) -> Self {
        let documents = ports::shared(&setup.store);
        Self {
            presenter: DocumentTypesPagePresenter::new(documents.clone(), setup.keymap.clone()),
            documents,
            _types_changed: notify_on(&ledger, Domain::DocumentTypes, cx),
            ledger,
            keymap: setup.keymap,
            page_focused: false,
        }
    }

    /// The one dispatch a keymap event takes into the page, from a key ([`Page::module_key`]) or a
    /// palette Command ([`Page::module_command`]): sent as one input.
    fn take(&mut self, event: DocumentTypesPageEvent, cx: &mut Context<'_, Self>) -> bool {
        self.send(event, cx);
        true
    }

    /// A key or click on the page: `event` as one input, committed once it is handled.
    pub(in crate::document_types) fn send(
        &mut self,
        event: DocumentTypesPageEvent,
        cx: &mut Context<'_, Self>,
    ) {
        crate::input::send(cx, |cx| self.handle(event, cx));
    }

    /// Hands `event` to the Presenter with the Desktop's Router, then announces whatever it wrote.
    /// Inside an input already, as [`Self::send`] or a Dialog's
    /// confirm.
    fn handle(&mut self, event: DocumentTypesPageEvent, cx: &mut Context<'_, Self>) {
        let mut router = PageRouter {
            documents: self.documents.clone(),
            ledger: self.ledger.clone(),
            cx,
        };
        self.presenter.handle(event, &mut router);
        ports::announce(&self.ledger, cx);
        cx.notify();
    }

    /// A confirmed Dialog's outcome, handed back by the Dialog host inside the confirm's input.
    pub(in crate::document_types) fn dialog_landed(
        &mut self,
        outcome: DocumentTypesOutcome,
        cx: &mut Context<'_, Self>,
    ) {
        self.handle(DocumentTypesPageEvent::DialogClosed(Some(outcome)), cx);
    }

    pub fn view_model(&self) -> DocumentTypesPageViewModel {
        self.presenter.view_model()
    }

    /// A click on a row or its controls: `event`, and focus on the page.
    fn click(&mut self, event: DocumentTypesPageEvent, cx: &mut Context<'_, Self>) {
        cx.emit(PageRowClicked);
        self.send(event, cx);
    }
}

impl PageSnapshot for SettingsDocumentTypesSnapshot {
    type Page = DocumentTypesPage;

    fn read(page: &DocumentTypesPage, _cx: &App) -> Self {
        let view = page.view_model();
        SettingsDocumentTypesSnapshot {
            selected_document_type: view
                .rows
                .iter()
                .find(|row| Some(row.id) == view.selected)
                .map(|row| row.name.clone()),
            document_type_files: view.rows.iter().map(|row| row.files).collect(),
            document_type_ids: view.rows.iter().map(|row| row.id).collect(),
            document_type_names: view.rows.into_iter().map(|row| row.name).collect(),
        }
    }
}

impl Page for DocumentTypesPage {
    const PAGE: SettingsPage = SettingsPage::Documents;

    /// Every chord the Document Types keymap binds, the arrows, `g g` and `J`/`K` among them. The
    /// Shell has already taken its own keys.
    fn module_key(&mut self, chord: &str, cx: &mut Context<'_, Self>) -> bool {
        let Some(event) = self.keymap.event(chord) else {
            return false;
        };
        self.take(event, cx)
    }

    /// A palette Command, by its keymap `command` name: the event its key sends.
    fn module_command(&mut self, command: &str, cx: &mut Context<'_, Self>) -> bool {
        let Some(event) = self.keymap.event_for(command) else {
            return false;
        };
        self.take(event, cx)
    }

    /// A movement the keymap doesn't bind does nothing on the page, and Settings leaves the page
    /// where it is: there is no hand-off from a Document Type.
    fn apply_movement(&mut self, _movement: Movement, _cx: &mut Context<'_, Self>) -> bool {
        true
    }

    /// The Presenter's legend, rebound keys included, which the `?` cheat sheet reads.
    fn hints(&self, _cx: &App) -> Vec<Hint> {
        self.view_model().hints
    }

    fn status_note(&self, _cx: &App) -> String {
        self.view_model().scope
    }

    fn set_page_focused(&mut self, focused: bool, _cx: &mut Context<'_, Self>) {
        self.page_focused = focused;
    }

    fn heading_meta(&self, _cx: &App) -> AnyElement {
        self.view_model().scope.into_any_element()
    }
}
