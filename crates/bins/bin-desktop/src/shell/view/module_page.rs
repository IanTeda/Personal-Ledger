//! The one page Entity every module page Presenter is hosted in (ADR-0056): [`ModulePage<P>`]
//! owns the Presenter, the widget state its View keeps beside it, whether the page has focus and
//! its repaints, and is the one take, send and handle path a key, a palette Command, a click or a
//! landed Dialog reaches the Presenter through. A domain writes a [`PageView`] on its page
//! Presenter (its Router, what it repaints on, how it draws and lays out) and no Entity of its own.
//!
//! The page never counts or announces a write: each input's `send` announces what its commit
//! names (ADR-0055), and the page repaints on the domains in [`PageView::REPAINTS`].
//!
//! It takes no leftover-key handler: a page types text through `PagePresenter::typed` and reports a
//! mode of its own through `PagePresenter::mode`, which the page turns into `EnterMode` and
//! `ExitMode` around each handle, and the key route offers it every key in that mode through
//! [`ModulePage::mode_key`] (#1100). Main Views and Settings pages alike are reached through
//! [`PageHandle`](super::page_handle::PageHandle) over their Entity (#1101), with [`PageView`]'s
//! hooks around a handle for the widget state a page keeps (ADR-0056's main-View consequence).
//! [`PresenterPage`] is how [`PageClicks`] reaches the page, for the generic page as for the
//! hand-written ones. A Settings page is a [`SettingsPageView`] too, naming its place in the index.

use gpui::{AnyElement, App, Context, Entity, EventEmitter, Keystroke, Pixels, Point};
use gpui::{Render, Subscription, Window};
use lib_database::Domain;
use lib_viper::keymap::SEARCH;
use lib_viper::presenter::{PageMode, PagePresenter, TextKey};

use super::page_clicks::{PageClicks, PresenterPage};
use super::page_handle::PageKey;
use super::statusline::PageStatus;
use crate::announcer::{Announcer, LedgerChanged, notify_on};
use crate::navigation::chord::chord;
use crate::settings::SettingsPage;
use crate::settings::pages::{PageFrame, PageRowClicked};
use crate::view::event::{Reveal, ViewEvent, ViewMode};
use crate::view::help::HelpGroup;

