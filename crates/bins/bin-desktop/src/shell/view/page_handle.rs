//! [`PageHandle`], the one hosting trait (ADR-0016's and ADR-0056's #1079 update): what a host
//! asks of the page it shows, without knowing which page it is. `Root` reaches the active View
//! through it from one match on the `ActiveView` (`view::active::page`), and the Settings View
//! reaches its open page through it from one match on the `SettingsPage`
//! (`settings::pages::SettingsPages::get`).
//!
//! It is implemented for every [`ModulePage<P>`] and for the Settings View, a composite host that
//! offers each key to its open page through this same trait. Both hosts keep their typed Entities
//! for what one page hands another (a reveal, a month turning, rebound keys): only dispatch is
//! erased.

use std::rc::Rc;

use gpui::{
    AnyElement, AnyView, App, Context, Entity, EventEmitter, Keystroke, Pixels, Subscription,
};

use super::module_page::{ModulePage, PageView};
use super::statusline::PageStatus;
use crate::settings::pages::{PageFrame, PageRowClicked};
use crate::view::event::{Reveal, ViewEvent};
use crate::view::help::HelpGroup;

/// What a page made of a key offered to its module keymap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageKey {
    /// Not the page's to take now: the key goes on to the Shell's next step.
    Declined,
    /// The page took it, and it stops there.
    Taken,
    /// The page took it and hands it on to the Shell's fallback, inside the same input
    /// ([`PageView::passes_on`]).
    PassedOn,
}

/// How a host lays a page out.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PageLayout {
    /// The page fills the body's height and scrolls inside itself (Tracing's log box), rather than
    /// the body scrolling the whole page.
    pub fills_page: bool,
    /// The page draws past the Settings content width to the View's (Units' tables).
    pub full_width: bool,
    /// The narrowest the page lays out at, when it has content that must not squeeze (the
    /// Transactions table); a View area narrower than this scrolls sideways (ADR-0042).
    pub min_width: Option<Pixels>,
    /// A movement the page's keymap didn't take falls back to the Settings frame, which scrolls
    /// the body with it.
    pub frame_scrolls: bool,
}

/// What a host does with one of its page's [`ViewEvent`]s.
type ViewEventSignal = dyn Fn(&ViewEvent, &mut App);

/// Where a hosted page's signals reach its host: its [`ViewEvent`]s, its changes, which repaint
/// the host, and its row clicks. Built over the host's Entity by [`Self::to`], so the trait's
/// `subscribe` names no host type.
#[derive(Clone)]
pub struct PageSignals {
    view_event: Rc<ViewEventSignal>,
    changed: Rc<dyn Fn(&mut App)>,
    row_clicked: Rc<dyn Fn(&mut App)>,
}

