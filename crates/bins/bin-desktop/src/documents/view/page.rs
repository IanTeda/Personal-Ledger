//! The Documents page's View: `lib-documents`' page Presenter's [`PageView`], which the generic
//! `ModulePage` hosts as the Documents destination (ADR-0056,
//! `docs/ux/desktop-mockups/04-documents/`). The keymaps (the Library's, the Inbox's and the
//! Search's), the Search as the page's mode, the mode a palette Command shows first and every event
//! are the Presenter's; it draws the view model (`super::render`). Its Dialogs open as
//! `ModuleDialog`s in the Dialog host (`super::dialog`), each landing its outcome back on the page.
//!
//! What stays here is gpui's: the Library's and the Inbox's scroll, which [`PageView::after`]
//! sends back to the top as the mode changes and otherwise onto the selection.
//!
//! The rows name records from every domain a Link reaches, and Settings › Documents writes the
//! Document Types, so the page repaints on a change to any of them.

use gpui::{AnyElement, Context, ScrollHandle, ScrollStrategy, UniformListScrollHandle, Window};
use lib_database::Domain;
use lib_documents::DocumentsMode;
use lib_documents::presenter::{
    DocumentsPageEvent, DocumentsPagePresenter, DocumentsPageViewModel,
};

use super::Scrolls;
use crate::documents::router::DocumentsPageRouter;
use crate::shell::view::module_page::{ModulePage, PageCue, PageDraw, PageView};
use crate::shell::view::statusline::PageStatus;
use crate::view::event::ViewEvent;
use crate::view::help::HelpGroup;

/// The Documents page: its Presenter in the generic page.
pub type DocumentsPage = ModulePage<DocumentsPagePresenter>;

/// The Library's and the Inbox's scroll.
#[derive(Default)]
pub struct DocumentsWidgets {
    library_scroll: UniformListScrollHandle,
    inbox_scroll: ScrollHandle,
}

/// The mode and the selected row before a handle, which the scroll follows from.
#[derive(Default)]
pub struct DocumentsBefore {
    mode: DocumentsMode,
    selected: usize,
}

impl PageView for DocumentsPagePresenter {
    /// The Dialog keymap's chords, which the page's Dialogs' hints show.
    type Setup = crate::dialog_keys::LiveDialogKeys;
    type Widgets = DocumentsWidgets;
    type Before = DocumentsBefore;

    const REPAINTS: &'static [Domain] = &[
        Domain::Accounts,
        Domain::Bills,
        Domain::DocumentTypes,
        Domain::Documents,
        Domain::Inventory,
        Domain::Payees,
        Domain::Transactions,
    ];

    /// The Router opens the Dialogs on the ports the Presenter holds, lent before the handle
    /// borrows it.
    fn with_router<R>(
        presenter: &mut Self,
        keys: &crate::dialog_keys::LiveDialogKeys,
        cx: &mut Context<'_, DocumentsPage>,
        handle: impl FnOnce(&mut Self, &mut Self::Router<'_>) -> R,
    ) -> R {
        let ports = presenter.dialog_ports();
        handle(
            presenter,
            &mut DocumentsPageRouter::lending(keys, cx, ports),
        )
    }

    fn before(
        &mut self,
        _event: &DocumentsPageEvent,
        _widgets: &DocumentsWidgets,
    ) -> DocumentsBefore {
        DocumentsBefore {
            mode: DocumentsPagePresenter::mode(self),
            selected: self.selected(),
        }
    }

    /// Back to the top as the mode changes; otherwise the selected row scrolled into view, landing
    /// at the edge it came from, as on Transactions.
    fn after(
        &self,
        before: DocumentsBefore,
        _setup: &Self::Setup,
        widgets: &mut DocumentsWidgets,
    ) -> Option<ViewEvent> {
        let (mode, at) = (DocumentsPagePresenter::mode(self), self.selected());
        if mode != before.mode {
            widgets
                .library_scroll
                .scroll_to_item(0, ScrollStrategy::Top);
            widgets.inbox_scroll.set_offset(gpui::Point::default());
            return None;
        }
        match mode {
            DocumentsMode::Library => {
                let strategy = if at < before.selected {
                    ScrollStrategy::Top
                } else {
                    ScrollStrategy::Bottom
                };
                widgets.library_scroll.scroll_to_item(at, strategy);
            }
            DocumentsMode::Inbox => widgets.inbox_scroll.scroll_to_item(at),
        }
        None
    }

    /// `g f` and the palette land on the list. A Search left open does not come back with it.
    fn cued(cue: PageCue) -> Option<DocumentsPageEvent> {
        matches!(cue, PageCue::Entered).then_some(DocumentsPageEvent::Entered)
    }

    /// The `?` cheat-sheet's Documents group: the index keys, then the mode's, as the keymap binds
    /// them.
    fn help(view: &DocumentsPageViewModel) -> Option<HelpGroup> {
        Some(HelpGroup {
            heading: crate::msg::desktop_help_documents_label(),
            keys: view
                .help
                .iter()
                .map(|hint| (hint.action.clone(), hint.chord.clone()))
                .collect(),
        })
    }

    fn render(
        view: &DocumentsPageViewModel,
        page: PageDraw<'_, Self>,
        _window: &mut Window,
        cx: &mut Context<'_, DocumentsPage>,
    ) -> AnyElement {
        let scroll = Scrolls {
            library: page.widgets.library_scroll.clone(),
            inbox: page.widgets.inbox_scroll.clone(),
            list_focused: false,
        };
        super::render(page.focused, view, page.clicks, scroll, cx)
    }

    /// The status line's legend for the zone and mode, and the stored size.
    fn status(view: &DocumentsPageViewModel) -> PageStatus {
        PageStatus {
            hints: view.hints.clone(),
            right: view.stored.clone().into(),
        }
    }

    /// `Esc`, innermost first: the Search (its query cleared), then a kept query, then the list
    /// back to the index.
    fn escape(view: &DocumentsPageViewModel) -> Option<DocumentsPageEvent> {
        view.takes_back.then_some(DocumentsPageEvent::Back)
    }
}