/// The bin's half of a page Presenter: the Router it is lent for each call, the domains it
/// repaints on, and how it draws. Implemented on the Presenter type itself.
pub trait PageView: PagePresenter + Sized + 'static {
    /// What the page's Router is built from for each call: the ports its Dialogs open on and
    /// anything else it routes with (the Clock, the Dialog keys). The page holds it beside the
    /// Presenter.
    type Setup: 'static;

    /// The widget state that is gpui's to keep beside the Presenter, which the Presenter never
    /// sees: a scroll handle, a row menu's position. `()` for a page with none. It is reset as the
    /// page becomes the destination.
    type Widgets: Default + 'static;

    /// What [`Self::before`] notes ahead of a handle for [`Self::after`]: `()` for a page that
    /// keeps nothing across one.
    type Before: Default;

    /// The Ledger domains whose change repaints the page: its own, and any its view model reads.
    const REPAINTS: &'static [Domain];

    /// Whether a Settings page fills the body's height and scrolls inside itself (Tracing's log
    /// box) rather than the body scrolling the whole page.
    const FILLS_PAGE: bool = false;

    /// Whether a Settings page draws past the Settings content width to the View's (Units' tables,
    /// down to their 610px minimum).
    const FULL_WIDTH: bool = false;

    /// Whether a movement the page's keymap didn't take falls back to the Settings frame, which
    /// scrolls the body with it. A page with a selection of its own binds every movement it takes,
    /// so leaves this `false` and a movement it declines does nothing; General, Ledger and the info
    /// pages have none.
    const FRAME_SCROLLS: bool = false;

    /// The narrowest the page lays out at, when it has content that must not squeeze (the
    /// Transactions table, whose width the Import step keeps). A View area narrower than this
    /// scrolls sideways (ADR-0042).
    fn min_width() -> Option<Pixels> {
        None
    }

    /// Runs `handle` on `presenter` with the page's Router, built over `setup` around this call's
    /// context, which is how a Router opens a Dialog in the host. The Router may read what the
    /// Presenter holds for the Dialogs it opens (the date style) before `handle` runs.
    fn with_router<R>(
        presenter: &mut Self,
        setup: &Self::Setup,
        cx: &mut Context<'_, ModulePage<Self>>,
        handle: impl FnOnce(&mut Self, &mut Self::Router<'_>) -> R,
    ) -> R;

    /// Notes what [`Self::after`] compares against, ahead of `event`'s handle: the tab shown, the
    /// selection. It may hand the Presenter widget facts (a half page).
    fn before(&mut self, _event: &Self::Event, _widgets: &Self::Widgets) -> Self::Before {
        Self::Before::default()
    }

    /// Follows a handle in the widget state (a scroll back to the top, onto the selection), and
    /// the View event it asks the Shell for (a status reset), from what [`Self::before`] noted and
    /// what the Router left in `setup` for it (Tracing's list effects).
    fn after(
        &self,
        _before: Self::Before,
        _setup: &Self::Setup,
        _widgets: &mut Self::Widgets,
    ) -> Option<ViewEvent> {
        None
    }

    /// What the page follows besides the Ledger, built over `setup` as the page is: a source that
    /// signals outside any input (Tracing's log capture), each signal sent as the page's own event
    /// through [`ModulePage::send`]. The page keeps the subscriptions for its life. None by default.
    fn subscribe(
        _setup: &Self::Setup,
        _cx: &mut Context<'_, ModulePage<Self>>,
    ) -> Vec<Subscription> {
        Vec::new()
    }

    /// The event the page is sent as it becomes the destination, after its widget state is reset.
    fn entered() -> Option<Self::Event> {
        None
    }

    /// The event giving the page the space it is drawn in (Display's grid columns), sent as the
    /// Settings View hands it its frame, which can be every draw. It is layout, not an input, so
    /// nothing commits; the Presenter declines a value it already has, so the page redraws only on
    /// a change. `None` for a page the frame changes nothing on.
    fn framed(_frame: PageFrame) -> Option<Self::Event> {
        None
    }

    /// Whether a key that sent `event` goes on to the Shell after the page handled it, which then
    /// does what the key does there: the Colour Theme grid's focus `tab` and half-page moves leave
    /// the grid, then move the focus or scroll. `false` for every event of a page that keeps the
    /// keys it takes.
    fn passes_on(_event: &Self::Event) -> bool {
        false
    }

    /// The event the page is sent when a domain in [`Self::REPAINTS`] changes, ahead of its
    /// repaint, for a Presenter that keeps what it read until told; `None` only repaints.
    fn ledger_changed() -> Option<Self::Event> {
        None
    }

    /// The page's group of keys for the `?` Help Dialog, `None` for a page that adds none.
    fn help(_view: &Self::ViewModel) -> Option<HelpGroup> {
        None
    }

    /// Draws the page from `view`.
    fn render(
        view: &Self::ViewModel,
        page: PageDraw<'_, Self>,
        window: &mut Window,
        cx: &mut Context<'_, ModulePage<Self>>,
    ) -> AnyElement;

    /// The status line's hints, rebound keys included, and its right-hand text, both from the
    /// view model. The `?` cheat sheet reads the same hints.
    fn status(view: &Self::ViewModel) -> PageStatus;

    /// A Settings page heading's right-aligned meta; `None` draws the page's scope note.
    fn heading_meta(_view: &Self::ViewModel, _cx: &App) -> Option<AnyElement> {
        None
    }

    /// A pane drawn at the View's right edge, beside the page.
    fn side_pane(
        _view: &Self::ViewModel,
        _clicks: &PageClicks<ModulePage<Self>>,
        _cx: &App,
    ) -> Option<AnyElement> {
        None
    }

    /// The event showing a record another View handed off, `None` for a record the page doesn't
    /// show.
    fn reveal(_reveal: Reveal) -> Option<Self::Event> {
        None
    }

    /// The event `Esc` sends to close what the page has open (a row menu), `None` while it has
    /// nothing open, which leaves `Esc` to Settings: it steps back to the index.
    fn escape(_view: &Self::ViewModel) -> Option<Self::Event> {
        None
    }

    /// Told whether the page, not Settings' index, has the keyboard, as the View draws it and as
    /// focus moves, which can be every draw. Returns whether the Presenter's view model changed, so
    /// the page redraws only then (General puts the keyboard on its first control).
    fn focus_changed(&mut self, _focused: bool) -> bool {
        false
    }

    /// Records where the pointer opened a row's menu, before its event is sent. The position is
    /// the View's to draw the menu at; the Presenter never sees it.
    fn menu_at(_widgets: &mut Self::Widgets, _at: Point<Pixels>) {}
}

