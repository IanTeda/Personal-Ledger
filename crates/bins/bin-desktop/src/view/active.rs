//! `ActiveViewEntity` -- the active View as `Shell` forwards to it (ADR-0037). Each method holds
//! the one exhaustive match over the Views and forwards to the inherent method of the same name on
//! the View's Entity, so a new View fails to compile here and nowhere else. There is no `View`
//! trait and no `dyn` erasure (ADR-0016).
//!
//! The Views not yet moved onto this interface sit in [`ShellView`], and `Shell` keeps their
//! handlers until their own ticket moves them. Every `ShellView` arm here does nothing, since
//! `Shell` has already run its own handler for it.

use gpui::{AnyElement, App, Entity, IntoElement, Keystroke};

use super::{
    bills::state::BillsView, budgets::state::BudgetsView, documents::state::DocumentsView,
    event::Reveal, transactions::state::TransactionsView,
};
use crate::{
    chrome::statusline::PageStatus,
    navigation::{active_view::ActiveView, key_router::Movement, nav::Noun},
    settings::SettingsSection,
};

/// The active View, holding its typed Entity once it renders and handles its own input.
#[derive(Clone)]
pub enum ActiveViewEntity {
    Bills(Entity<BillsView>),
    Budgets(Entity<BudgetsView>),
    /// The Transactions page, and the Import step that takes it over while it is open.
    Transactions(Entity<TransactionsView>),
    Documents(Entity<DocumentsView>),
    /// A View whose rendering and input are still `Shell`'s.
    Shell(ShellView),
}

/// The Views whose rendering and input are still `Shell`'s: [`ActiveView`] less each View that
/// has moved onto [`ActiveViewEntity`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellView {
    Dashboard,
    Settings(SettingsSection),
    /// A Noun whose surface is a later map's work (#420).
    Placeholder(Noun),
}

/// The View Entities `Shell` owns, one per View that renders and handles its own input.
pub struct ViewEntities {
    pub bills: Entity<BillsView>,
    pub budgets: Entity<BudgetsView>,
    pub transactions: Entity<TransactionsView>,
    pub documents: Entity<DocumentsView>,
}

impl ActiveViewEntity {
    /// The Entity behind `view`, from the View Entities `Shell` owns.
    pub fn new(view: ActiveView, views: &ViewEntities) -> Self {
        match view {
            ActiveView::Bills => Self::Bills(views.bills.clone()),
            ActiveView::Budgets => Self::Budgets(views.budgets.clone()),
            ActiveView::Transactions | ActiveView::Import => {
                Self::Transactions(views.transactions.clone())
            }
            ActiveView::Dashboard => Self::Shell(ShellView::Dashboard),
            ActiveView::Documents => Self::Documents(views.documents.clone()),
            ActiveView::Settings(section) => Self::Shell(ShellView::Settings(section)),
            ActiveView::Placeholder(noun) => Self::Shell(ShellView::Placeholder(noun)),
        }
    }

    /// The View as a child element of `Shell`'s main pane, told whether the View zone has focus
    /// for the focus rings it draws within itself. A `ShellView` is drawn by `Shell`.
    pub fn element(&self, zone_focused: bool, cx: &mut App) -> AnyElement {
        match self {
            Self::Documents(view) => {
                view.update(cx, |view, _| view.set_zone_focused(zone_focused));
                view.clone().into_any_element()
            }
            Self::Bills(view) => view.clone().into_any_element(),
            Self::Budgets(view) => view.clone().into_any_element(),
            Self::Transactions(view) => view.clone().into_any_element(),
            Self::Shell(_) => gpui::Empty.into_any_element(),
        }
    }

    /// A key the View claims before `route_key`. Returns whether it took the key.
    pub fn key_ahead(&self, keystroke: &Keystroke, cx: &mut App) -> bool {
        match self {
            Self::Bills(view) => view.update(cx, |view, cx| view.key_ahead(keystroke, cx)),
            Self::Budgets(view) => view.update(cx, |view, cx| view.key_ahead(keystroke, cx)),
            Self::Transactions(view) => view.update(cx, |view, cx| view.key_ahead(keystroke, cx)),
            Self::Documents(view) => view.update(cx, |view, cx| view.key_ahead(keystroke, cx)),
            Self::Shell(_) => false,
        }
    }

