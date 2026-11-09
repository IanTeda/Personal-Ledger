//! The Documents page's View: the `lib-documents` page Presenter in a gpui Entity, which the Shell
//! hosts as the Documents destination (`docs/ux/desktop-mockups/04-documents/`). It looks up each
//! key the Shell passes on, as chord text, in the Documents keymap and sends the event when the
//! Presenter has a use for it in the zone and mode showing, turns clicks into events, draws the
//! view model (`super::render`). Its Dialogs open as Entities in the Dialog host
//! (`super::dialog`), and what each asks of the page as it closes comes back through
//! [`DocumentsPage::dialog_landed`]. Each key and click is one input (`crate::input::send`), which
//! commits what it wrote.
//!
//! The Library's Search is the Shell's Search mode: `/` comes in as [`DocumentsPage::start_search`]
//! and the typed keys as [`DocumentsPage::search_key`]. The page leaves the mode as the Presenter
//! closes the Search.

use std::path::PathBuf;
use std::rc::Rc;

use gpui::{
    App, Context, Entity, EventEmitter, Keystroke, ScrollHandle, ScrollStrategy, Subscription,
    UniformListScrollHandle, Window, prelude::*,
};
use lib_core::{Clock, DateStyle};
use lib_database::Domain;
use lib_documents::presenter::{DocumentsPageEvent, DocumentsPagePresenter};
use lib_documents::{
    DocumentsMode, DocumentsOutcome, LibraryScope, LibrarySort, SharedFiles, SharedRepository,
};
use lib_viper::dialog::DialogKeys;
use lib_viper::keymap::Keymap;

use super::Scrolls;
use crate::announcer::{Announcer, LedgerChanged};
use crate::documents::router::PageRouter;
use crate::navigation::movement::Movement;
use crate::shell::view::page_clicks::{PageClicks, PresenterPage};
use crate::shell::view::statusline::PageStatus;
use crate::view::event::{Reveal, ViewEvent, ViewMode};
use crate::view::help::HelpGroup;

impl EventEmitter<ViewEvent> for DocumentsPage {}

pub struct DocumentsPage {
    presenter: DocumentsPagePresenter,
    keymap: Keymap<DocumentsPageEvent>,
    /// Whether the View zone has focus, which the index rail's and the list's focus rings draw
    /// within. Handed over by the Shell as it draws the page.
    zone_focused: bool,
    library_scroll: UniformListScrollHandle,
    inbox_scroll: ScrollHandle,
    /// The Dialog keymap's chords, which the page's Dialogs' hints show.
    dialog_keys: DialogKeys,
    _ledger_changed: Subscription,
}

impl DocumentsPage {
    pub fn new(
        presenter: DocumentsPagePresenter,
        announcer: &Entity<Announcer>,
        keymap: Keymap<DocumentsPageEvent>,
        dialog_keys: DialogKeys,
        cx: &mut Context<'_, Self>,
    ) -> Self {
        // The rows name records from every domain a Link reaches, and Settings › Documents writes
        // the Document Types: a change to any is redrawn.
        let _ledger_changed = cx.subscribe(announcer, |_, _, event: &LedgerChanged, cx| {
            let reads = [
                Domain::Accounts,
                Domain::Bills,
                Domain::DocumentTypes,
                Domain::Documents,
                Domain::Inventory,
                Domain::Payees,
                Domain::Transactions,
            ];
            if reads.iter().any(|domain| event.what.contains(domain)) {
                cx.notify();
            }
        });
        Self {
            presenter,
            keymap,
            zone_focused: false,
            library_scroll: UniformListScrollHandle::new(),
            inbox_scroll: ScrollHandle::new(),
            dialog_keys,
            _ledger_changed,
        }
    }

    /// A key or click on the page, or the Shell telling it something: `event` as one input,
    /// committed once it is handled.
    fn send(&mut self, event: DocumentsPageEvent, cx: &mut Context<'_, Self>) {
        crate::input::join_or_send(cx, |cx| self.handle(event, cx));
    }

    /// Hands `event` to the Presenter with the Desktop's Router, then brings the Shell's Search
    /// mode in line with the Presenter's Search, and the scroll with the selection. Inside an input already, as [`Self::send`] or a Dialog's confirm.
    fn handle(&mut self, event: DocumentsPageEvent, cx: &mut Context<'_, Self>) {
        let searching = self.presenter.searching();
        self.handle_quietly(event, cx);
        match (searching, self.presenter.searching()) {
            (false, true) => cx.emit(ViewEvent::EnterMode(ViewMode::Search)),
            (true, false) => cx.emit(ViewEvent::ExitMode),
            _ => {}
        }
    }

    /// [`Self::handle`] without the Search mode: for `/`, whose mode the Shell enters itself.
    fn handle_quietly(&mut self, event: DocumentsPageEvent, cx: &mut Context<'_, Self>) {
        let (mode, before) = (self.presenter.mode(), self.presenter.selected());
        let mut router = PageRouter {
            ports: self.presenter.dialog_ports(),
            keys: self.dialog_keys.clone(),
            cx,
        };
        self.presenter.handle(event, &mut router);
        self.follow_selection(mode, before);
        cx.notify();
    }