/// A Settings page's [`PageView`]: its place in the index, which the Settings View finds the page
/// by. Its identity (order, id, label, scope note) stays on [`SettingsPage`], which the page names
/// through [`Self::PAGE`]; how the body lays it out is [`PageView`]'s, as every page's is.
pub trait SettingsPageView: PageView {
    /// The page's place in the index.
    const PAGE: SettingsPage;
}

/// What [`PageView::render`] draws with beside the view model.
pub struct PageDraw<'a, P: PageView> {
    /// The page's clicks, each sending one of its Presenter's events.
    pub clicks: &'a PageClicks<ModulePage<P>>,
    /// Whether the page, not Settings' index, has keyboard focus.
    pub focused: bool,
    pub widgets: &'a mut P::Widgets,
}

/// A module page: its Presenter, the Setup its Router is built from and the widget state its View
/// keeps.
pub struct ModulePage<P: PageView> {
    presenter: P,
    setup: P::Setup,
    widgets: P::Widgets,
    /// Whether the page has the keyboard, told by its host as it draws the page.
    page_focused: bool,
    /// Repaints the page when a domain in [`PageView::REPAINTS`] changes, and follows what
    /// [`PageView::subscribe`] names.
    _subscriptions: Vec<Subscription>,
}

impl<P: PageView> ModulePage<P> {
    /// Hosts `presenter`, its Router built over `setup`, repainting on `announcer`'s changes to
    /// [`PageView::REPAINTS`] and following [`PageView::subscribe`]'s sources.
    pub fn new(
        presenter: P,
        setup: P::Setup,
        announcer: &Entity<Announcer>,
        cx: &mut Context<'_, Self>,
    ) -> Self {
        let mut subscriptions: Vec<Subscription> = match P::ledger_changed() {
            None => P::REPAINTS
                .iter()
                .map(|domain| notify_on(announcer, *domain, cx))
                .collect(),
            // The announcement comes after the input's commit, so the event is handled directly:
            // telling the Presenter writes nothing.
            Some(_) => vec![cx.subscribe(
                announcer,
                |page: &mut Self, _, event: &LedgerChanged, cx| {
                    if P::REPAINTS.iter().any(|domain| event.what.contains(domain))
                        && let Some(changed) = P::ledger_changed()
                        && let Some(followed) = page.handle(changed, cx)
                    {
                        cx.emit(followed);
                    }
                },
            )],
        };
        subscriptions.extend(P::subscribe(&setup, cx));
        Self {
            presenter,
            setup,
            widgets: P::Widgets::default(),
            page_focused: false,
            _subscriptions: subscriptions,
        }
    }

    /// Hosts `presenter` on no Ledger, for a page that reads no Ledger domain and stands before a
    /// Ledger (and its `Announcer`) exists: General, Tracing and About. It follows only
    /// [`PageView::subscribe`]'s sources.
    pub fn detached(presenter: P, setup: P::Setup, cx: &mut Context<'_, Self>) -> Self {
        debug_assert!(
            P::REPAINTS.is_empty() && P::ledger_changed().is_none(),
            "a detached page has no domain to repaint on"
        );
        let subscriptions = P::subscribe(&setup, cx);
        Self {
            presenter,
            setup,
            widgets: P::Widgets::default(),
            page_focused: false,
            _subscriptions: subscriptions,
        }
    }

