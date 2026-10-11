//! `ActiveViewEntity` -- the active View as `Shell` forwards to it (ADR-0037). Each method holds
//! the one exhaustive match over the Views and forwards to the inherent method of the same name on
//! the View's Entity, so a new View fails to compile here and nowhere else. There is no `View`
//! trait and no `dyn` erasure (ADR-0016).
//!
//! The Views not yet moved onto this interface sit in [`ShellView`], and `Shell` keeps their
//! handlers until their own ticket moves them. Every `ShellView` arm here does nothing, since
//! `Shell` has already run its own handler for it.

use gpui::{AnyElement, App, Entity, IntoElement, Keystroke};

use super::{bills::state::BillsView, budgets::state::BudgetsView, event::Reveal};
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
    /// A View whose rendering and input are still `Shell`'s.
    Shell(ShellView),
}

/// The Views whose rendering and input are still `Shell`'s: [`ActiveView`] less each View that
/// has moved onto [`ActiveViewEntity`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellView {
    Dashboard,
    Transactions,
    /// The CSV import step, which takes over the Transactions pane while it is open.
    Import,
    Documents,
    Settings(SettingsSection),
    /// A Noun whose surface is a later map's work (#420).
    Placeholder(Noun),
}

impl ActiveViewEntity {
    /// The Entity behind `view`, from the View Entities `Shell` owns.
    pub fn new(view: ActiveView, bills: &Entity<BillsView>, budgets: &Entity<BudgetsView>) -> Self {
        match view {
            ActiveView::Bills => Self::Bills(bills.clone()),
            ActiveView::Budgets => Self::Budgets(budgets.clone()),
            ActiveView::Dashboard => Self::Shell(ShellView::Dashboard),
            ActiveView::Transactions => Self::Shell(ShellView::Transactions),
            ActiveView::Import => Self::Shell(ShellView::Import),
            ActiveView::Documents => Self::Shell(ShellView::Documents),
            ActiveView::Settings(section) => Self::Shell(ShellView::Settings(section)),
            ActiveView::Placeholder(noun) => Self::Shell(ShellView::Placeholder(noun)),
        }
    }

    /// The View as a child element of `Shell`'s main pane. A `ShellView` is drawn by `Shell`.
    pub fn element(&self) -> AnyElement {
        match self {
            Self::Bills(view) => view.clone().into_any_element(),
            Self::Budgets(view) => view.clone().into_any_element(),
            Self::Shell(_) => gpui::Empty.into_any_element(),
        }
    }

    /// A key the View claims before `route_key`. Returns whether it took the key.
    pub fn key_ahead(&self, keystroke: &Keystroke, cx: &mut App) -> bool {
        match self {
            Self::Bills(view) => view.update(cx, |view, cx| view.key_ahead(keystroke, cx)),
            Self::Budgets(view) => view.update(cx, |view, cx| view.key_ahead(keystroke, cx)),
            Self::Shell(_) => false,
        }
    }

    /// A key the router left as `NoOp`. Returns whether the View took it.
    pub fn handle_key(&self, keystroke: &Keystroke, cx: &mut App) -> bool {
        match self {
            Self::Bills(view) => view.update(cx, |view, cx| view.handle_key(keystroke, cx)),
            Self::Budgets(view) => view.update(cx, |view, cx| view.handle_key(keystroke, cx)),
            Self::Shell(_) => false,
        }
    }

    pub fn apply_movement(&self, movement: Movement, cx: &mut App) {
        match self {
            Self::Bills(view) => view.update(cx, |view, cx| view.apply_movement(movement, cx)),
            Self::Budgets(view) => view.update(cx, |view, cx| view.apply_movement(movement, cx)),
            Self::Shell(_) => {}
        }
    }

    /// `Esc`, after the Dialog host's open select. Returns whether the View consumed it.
    pub fn escape(&self, cx: &mut App) -> bool {
        match self {
            Self::Bills(view) => view.update(cx, |view, cx| view.escape(cx)),
            Self::Budgets(view) => view.update(cx, |view, cx| view.escape(cx)),
            Self::Shell(_) => false,
        }
    }

    /// `/`. Returns `false` when the View has no Search, leaving `/` inert.
    pub fn start_search(&self, cx: &mut App) -> bool {
        match self {
            Self::Bills(view) => view.update(cx, |view, cx| view.start_search(cx)),
            Self::Budgets(view) => view.update(cx, |view, cx| view.start_search(cx)),
            Self::Shell(_) => false,
        }
    }

    /// A key typed while the View's Search is open. Returns whether the View took it.
    pub fn search_key(&self, keystroke: &Keystroke, cx: &mut App) -> bool {
        match self {
            Self::Bills(view) => view.update(cx, |view, cx| view.search_key(keystroke, cx)),
            Self::Budgets(view) => view.update(cx, |view, cx| view.search_key(keystroke, cx)),
            Self::Shell(_) => false,
        }
    }

    /// The View has just become the destination.
    pub fn on_enter(&self, cx: &mut App) {
        match self {
            Self::Bills(view) => view.update(cx, |view, cx| view.on_enter(cx)),
            Self::Budgets(view) => view.update(cx, |view, cx| view.on_enter(cx)),
            Self::Shell(_) => {}
        }
    }

    /// Shows a record another View handed off.
    pub fn reveal(&self, reveal: Reveal, cx: &mut App) {
        match self {
            Self::Bills(view) => view.update(cx, |view, cx| view.reveal(reveal, cx)),
            Self::Budgets(view) => view.update(cx, |view, cx| view.reveal(reveal, cx)),
            Self::Shell(_) => {}
        }
    }

    /// The status line's hints and right-hand text for the View's own state.
    pub fn page_status(&self, cx: &App) -> Option<PageStatus> {
        match self {
            Self::Bills(view) => view.read(cx).page_status(cx),
            Self::Budgets(view) => view.read(cx).page_status(cx),
            Self::Shell(_) => None,
        }
    }
}
