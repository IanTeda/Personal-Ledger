//! `ActiveViewEntity` -- the active View as `Root` forwards to it (ADR-0037). Each method holds
//! the one exhaustive match over the Views and forwards to the inherent method of the same name on
//! the View's Entity, so a new View fails to compile here and nowhere else. There is no `View`
//! trait and no `dyn` erasure (ADR-0016).
//!
//! The Views not yet moved onto this interface sit in [`ShellView`], and `Root` keeps their
//! handlers until their own ticket moves them. Every `ShellView` arm here does nothing, since
//! `Root` has already run its own handler for it.

use gpui::{AnyElement, App, Entity, IntoElement, Keystroke};

use super::{
    budgets::state::BudgetsView, dashboard::state::DashboardView, documents::state::DocumentsView,
    event::Reveal,
};
use crate::bills::BillsPage;
use crate::settings::state::SettingsView;
use crate::transaction_import::ImportPage;
use crate::transactions::TransactionsPage;
use crate::{
    navigation::{active_view::ActiveView, movement::Movement, nav::Noun},
    shell::view::statusline::PageStatus,
    view::help::HelpGroup,
};

/// The active View, holding its typed Entity once it renders and handles its own input.
#[derive(Clone)]
pub enum ActiveViewEntity {
    Dashboard(Entity<DashboardView>),
    Bills(Entity<BillsPage>),
    Budgets(Entity<BudgetsView>),
    /// The Transactions list.
    Transactions(Entity<TransactionsPage>),
    /// The Transactions destination's Import step, shown in place of the list while it is open: the
    /// Transaction Import module's View.
    Import(Entity<ImportPage>),
    Documents(Entity<DocumentsView>),
    /// Settings: the index and every page. A page with its own Entity (Accounts, Payees) is
    /// forwarded to by the View; `Root` still draws the other pages with Ledger data and handles
    /// their keys first (`Root`) until each moves into the View.
    Settings(Entity<SettingsView>),
    /// A View whose rendering and input are still `Root`'s.
    Shell(ShellView),
}

/// The Views whose rendering and input are still `Root`'s: [`ActiveView`] less each View that
/// has moved onto [`ActiveViewEntity`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellView {
    /// A Noun whose surface is a later map's work (#420).
    Placeholder(Noun),
}

/// The View Entities `Root` owns, one per View that renders and handles its own input.
pub struct ViewEntities {
    pub dashboard: Entity<DashboardView>,
    pub bills: Entity<BillsPage>,
    pub budgets: Entity<BudgetsView>,
    pub transactions: Entity<TransactionsPage>,
    pub import: Entity<ImportPage>,
    pub documents: Entity<DocumentsView>,
    pub settings: Entity<SettingsView>,
}

impl ActiveViewEntity {
    /// The Entity behind `view`, from the View Entities `Root` owns.
    pub fn new(view: ActiveView, views: &ViewEntities) -> Self {
        match view {
            ActiveView::Bills => Self::Bills(views.bills.clone()),
            ActiveView::Budgets => Self::Budgets(views.budgets.clone()),
            ActiveView::Transactions => Self::Transactions(views.transactions.clone()),
            ActiveView::Import => Self::Import(views.import.clone()),
            ActiveView::Dashboard => Self::Dashboard(views.dashboard.clone()),
            ActiveView::Documents => Self::Documents(views.documents.clone()),
            ActiveView::Settings(_) => Self::Settings(views.settings.clone()),
            ActiveView::Placeholder(noun) => Self::Shell(ShellView::Placeholder(noun)),
        }
    }

    /// The View as a child element of `Root`'s main pane, told whether the View zone has focus
    /// for the focus rings it draws within itself. A `ShellView` is drawn by `Root`.
    pub fn element(
        &self,
        zone_focused: bool,
        ledger_open: bool,
        window_size: crate::shell::view::window_size::WindowSize,
        cx: &mut App,
    ) -> AnyElement {
        match self {
            Self::Dashboard(view) => {
                view.update(cx, |view, _| view.set_ledger_open(ledger_open));
                view.clone().into_any_element()
            }
            Self::Documents(view) => {
                view.update(cx, |view, _| view.set_zone_focused(zone_focused));
                view.clone().into_any_element()
            }
            Self::Settings(view) => {
                view.update(cx, |view, cx| {
                    view.set_zone_focused(zone_focused);
                    view.set_window_size(window_size, cx);
                });
                view.clone().into_any_element()
            }
            Self::Bills(view) => view.clone().into_any_element(),
            Self::Budgets(view) => view.clone().into_any_element(),
            Self::Transactions(view) => view.clone().into_any_element(),
            Self::Import(view) => view.clone().into_any_element(),
            Self::Shell(_) => gpui::Empty.into_any_element(),
        }
    }