    /// A key the router left as `NoOp`. Returns whether the View took it.
    pub fn handle_key(&self, keystroke: &Keystroke, cx: &mut App) -> bool {
        match self {
            Self::Bills(view) => view.update(cx, |view, cx| view.handle_key(keystroke, cx)),
            Self::Budgets(view) => view.update(cx, |view, cx| view.handle_key(keystroke, cx)),
            Self::Transactions(view) => view.update(cx, |view, cx| view.handle_key(keystroke, cx)),
            Self::Documents(view) => view.update(cx, |view, cx| view.handle_key(keystroke, cx)),
            Self::Shell(_) => false,
        }
    }

    pub fn apply_movement(&self, movement: Movement, cx: &mut App) {
        match self {
            Self::Bills(view) => view.update(cx, |view, cx| view.apply_movement(movement, cx)),
            Self::Budgets(view) => view.update(cx, |view, cx| view.apply_movement(movement, cx)),
            Self::Transactions(view) => {
                view.update(cx, |view, cx| view.apply_movement(movement, cx))
            }
            Self::Documents(view) => view.update(cx, |view, cx| view.apply_movement(movement, cx)),
            Self::Shell(_) => {}
        }
    }

    /// `Esc`, after the Dialog host's open select. Returns whether the View consumed it.
    pub fn escape(&self, cx: &mut App) -> bool {
        match self {
            Self::Bills(view) => view.update(cx, |view, cx| view.escape(cx)),
            Self::Budgets(view) => view.update(cx, |view, cx| view.escape(cx)),
            Self::Transactions(view) => view.update(cx, |view, cx| view.escape(cx)),
            Self::Documents(view) => view.update(cx, |view, cx| view.escape(cx)),
            Self::Shell(_) => false,
        }
    }

    /// `/`. Returns `false` when the View has no Search, leaving `/` inert.
    pub fn start_search(&self, cx: &mut App) -> bool {
        match self {
            Self::Bills(view) => view.update(cx, |view, cx| view.start_search(cx)),
            Self::Budgets(view) => view.update(cx, |view, cx| view.start_search(cx)),
            Self::Transactions(view) => view.update(cx, |view, cx| view.start_search(cx)),
            Self::Documents(view) => view.update(cx, |view, cx| view.start_search(cx)),
            Self::Shell(_) => false,
        }
    }

    /// A key typed while the View's Search is open. Returns whether the View took it.
    pub fn search_key(&self, keystroke: &Keystroke, cx: &mut App) -> bool {
        match self {
            Self::Bills(view) => view.update(cx, |view, cx| view.search_key(keystroke, cx)),
            Self::Budgets(view) => view.update(cx, |view, cx| view.search_key(keystroke, cx)),
            Self::Transactions(view) => view.update(cx, |view, cx| view.search_key(keystroke, cx)),
            Self::Documents(view) => view.update(cx, |view, cx| view.search_key(keystroke, cx)),
            Self::Shell(_) => false,
        }
    }

    /// The View has just become the destination.
    pub fn on_enter(&self, cx: &mut App) {
        match self {
            Self::Bills(view) => view.update(cx, |view, cx| view.on_enter(cx)),
            Self::Budgets(view) => view.update(cx, |view, cx| view.on_enter(cx)),
            Self::Transactions(view) => view.update(cx, |view, cx| view.on_enter(cx)),
            Self::Documents(view) => view.update(cx, |view, cx| view.on_enter(cx)),
            Self::Shell(_) => {}
        }
    }

    /// Shows a record another View handed off.
    pub fn reveal(&self, reveal: Reveal, cx: &mut App) {
        match self {
            Self::Bills(view) => view.update(cx, |view, cx| view.reveal(reveal, cx)),
            Self::Budgets(view) => view.update(cx, |view, cx| view.reveal(reveal, cx)),
            Self::Transactions(view) => view.update(cx, |view, cx| view.reveal(reveal, cx)),
            Self::Documents(view) => view.update(cx, |view, cx| view.reveal(reveal, cx)),
            Self::Shell(_) => {}
        }
    }

    /// The status line's hints and right-hand text for the View's own state.
    pub fn page_status(&self, cx: &App) -> Option<PageStatus> {
        match self {
            Self::Bills(view) => view.read(cx).page_status(cx),
            Self::Budgets(view) => view.read(cx).page_status(cx),
            Self::Transactions(view) => view.read(cx).page_status(cx),
            Self::Documents(view) => view.read(cx).page_status(cx),
            Self::Shell(_) => None,
        }
    }
}