    pub fn view_model(&self) -> P::ViewModel {
        self.presenter.view_model()
    }

    /// Changes what the Presenter holds outside an event (a `[keybindings]` rebind, what start-up
    /// read), then redraws.
    pub fn update_presenter(&mut self, cx: &mut Context<'_, Self>, change: impl FnOnce(&mut P)) {
        change(&mut self.presenter);
        cx.notify();
    }

    /// The Presenter, for a test's snapshot of what it holds beyond its view model.
    pub fn presenter(&self) -> &P {
        &self.presenter
    }

    /// What the page's Router is built from, for a test's snapshot.
    pub fn setup(&self) -> &P::Setup {
        &self.setup
    }

    /// The widget state the View keeps, for a test's snapshot (Tracing's list scroll).
    pub fn widgets(&self) -> &P::Widgets {
        &self.widgets
    }

    /// Where a Dialog the page opens hands its confirmed outcome, inside the confirm's input,
    /// whose commit names what the confirm wrote. A page that has gone has nothing to land on.
    pub fn landing(cx: &Context<'_, Self>) -> impl Fn(P::Outcome, &mut App) + 'static {
        let page = cx.weak_entity();
        move |outcome, cx| {
            let _ = page.update(cx, |page, cx| page.dialog_landed(outcome, cx));
        }
    }

    /// A key the page's keymap binds, by its `chord`: the event it sends, unless the page doesn't
    /// take it now. The Shell has already taken its own keys. One whose event
    /// [`PageView::passes_on`] goes on to the Shell's fallback even once taken, inside the same
    /// input.
    pub fn module_key(&mut self, chord: &str, cx: &mut Context<'_, Self>) -> PageKey {
        let event = self.presenter.keymap().event(chord);
        let passes_on = event.as_ref().is_some_and(P::passes_on);
        match (self.take(event, cx), passes_on) {
            (false, _) => PageKey::Declined,
            (true, false) => PageKey::Taken,
            (true, true) => PageKey::PassedOn,
        }
    }

    /// A palette Command, by its keymap `command` name: the event its key sends.
    /// The tab or mode it belongs to is shown first ([`PagePresenter::shows`]), in the same input.
    /// Whether the page took the Command's own event: one it refuses once shown is reported
    /// refused, even though its tab or mode now shows.
    pub fn module_command(&mut self, command: &str, cx: &mut Context<'_, Self>) -> bool {
        let Some(event) = self.presenter.keymap().event_for(command) else {
            return false;
        };
        let Some(shown) = self.presenter.shows(&event) else {
            return self.take(Some(event), cx);
        };
        let (followed, taken) = crate::input::join_or_send(cx, |cx| {
            let mut followed: Vec<ViewEvent> = self.handle(shown, cx).into_iter().collect();
            let taken = self.presenter.takes(&event);
            if taken {
                followed.extend(self.handle(event, cx));
            }
            (followed, taken)
        });
        for event in followed {
            cx.emit(event);
        }
        taken
    }

    /// Shows a record another View handed off, if the page shows it.
    pub fn reveal(&mut self, reveal: Reveal, cx: &mut Context<'_, Self>) {
        if let Some(event) = P::reveal(reveal) {
            self.send(event, cx);
        }
    }

    /// `Esc` on the page: closes what [`PageView::escape`] names, if the page has it open.
    pub fn escape(&mut self, cx: &mut Context<'_, Self>) -> bool {
        let event = P::escape(&self.view_model());
        self.take(event, cx)
    }

    pub fn page_status(&self) -> PageStatus {
        P::status(&self.view_model())
    }

    /// The `?` Help Dialog's group for the page.
    pub fn help_group(&self) -> Option<HelpGroup> {
        P::help(&self.view_model())
    }

    /// Hands the Presenter [`PageView::framed`]'s event for `frame`, if it takes it. Not an input:
    /// the frame is layout, and it is told outside any input (as the Settings View draws).
    pub fn framed(&mut self, frame: PageFrame, cx: &mut Context<'_, Self>) {
        if let Some(event) = P::framed(frame).filter(|event| self.presenter.takes(event))
            && let Some(followed) = self.handle(event, cx)
        {
            cx.emit(followed);
        }
    }

    /// The page has just become the destination: its widget state starts afresh (scrolled to the
    /// top), and the Presenter is handed [`PageView::entered`]. Not an input of its own: entering
    /// writes nothing, and the navigation that entered the page is the input, or follows a
    /// hand-off's, whose commit has landed.
    pub fn on_enter(&mut self, cx: &mut Context<'_, Self>) {
        self.widgets = P::Widgets::default();
        match P::entered() {
            Some(event) => {
                if let Some(followed) = self.handle(event, cx) {
                    cx.emit(followed);
                }
            }
            None => cx.notify(),
        }
    }

    /// `/`: the keymap's `search` command, reporting whether the page is now in its Search. A
    /// page without one leaves `/` inert.
    pub fn start_search(&mut self, cx: &mut Context<'_, Self>) -> bool {
        self.module_command(SEARCH, cx) && self.presenter.mode() == Some(PageMode::Search)
    }

    /// A key while the page is in a mode of its own: offered to the keymap by its chord, then to
    /// [`PagePresenter::typed`]. Returns whether either took it; in Normal mode the page takes
    /// nothing here, and the key goes on to the Shell's keymap.
    pub fn mode_key(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        if self.presenter.mode().is_none() {
            return false;
        }
        if let Some(chord) = chord(keystroke)
            && self.module_key(&chord, cx) == PageKey::Taken
        {
            return true;
        }
        let events: Vec<P::Event> = text_keys(keystroke)
            .into_iter()
            .filter_map(|key| self.presenter.typed(key))
            .collect();
        if events.is_empty() {
            return false;
        }
        let followed = crate::input::join_or_send(cx, |cx| {
            events
                .into_iter()
                .filter_map(|event| self.handle(event, cx))
                .collect::<Vec<_>>()
        });
        for event in followed {
            cx.emit(event);
        }
        true
    }

    /// A key or click on the page: `event` as one input, committed once it is handled. Inside an
    /// input already (a palette Command's), it joins that one.
    ///
    /// What [`PageView::after`] asks of the Shell is told it once the input is sent, so it lands
    /// after the Notices the input's drain shows: a status the event clears stays cleared (#828).
    pub fn send(&mut self, event: P::Event, cx: &mut Context<'_, Self>) {
        if let Some(followed) = crate::input::join_or_send(cx, |cx| self.handle(event, cx)) {
            cx.emit(followed);
        }
    }

    /// A confirmed Dialog's outcome, handed back by the Dialog host inside the confirm's input.
    fn dialog_landed(&mut self, outcome: P::Outcome, cx: &mut Context<'_, Self>) {
        if let Some(event) = P::dialog_closed(Some(outcome))
            && let Some(followed) = self.handle(event, cx)
        {
            cx.emit(followed);
        }
    }

    /// The one dispatch a keymap event takes into the page: sent as one input, unless the page
    /// doesn't take it now, which leaves the key to the Shell's next step.
    fn take(&mut self, event: Option<P::Event>, cx: &mut Context<'_, Self>) -> bool {
        let Some(event) = event.filter(|event| self.presenter.takes(event)) else {
            return false;
        };
        self.send(event, cx);
        true
    }

    /// Hands `event` to the Presenter with the page's Router, then tells the Shell of a mode the
    /// Presenter entered or left and follows the handle in the widget state, then redraws. Inside
    /// an input already, as [`Self::send`] or a Dialog's confirm. Returns what
    /// [`PageView::after`] asks of the Shell, for the caller to tell it.
    fn handle(&mut self, event: P::Event, cx: &mut Context<'_, Self>) -> Option<ViewEvent> {
        let mode = self.presenter.mode();
        let before = self.presenter.before(&event, &self.widgets);
        let Self {
            presenter, setup, ..
        } = self;
        P::with_router(presenter, setup, cx, |presenter, router| {
            presenter.handle(event, router);
        });
        let now = self.presenter.mode();
        if now != mode {
            cx.emit(match now {
                Some(mode) => ViewEvent::EnterMode(view_mode(mode)),
                None => ViewEvent::ExitMode,
            });
        }
        let followed = self.presenter.after(before, &self.setup, &mut self.widgets);
        cx.notify();
        followed
    }
}

/// The Shell's mode for a page's own.
fn view_mode(mode: PageMode) -> ViewMode {
    match mode {
        PageMode::Insert => ViewMode::Insert,
        PageMode::Search => ViewMode::Search,
        PageMode::Filter => ViewMode::Filter,
    }
}

/// What `keystroke` types into a page's mode: the editing keys, or each character an unmodified
/// key types. A modified chord types nothing, as in a Dialog (`DialogKey::Char`).
fn text_keys(keystroke: &Keystroke) -> Vec<TextKey> {
    let shift = keystroke.modifiers.shift;
    match keystroke.key.as_str() {
        "backspace" => vec![TextKey::Backspace],
        "enter" => vec![TextKey::Enter { shift }],
        "tab" => vec![TextKey::Tab { shift }],
        _ if keystroke.modifiers.control
            || keystroke.modifiers.alt
            || keystroke.modifiers.platform
            || keystroke.modifiers.function =>
        {
            Vec::new()
        }
        _ => keystroke
            .key_char
            .as_deref()
            .map(|text| text.chars().map(TextKey::Char).collect())
            .unwrap_or_default(),
    }
}

impl<P: PageView> EventEmitter<ViewEvent> for ModulePage<P> {}
impl<P: PageView> EventEmitter<PageRowClicked> for ModulePage<P> {}

impl<P: PageView> Render for ModulePage<P> {
    fn render(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> impl gpui::IntoElement {
        let view = self.presenter.view_model();
        let clicks = PageClicks::new(cx);
        let page = PageDraw {
            clicks: &clicks,
            focused: self.page_focused,
            widgets: &mut self.widgets,
        };
        P::render(&view, page, window, cx)
    }
}

/// The page's clicks on its rows and buttons.
impl<P: PageView> PresenterPage<P::Event> for ModulePage<P> {
    fn send(&mut self, event: P::Event, cx: &mut Context<'_, Self>) {
        ModulePage::send(self, event, cx);
    }