impl PageSignals {
    /// Signals reaching the host `cx` is the context of: each [`ViewEvent`] handed to
    /// `view_event`, each row click to `row_clicked`, and each change repainting the host. A host
    /// gone has nothing left to tell.
    pub fn to<H: 'static>(
        cx: &Context<'_, H>,
        view_event: fn(&mut H, &ViewEvent, &mut Context<'_, H>),
        row_clicked: fn(&mut H, &mut Context<'_, H>),
    ) -> Self {
        let host = cx.weak_entity();
        Self {
            view_event: Rc::new({
                let host = host.clone();
                move |event, cx| {
                    let _ = host.update(cx, |host, cx| view_event(host, event, cx));
                }
            }),
            changed: Rc::new({
                let host = host.clone();
                move |cx| {
                    let _ = host.update(cx, |_, cx| cx.notify());
                }
            }),
            row_clicked: Rc::new(move |cx| {
                let _ = host.update(cx, row_clicked);
            }),
        }
    }

    /// Hands `entity`'s [`ViewEvent`]s and changes to the host.
    pub fn follow<T: EventEmitter<ViewEvent>>(
        &self,
        entity: &Entity<T>,
        cx: &mut App,
    ) -> Vec<Subscription> {
        let (view_event, changed) = (self.view_event.clone(), self.changed.clone());
        vec![
            cx.subscribe(entity, move |_, event: &ViewEvent, cx| {
                view_event(event, cx)
            }),
            cx.observe(entity, move |_, cx| changed(cx)),
        ]
    }
}

/// A hosted page behind one `dyn` handle: what a host calls, without knowing which page it shows.
pub trait PageHandle {
    /// A key the page's module keymap takes, by `chord`. Whether it took the key, and whether the
    /// key stops there.
    fn module_key(&self, chord: &str, cx: &mut App) -> PageKey;
    /// A palette Command, by its keymap `command` name. Whether the page took it.
    fn module_command(&self, command: &str, cx: &mut App) -> bool;
    /// A key while the page is in a mode it entered for something of its own (its Search, a
    /// filter popover, an edit in place, Settings' page picker). In Normal mode it takes nothing.
    fn mode_key(&self, keystroke: &Keystroke, cx: &mut App) -> bool;
    /// `Esc`: closes what the page has open. Whether it consumed the key.
    fn escape(&self, cx: &mut App) -> bool;
    /// `/`. `false` when the page has no Search, leaving `/` inert.
    fn start_search(&self, cx: &mut App) -> bool;
    /// The page has just become the destination.
    fn on_enter(&self, cx: &mut App);
    /// Shows a record another View handed off.
    fn reveal(&self, reveal: Reveal, cx: &mut App);
    /// Whether the page has the keyboard, as the host draws it and as focus moves, which can be
    /// every draw.
    fn set_focused(&self, focused: bool, cx: &mut App);
    /// The space the page is drawn in, told outside any input, which can be every draw.
    fn set_frame(&self, frame: PageFrame, cx: &mut App);
    /// The status line's hints and right-hand text for the page's own state; `None` leaves the
    /// Shell's strip and the Ledger file.
    fn page_status(&self, cx: &App) -> Option<PageStatus>;
    /// The page's group of keys for the `?` Help Dialog, `None` for a page that adds none.
    fn help_group(&self, cx: &App) -> Option<HelpGroup>;
    /// A Settings page heading's right-aligned meta; `None` draws the page's scope note.
    fn heading_meta(&self, cx: &App) -> Option<AnyElement>;
    /// A pane drawn at the View's right edge, beside the page.
    fn side_pane(&self, cx: &App) -> Option<AnyElement>;
    /// How the host lays the page out.
    fn layout(&self) -> PageLayout;
    /// The page's Entity, to draw.
    fn view(&self) -> AnyView;
    /// Hands the page's events, changes and row clicks to its host through `signals`. The host
    /// keeps the subscriptions for as long as it shows the page.
    fn subscribe(&self, signals: &PageSignals, cx: &mut App) -> Vec<Subscription>;
}

impl<P: PageView> PageHandle for Entity<ModulePage<P>> {
    /// Every chord the page's keymap binds, the arrows and `g g` among them.
    fn module_key(&self, chord: &str, cx: &mut App) -> PageKey {
        self.update(cx, |page, cx| page.module_key(chord, cx))
    }

    fn module_command(&self, command: &str, cx: &mut App) -> bool {
        self.update(cx, |page, cx| page.module_command(command, cx))
    }

    fn mode_key(&self, keystroke: &Keystroke, cx: &mut App) -> bool {
        self.update(cx, |page, cx| page.mode_key(keystroke, cx))
    }

    /// Closes what the page has open; `false` leaves `Esc` to its host (Settings steps back to the
    /// index).
    fn escape(&self, cx: &mut App) -> bool {
        self.update(cx, |page, cx| page.escape(cx))
    }

    fn start_search(&self, cx: &mut App) -> bool {
        self.update(cx, |page, cx| page.start_search(cx))
    }

    fn on_enter(&self, cx: &mut App) {
        self.update(cx, |page, cx| page.on_enter(cx));
    }

    fn reveal(&self, reveal: Reveal, cx: &mut App) {
        self.update(cx, |page, cx| page.reveal(reveal, cx));
    }

    fn set_focused(&self, focused: bool, cx: &mut App) {
        self.update(cx, |page, cx| page.set_focused(focused, cx));
    }

    /// Handed on as `PageView::framed`'s event.
    fn set_frame(&self, frame: PageFrame, cx: &mut App) {
        self.update(cx, |page, cx| page.framed(frame, cx));
    }

    /// An empty status (the Dashboard words no hints of its own) leaves the Shell's strip.
    fn page_status(&self, cx: &App) -> Option<PageStatus> {
        let status = self.read(cx).page_status();
        (!status.hints.is_empty() || !status.right.is_empty()).then_some(status)
    }

    fn help_group(&self, cx: &App) -> Option<HelpGroup> {
        self.read(cx).help_group()
    }

    fn heading_meta(&self, cx: &App) -> Option<AnyElement> {
        self.read(cx).heading_meta(cx)
    }

    fn side_pane(&self, cx: &App) -> Option<AnyElement> {
        self.read(cx).side_pane(self, cx)
    }

    fn layout(&self) -> PageLayout {
        PageLayout {
            fills_page: P::FILLS_PAGE,
            full_width: P::FULL_WIDTH,
            min_width: P::min_width(),
            frame_scrolls: P::FRAME_SCROLLS,
        }
    }

    fn view(&self) -> AnyView {
        self.clone().into()
    }

    fn subscribe(&self, signals: &PageSignals, cx: &mut App) -> Vec<Subscription> {
        let row_clicked = signals.row_clicked.clone();
        let mut subscriptions = signals.follow(self, cx);
        subscriptions.push(cx.subscribe(self, move |_page, _: &PageRowClicked, cx| {
            row_clicked(cx);
        }));
        subscriptions
    }
}