    /// Back to the top as the mode changes; otherwise the selected row scrolled into view, landing
    /// at the edge it came from, as on Transactions.
    fn follow_selection(&mut self, mode: DocumentsMode, before: usize) {
        let at = self.presenter.selected();
        if self.presenter.mode() != mode {
            self.library_scroll.scroll_to_item(0, ScrollStrategy::Top);
            self.inbox_scroll.set_offset(gpui::Point::default());
            return;
        }
        match mode {
            DocumentsMode::Library => {
                let strategy = if at < before {
                    ScrollStrategy::Top
                } else {
                    ScrollStrategy::Bottom
                };
                self.library_scroll.scroll_to_item(at, strategy);
            }
            DocumentsMode::Inbox => self.inbox_scroll.scroll_to_item(at),
        }
    }

    /// A Documents Dialog the page opened has closed, inside the input that closed it: `outcome`
    /// is what it asks of the page, or `None` when it asks nothing but a redraw of what it wrote.
    pub fn dialog_landed(&mut self, outcome: Option<DocumentsOutcome>, cx: &mut Context<'_, Self>) {
        match outcome {
            Some(outcome) => self.handle(DocumentsPageEvent::DialogClosed(outcome), cx),
            None => {
                cx.notify();
            }
        }
    }

    /// Files dropped anywhere on the window: into the Inbox, the page staying where it is.
    pub fn drop_paths(&mut self, paths: &[PathBuf], cx: &mut Context<'_, Self>) {
        let typed = paths
            .iter()
            .map(|path| path.display().to_string())
            .collect();
        self.send(DocumentsPageEvent::Drop(typed), cx);
    }

    /// How many Documents wait in the Inbox, for the rail's badge.
    pub fn inbox_count(&self) -> usize {
        self.presenter.inbox_count()
    }

    /// The mode, scope and sort that persist across restarts.
    pub fn persisted(&self) -> (DocumentsMode, LibraryScope, LibrarySort) {
        self.presenter.persisted()
    }

    /// Restores the mode, scope and sort from the last run. It writes nothing, so it is no input.
    pub fn set_persisted(
        &mut self,
        mode: DocumentsMode,
        scope: LibraryScope,
        sort: LibrarySort,
        cx: &mut Context<'_, Self>,
    ) {
        self.handle(DocumentsPageEvent::Restore { mode, scope, sort }, cx);
    }

    pub(in crate::documents) fn presenter(&self) -> &DocumentsPagePresenter {
        &self.presenter
    }

    /// Records whether the View zone has focus, as the Shell draws the page.
    pub fn set_zone_focused(&mut self, focused: bool) {
        self.zone_focused = focused;
    }

    /// Follows a change to `[keybindings]`.
    pub fn set_keymaps(&mut self, config: &lib_config::KeyBindingConfig) {
        self.keymap = lib_documents::keymap::documents_page_keymap(config);
        self.presenter.set_keymap(self.keymap.clone());
    }

    /// Every chord the Documents keymap binds that the zone and mode showing have a use for. A key
    /// they don't is left to the Shell; `a` in the Inbox is swallowed so it never enters Insert.
    pub fn module_key(&mut self, chord: &str, cx: &mut Context<'_, Self>) -> bool {
        let Some(event) = self.keymap.event(chord) else {
            return false;
        };
        if !self.presenter.takes(&event) {
            return false;
        }
        self.send(event, cx);
        true
    }

    /// A palette Command, by its keymap `command` name: the event its key sends, in the mode it
    /// belongs to. A Command of the other mode switches to it first, in the same input.
    pub fn module_command(&mut self, command: &str, cx: &mut Context<'_, Self>) -> bool {
        let Some(event) = self.keymap.event_for(command) else {
            return false;
        };
        let switch = event
            .mode()
            .filter(|mode| *mode != self.presenter.mode())
            .map(|mode| match mode {
                DocumentsMode::Inbox => DocumentsPageEvent::ShowInbox,
                DocumentsMode::Library => DocumentsPageEvent::ShowLibrary,
            });
        crate::input::join_or_send(cx, |cx| {
            if let Some(switch) = switch {
                self.handle(switch, cx);
            }
            self.handle(event, cx);
        });
        true
    }

    /// Every Documents key is the keymap's.
    pub fn key_ahead(&mut self, _keystroke: &Keystroke, _cx: &mut Context<'_, Self>) -> bool {
        false
    }

    /// Every Documents key is the keymap's.
    pub fn handle_key(&mut self, _keystroke: &Keystroke, _cx: &mut Context<'_, Self>) -> bool {
        false
    }

