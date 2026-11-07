//! The one page Entity every module page Presenter is hosted in (ADR-0056): [`ModulePage<P>`]
//! owns the Presenter, the widget state its View keeps beside it, whether the page has focus and
//! its repaints, and is the one take, send and handle path a key, a palette Command, a click or a
//! landed Dialog reaches the Presenter through. A domain writes a [`PageView`] on its page
//! Presenter (its Router, what it repaints on, how it draws) and no Entity of its own.
//!
//! The page never counts or announces a write: each input's `send` announces what its commit
//! names (ADR-0055), and the page repaints on the domains in [`PageView::REPAINTS`].
//!
//! It takes no `handle_key`, `key_ahead` or `search_key`: those are the pre-module key paths
//! #1079 retires, so a page that still needs one stays hand-written until then.
//! [`PresenterPage`] is how [`PageClicks`] reaches the page, for the generic page as for the
//! hand-written ones.

use gpui::{AnyElement, App, Context, Entity, EventEmitter, IntoElement as _, Pixels, Point};
use gpui::{Render, Subscription, Window};
use lib_database::Domain;
use lib_shell::Hint;
use lib_viper::presenter::PagePresenter;

use super::page_clicks::{PageClicks, PresenterPage};
use super::statusline::PageStatus;
use crate::announcer::{Announcer, notify_on};
use crate::navigation::movement::Movement;
use crate::settings::SettingsPage;
use crate::settings::pages::{Page, PageRowClicked};
use crate::view::event::{Reveal, ViewEvent};

/// The bin's half of a page Presenter: the Router it is lent for each call, the domains it
/// repaints on, and how it draws. Implemented on the Presenter type itself.
pub trait PageView: PagePresenter + Sized + 'static {
    /// What the page's Router is built from for each call: the ports its Dialogs open on and
    /// anything else it routes with (the Clock, the Dialog keys). The page holds it beside the
    /// Presenter.
    type Setup: 'static;

    /// The widget state that is gpui's to keep beside the Presenter, which the Presenter never
    /// sees: a scroll handle, a row menu's position. `()` for a page with none.
    type Widgets: Default + 'static;

    /// The Ledger domains whose change repaints the page: its own, and any its view model reads.
    const REPAINTS: &'static [Domain];

    /// Runs `handle` with the page's Router, built over `setup` around this call's context, which
    /// is how a Router opens a Dialog in the host.
    fn with_router<R>(
        setup: &Self::Setup,
        cx: &mut Context<'_, ModulePage<Self>>,
        handle: impl FnOnce(&mut Self::Router<'_>) -> R,
    ) -> R;

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

    /// Records where the pointer opened a row's menu, before its event is sent. The position is
    /// the View's to draw the menu at; the Presenter never sees it.
    fn menu_at(_widgets: &mut Self::Widgets, _at: Point<Pixels>) {}
}

/// A Settings page's [`PageView`]: its place in the index, which `Page` is keyed by.
pub trait SettingsPageView: PageView {
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
    /// Whether the page, not Settings' index, has keyboard focus, told by the Settings View as it
    /// draws the page.
    page_focused: bool,
    /// Repaints the page when a domain in [`PageView::REPAINTS`] changes.
    _repaints: Vec<Subscription>,
}

impl<P: PageView> ModulePage<P> {
    /// Hosts `presenter`, its Router built over `setup`, repainting on `announcer`'s changes to
    /// [`PageView::REPAINTS`].
    pub fn new(
        presenter: P,
        setup: P::Setup,
        announcer: &Entity<Announcer>,
        cx: &mut Context<'_, Self>,
    ) -> Self {
        Self {
            presenter,
            setup,
            widgets: P::Widgets::default(),
            page_focused: false,
            _repaints: P::REPAINTS
                .iter()
                .map(|domain| notify_on(announcer, *domain, cx))
                .collect(),
        }
    }

    pub fn view_model(&self) -> P::ViewModel {
        self.presenter.view_model()
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
    /// take it now. The Shell has already taken its own keys.
    pub fn module_key(&mut self, chord: &str, cx: &mut Context<'_, Self>) -> bool {
        let event = self.presenter.keymap().event(chord);
        self.take(event, cx)
    }

    /// A palette Command, by its keymap `command` name: the event its key sends.
    pub fn module_command(&mut self, command: &str, cx: &mut Context<'_, Self>) -> bool {
        let event = self.presenter.keymap().event_for(command);
        self.take(event, cx)
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

    /// A key or click on the page: `event` as one input, committed once it is handled. Inside an
    /// input already (a palette Command's), it joins that one.
    pub fn send(&mut self, event: P::Event, cx: &mut Context<'_, Self>) {
        crate::input::join_or_send(cx, |cx| self.handle(event, cx));
    }

    /// A confirmed Dialog's outcome, handed back by the Dialog host inside the confirm's input.
    fn dialog_landed(&mut self, outcome: P::Outcome, cx: &mut Context<'_, Self>) {
        self.handle(P::dialog_closed(Some(outcome)), cx);
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

    /// Hands `event` to the Presenter with the page's Router, then redraws. Inside an input
    /// already, as [`Self::send`] or a Dialog's confirm.
    fn handle(&mut self, event: P::Event, cx: &mut Context<'_, Self>) {
        let Self {
            presenter, setup, ..
        } = self;
        P::with_router(setup, cx, |router| presenter.handle(event, router));
        cx.notify();
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

impl<P: SettingsPageView> Page for ModulePage<P> {
    const PAGE: SettingsPage = P::PAGE;

    /// Every chord the page's keymap binds, the arrows and `g g` among them.
    fn module_key(&mut self, chord: &str, cx: &mut Context<'_, Self>) -> bool {
        ModulePage::module_key(self, chord, cx)
    }

    fn module_command(&mut self, command: &str, cx: &mut Context<'_, Self>) -> bool {
        ModulePage::module_command(self, command, cx)
    }

    /// A movement the keymap doesn't bind (`enter`) does nothing on the page, and Settings leaves
    /// the page where it is: the keymap binds every movement a module page takes.
    fn apply_movement(&mut self, _movement: Movement, _cx: &mut Context<'_, Self>) -> bool {
        true
    }

    fn reveal(&mut self, reveal: Reveal, cx: &mut Context<'_, Self>) {
        ModulePage::reveal(self, reveal, cx);
    }

    fn escape(&mut self, cx: &mut Context<'_, Self>) -> bool {
        ModulePage::escape(self, cx)
    }

    fn set_page_focused(&mut self, focused: bool, _cx: &mut Context<'_, Self>) {
        self.page_focused = focused;
    }

    fn heading_meta(&self, cx: &App) -> AnyElement {
        P::heading_meta(&self.view_model(), cx)
            .unwrap_or_else(|| crate::settings::scope_note(P::PAGE).into_any_element())
    }

    fn hints(&self, _cx: &App) -> Vec<Hint> {
        self.page_status().hints
    }

    fn page_status(&self, _cx: &App) -> Option<PageStatus> {
        Some(self.page_status())
    }

    fn status_note(&self, _cx: &App) -> String {
        self.page_status().right
    }

    fn side_pane(&self, entity: &Entity<Self>, cx: &App) -> Option<AnyElement> {
        P::side_pane(&self.view_model(), &PageClicks::of(entity), cx)
    }
}