    /// A key the focused module's keymap takes, by `chord`: the Bills or Transactions page's, the
    /// Import step's, or a Settings page that is a module's View. Returns whether it took the key.
    pub fn module_key(&self, chord: &str, cx: &mut App) -> bool {
        match self {
            Self::Bills(view) => view.update(cx, |view, cx| view.module_key(chord, cx)),
            Self::Transactions(view) => view.update(cx, |view, cx| view.module_key(chord, cx)),
            Self::Settings(view) => view.update(cx, |view, cx| view.module_key(chord, cx)),
            Self::Import(view) => view.update(cx, |view, cx| view.module_key(chord, cx)),
            Self::Dashboard(_) | Self::Budgets(_) | Self::Documents(_) | Self::Shell(_) => false,
        }
    }

    /// A key the View claims before the Shell's fallback. Returns whether it took the key.
    pub fn key_ahead(&self, keystroke: &Keystroke, cx: &mut App) -> bool {
        match self {
            Self::Dashboard(view) => view.update(cx, |view, cx| view.key_ahead(keystroke, cx)),
            Self::Bills(view) => view.update(cx, |view, cx| view.key_ahead(keystroke, cx)),
            Self::Budgets(view) => view.update(cx, |view, cx| view.key_ahead(keystroke, cx)),
            Self::Transactions(view) => view.update(cx, |view, cx| view.key_ahead(keystroke, cx)),
            Self::Documents(view) => view.update(cx, |view, cx| view.key_ahead(keystroke, cx)),
            Self::Settings(view) => view.update(cx, |view, cx| view.key_ahead(keystroke, cx)),
            // Every step key is its keymap's.
            Self::Import(_) | Self::Shell(_) => false,
        }
    }

    /// A key no other step took. Returns whether the View took it.
    pub fn handle_key(&self, keystroke: &Keystroke, cx: &mut App) -> bool {
        match self {
            Self::Dashboard(view) => view.update(cx, |view, cx| view.handle_key(keystroke, cx)),
            Self::Bills(view) => view.update(cx, |view, cx| view.handle_key(keystroke, cx)),
            Self::Budgets(view) => view.update(cx, |view, cx| view.handle_key(keystroke, cx)),
            Self::Transactions(view) => view.update(cx, |view, cx| view.handle_key(keystroke, cx)),
            Self::Documents(view) => view.update(cx, |view, cx| view.handle_key(keystroke, cx)),
            Self::Settings(view) => view.update(cx, |view, cx| view.handle_key(keystroke, cx)),
            // Every step key is its keymap's.
            Self::Import(_) | Self::Shell(_) => false,
        }
    }

    pub fn apply_movement(&self, movement: Movement, cx: &mut App) {
        match self {
            Self::Dashboard(view) => view.update(cx, |view, cx| view.apply_movement(movement, cx)),
            Self::Bills(view) => view.update(cx, |view, cx| view.apply_movement(movement, cx)),
            Self::Budgets(view) => view.update(cx, |view, cx| view.apply_movement(movement, cx)),
            Self::Transactions(view) => {
                view.update(cx, |view, cx| view.apply_movement(movement, cx))
            }
            Self::Documents(view) => view.update(cx, |view, cx| view.apply_movement(movement, cx)),
            Self::Settings(view) => view.update(cx, |view, cx| view.apply_movement(movement, cx)),
            // The step's movements are its keymap's.
            Self::Import(_) | Self::Shell(_) => {}
        }
    }

    /// `Esc`, after the Dialog host's open select. Returns whether the View consumed it.
    pub fn escape(&self, cx: &mut App) -> bool {
        match self {
            Self::Dashboard(view) => view.update(cx, |view, cx| view.escape(cx)),
            Self::Bills(view) => view.update(cx, |view, cx| view.escape(cx)),
            Self::Budgets(view) => view.update(cx, |view, cx| view.escape(cx)),
            Self::Transactions(view) => view.update(cx, |view, cx| view.escape(cx)),
            Self::Import(view) => view.update(cx, |view, cx| view.escape(cx)),
            Self::Documents(view) => view.update(cx, |view, cx| view.escape(cx)),
            Self::Settings(view) => view.update(cx, |view, cx| view.escape(cx)),
            Self::Shell(_) => false,
        }
    }