    /// A movement the keymap doesn't bind does nothing on the page.
    pub fn apply_movement(&mut self, _movement: Movement, _cx: &mut Context<'_, Self>) {}

    /// `Esc`, innermost first: the Search (its query cleared), then a kept query, then the list
    /// back to the index.
    pub fn escape(&mut self, cx: &mut Context<'_, Self>) -> bool {
        if !self.presenter.takes_back() {
            return false;
        }
        self.send(DocumentsPageEvent::Back, cx);
        true
    }

    /// The Library's `/`: the Search, from either zone. Inert in the Inbox, which has none. The
    /// Shell enters Search mode on `true`. Not an input of its own: the root Router calls it inside
    /// the input `/` arrived in, which commits it.
    pub fn start_search(&mut self, cx: &mut Context<'_, Self>) -> bool {
        self.handle_quietly(DocumentsPageEvent::StartSearch, cx);
        self.presenter.searching()
    }

    /// Keys while the Search is open: typing narrows live, `Backspace` edits, `Enter` keeps the
    /// query and returns to the list. `Esc` comes back as [`Self::escape`].
    pub fn search_key(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        let event = match keystroke.key.as_str() {
            "enter" => DocumentsPageEvent::KeepSearch,
            "backspace" => DocumentsPageEvent::SearchBackspace,
            _ => {
                let modifiers = &keystroke.modifiers;
                if modifiers.control || modifiers.alt || modifiers.platform || modifiers.function {
                    return false;
                }
                let mut chars = keystroke.key_char.as_deref().unwrap_or_default().chars();
                match (chars.next(), chars.next()) {
                    (Some(ch), None) if !ch.is_control() => DocumentsPageEvent::SearchType(ch),
                    _ => return false,
                }
            }
        };
        self.send(event, cx);
        true
    }

    /// The page has just become the destination: `g f` and the palette land on the list. A Search
    /// left open does not come back with it.
    pub fn on_enter(&mut self, cx: &mut Context<'_, Self>) {
        self.send(DocumentsPageEvent::Entered, cx);
    }

    /// No other View hands a record to Documents yet.
    pub fn reveal(&mut self, _reveal: Reveal, _cx: &mut Context<'_, Self>) {}

    /// The calendar month turned while the Desktop was open: inside the day poll's input.
    pub fn month_turned(&mut self, cx: &mut Context<'_, Self>) {
        self.handle(DocumentsPageEvent::MonthTurned, cx);
    }

    /// Follows a change to the Display Preferences.
    pub fn set_display(&mut self, date_style: Option<DateStyle>, cx: &mut Context<'_, Self>) {
        self.send(DocumentsPageEvent::SetDateStyle(date_style), cx);
    }

    /// The status line's legend for the zone and mode, and the stored size.
    pub fn page_status(&self, _cx: &App) -> Option<PageStatus> {
        let view = self.presenter.view_model();
        Some(PageStatus {
            hints: view.hints,
            right: view.stored,
        })
    }

    /// The `?` cheat-sheet's Documents group: the index keys, then the mode's, as the keymap binds
    /// them.
    pub fn help_group(&self, _cx: &App) -> Option<HelpGroup> {
        let keys = self
            .presenter
            .help()
            .into_iter()
            .map(|hint| (hint.action, hint.chord))
            .collect();
        Some(HelpGroup {
            heading: crate::msg::desktop_help_documents_label(),
            keys,
        })
    }
}

/// The Documents module's entry point: its page, on the Documents repository (`documents`), the other
/// domains' reads over their stores (`reads`) and the disk (`files`), which the Shell hosts as the
/// Documents destination. `clock` reads the day the Key Dates are banded against. The page writes
/// the store through its Interactors; the input that wrote announces the Documents, so the rail's
/// Inbox count follows a filing. Its keys, and the Dialog keys its Dialogs' hints show, are
/// `config`'s.
pub fn documents_page(
    documents: &SharedRepository,
    files: SharedFiles,
    reads: lib_database::StoredDocumentReads,
    announcer: Entity<Announcer>,
    config: &lib_config::KeyBindingConfig,
    clock: Clock,
    cx: &mut App,
) -> Entity<DocumentsPage> {
    let keymap = lib_documents::keymap::documents_page_keymap(config);
    let dialog_keys = DialogKeys::of(&lib_shell::dialog_keymap(config));
    let documents = documents.clone();
    let reads = Rc::new(reads);
    let presenter = DocumentsPagePresenter::new(
        documents,
        reads,
        files,
        clock,
        keymap.clone(),
        crate::shell::view::statusline::STUB_LEDGER_FILE.to_string(),
    );
    cx.new(|cx| DocumentsPage::new(presenter, &announcer, keymap, dialog_keys, cx))
}

/// The page's clicks, each one an event sent to its Presenter.
impl PresenterPage<DocumentsPageEvent> for DocumentsPage {
    fn send(&mut self, event: DocumentsPageEvent, cx: &mut Context<'_, Self>) {
        DocumentsPage::send(self, event, cx);
    }
}

impl Render for DocumentsPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let clicks = PageClicks::new(cx);
        let view = self.presenter.view_model();
        let scroll = Scrolls {
            library: self.library_scroll.clone(),
            inbox: self.inbox_scroll.clone(),
            list_focused: false,
        };
        super::render(self.zone_focused, &view, &clicks, scroll, cx)
    }
}