    /// A click on a row hands the page focus.
    fn row_clicked(&mut self, cx: &mut Context<'_, Self>) {
        cx.emit(PageRowClicked);
    }

    fn menu_at(&mut self, at: Point<Pixels>, _cx: &mut Context<'_, Self>) {
        P::menu_at(&mut self.widgets, at);
    }
}

/// What a host reads of the page beyond its keys, through `PageHandle`.
impl<P: PageView> ModulePage<P> {
    /// Told whether the page has the keyboard, as its host draws it and as focus moves in or out:
    /// the View zone for a main View, the page rather than the index for a Settings page. The focus
    /// rings the page draws within itself follow it.
    pub fn set_focused(&mut self, focused: bool, cx: &mut Context<'_, Self>) {
        self.page_focused = focused;
        if self.presenter.focus_changed(focused) {
            cx.notify();
        }
    }

    /// A Settings page heading's right-aligned meta, [`PageView::heading_meta`].
    pub fn heading_meta(&self, cx: &App) -> Option<AnyElement> {
        P::heading_meta(&self.view_model(), cx)
    }

    /// A pane drawn at the View's right edge, beside the page, its clicks reaching `entity`.
    pub fn side_pane(&self, entity: &Entity<Self>, cx: &App) -> Option<AnyElement> {
        P::side_pane(&self.view_model(), &PageClicks::of(entity), cx)
    }
}