    /// `/`. Returns `false` when the View has no Search, leaving `/` inert.
    pub fn start_search(&self, cx: &mut App) -> bool {
        match self {
            Self::Dashboard(view) => view.update(cx, |view, cx| view.start_search(cx)),
            Self::Bills(view) => view.update(cx, |view, cx| view.start_search(cx)),
            Self::Budgets(view) => view.update(cx, |view, cx| view.start_search(cx)),
            Self::Transactions(view) => view.update(cx, |view, cx| view.start_search(cx)),
            Self::Documents(view) => view.update(cx, |view, cx| view.start_search(cx)),
            Self::Settings(view) => view.update(cx, |view, cx| view.start_search(cx)),
            Self::Import(_) | Self::Shell(_) => false,
        }
    }

    /// A key typed while the View's Search is open. Returns whether the View took it.
    pub fn search_key(&self, keystroke: &Keystroke, cx: &mut App) -> bool {
        match self {
            Self::Dashboard(view) => view.update(cx, |view, cx| view.search_key(keystroke, cx)),
            Self::Bills(view) => view.update(cx, |view, cx| view.search_key(keystroke, cx)),
            Self::Budgets(view) => view.update(cx, |view, cx| view.search_key(keystroke, cx)),
            Self::Transactions(view) => view.update(cx, |view, cx| view.search_key(keystroke, cx)),
            Self::Documents(view) => view.update(cx, |view, cx| view.search_key(keystroke, cx)),
            Self::Settings(view) => view.update(cx, |view, cx| view.search_key(keystroke, cx)),
            Self::Import(_) | Self::Shell(_) => false,
        }
    }

    /// The View has just become the destination.
    pub fn on_enter(&self, cx: &mut App) {
        match self {
            Self::Dashboard(view) => view.update(cx, |view, cx| view.on_enter(cx)),
            Self::Bills(view) => view.update(cx, |view, cx| view.on_enter(cx)),
            Self::Budgets(view) => view.update(cx, |view, cx| view.on_enter(cx)),
            Self::Transactions(view) => view.update(cx, |view, cx| view.on_enter(cx)),
            Self::Import(view) => view.update(cx, |view, cx| view.on_enter(cx)),
            Self::Documents(view) => view.update(cx, |view, cx| view.on_enter(cx)),
            Self::Settings(view) => view.update(cx, |view, cx| view.on_enter(cx)),
            Self::Shell(_) => {}
        }
    }

    /// Shows a record another View handed off.
    pub fn reveal(&self, reveal: Reveal, cx: &mut App) {
        match self {
            Self::Dashboard(view) => view.update(cx, |view, cx| view.reveal(reveal, cx)),
            Self::Bills(view) => view.update(cx, |view, cx| view.reveal(reveal, cx)),
            Self::Budgets(view) => view.update(cx, |view, cx| view.reveal(reveal, cx)),
            Self::Documents(view) => view.update(cx, |view, cx| view.reveal(reveal, cx)),
            Self::Settings(view) => view.update(cx, |view, cx| view.reveal(reveal, cx)),
            // Transactions is reached through its entry point, not a `Reveal`. `Root` makes
            // Transactions the destination first, which closes the step.
            Self::Transactions(_) | Self::Import(_) | Self::Shell(_) => {}
        }
    }

    /// The narrowest the View lays out at, when it has content that must not squeeze (the
    /// Transactions table, whose width the Import step keeps). A View area narrower than this
    /// scrolls sideways (ADR-0042).
    pub fn min_width(&self) -> Option<gpui::Pixels> {
        match self {
            Self::Transactions(_) | Self::Import(_) => {
                Some(crate::transactions::view::table::min_width())
            }
            _ => None,
        }
    }

    /// The status line's hints and right-hand text for the View's own state.
    pub fn page_status(&self, cx: &App) -> Option<PageStatus> {
        match self {
            Self::Dashboard(view) => view.read(cx).page_status(cx),
            Self::Bills(view) => view.read(cx).page_status(cx),
            Self::Budgets(view) => view.read(cx).page_status(cx),
            Self::Transactions(view) => view.read(cx).page_status(cx),
            Self::Import(view) => view.read(cx).page_status(cx),
            Self::Documents(view) => view.read(cx).page_status(cx),
            Self::Settings(view) => view.read(cx).page_status(cx),
            Self::Shell(_) => None,
        }
    }

    /// The View's own group of keys for the `?` Help Dialog, `None` for a View that adds none
    /// to the global shortcuts.
    pub fn help_group(&self, cx: &App) -> Option<HelpGroup> {
        match self {
            Self::Bills(view) => view.read(cx).help_group(cx),
            Self::Transactions(view) => view.read(cx).help_group(cx),
            Self::Documents(view) => view.read(cx).help_group(cx),
            Self::Settings(view) => view.read(cx).help_group(cx),
            Self::Dashboard(_) | Self::Budgets(_) | Self::Import(_) | Self::Shell(_) => None,
        }
    }
}
