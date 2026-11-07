//! The Budgets page's View: the `lib-budgets` page Presenter in a gpui Entity, which the Shell
//! hosts as the Budgets destination (`docs/ux/desktop-mockups/14-budgets-v2/`). It looks up each
//! key the Shell passes on, as chord text, in the Budgets keymap for the tab shown, turns clicks
//! into events, draws the view model (`progress.rs`, `plan.rs`, `history.rs`). Each key and click is one
//! input (`crate::input::send`), which commits what it wrote.
//!
//! Typing into a Plan cell is the Shell's Insert mode, which the page asks for as the Presenter
//! starts an edit and leaves as it ends one. While it is on, every key reaches the page ahead of
//! the Shell ([`BudgetsPage::key_ahead`]) and turns into a cell event; `esc` comes back as
//! [`BudgetsPage::escape`].

use gpui::{
    AnyElement, App, Context, Entity, EventEmitter, Keystroke, ScrollHandle, SharedString,
    Subscription, Window, div, prelude::*, px,
};
use lib_budgets::presenter::{
    BudgetsPageEvent, BudgetsPagePresenter, BudgetsPageViewModel, BudgetsTab, HeaderView,
    HistoryEvent, PlanEvent, ProgressEvent, TabBody,
};
use lib_budgets::{BudgetsOutcome, Span};
use lib_core::{BudgetId, Clock, DateStyle};
use lib_database::Domain;
use lib_locale::format::upper;
use lib_viper::keymap::Keymap;

use super::{history, plan, progress};
use crate::announcer::{Announcer, LedgerChanged};
use crate::budgets::router::PageRouter;
use crate::navigation::movement::Movement;
use crate::navigation::nav::Noun;
use crate::shell::view::page_clicks::{PageClicks, PresenterPage};
use crate::shell::view::statusline::PageStatus;
use crate::theme::color;
use crate::view::event::{Reveal, ViewEvent, ViewMode};
use crate::view::help::HelpGroup;
use crate::view::placeholder;
use lib_budgets::SharedPorts;

impl EventEmitter<ViewEvent> for BudgetsPage {}

pub struct BudgetsPage {
    presenter: BudgetsPagePresenter,
    ports: SharedPorts,
    clock: Clock,
    keymap: Keymap<BudgetsPageEvent>,
    /// The page's own scroll, reset as it becomes the destination, as its tab changes and as
    /// another Budget is shown.
    scroll: ScrollHandle,
    _ledger_changed: Subscription,
}

impl BudgetsPage {
    pub fn new(
        ports: SharedPorts,
        announcer: Entity<Announcer>,
        keymap: Keymap<BudgetsPageEvent>,
        clock: Clock,
        cx: &mut Context<'_, Self>,
    ) -> Self {
        let presenter = BudgetsPagePresenter::new(ports.clone(), clock.clone(), keymap.clone());
        // The figures read every domain Budgets counts, so a change the Ledger announces in any of
        // them is redrawn.
        let _ledger_changed = cx.subscribe(&announcer, |_, _, event: &LedgerChanged, cx| {
            let reads = [
                Domain::Accounts,
                Domain::Bills,
                Domain::Budgets,
                Domain::Categories,
                Domain::Payees,
                Domain::Transactions,
                Domain::Units,
            ];
            if reads.iter().any(|domain| event.what.contains(domain)) {
                cx.notify();
            }
        });
        Self {
            presenter,
            ports,
            clock,
            keymap,
            scroll: ScrollHandle::new(),
            _ledger_changed,
        }
    }

    /// The Budget on show.
    pub fn current(&self) -> BudgetId {
        self.presenter.current()
    }

    /// The ports the page reads, which its snapshot reads too.
    pub fn ports(&self) -> &SharedPorts {
        &self.ports
    }

    /// The day the figures are worked out on.
    pub fn today(&self) -> chrono::NaiveDate {
        self.clock.today()
    }

    pub(in crate::budgets) fn presenter(&self) -> &BudgetsPagePresenter {
        &self.presenter
    }

    /// A key or click on the page, or the Shell telling it something: `event` as one input,
    /// committed once it is handled.
    fn send(&mut self, event: BudgetsPageEvent, cx: &mut Context<'_, Self>) {
        crate::input::join_or_send(cx, |cx| self.handle(event, cx));
    }

    /// Hands `event` to the Presenter with the Desktop's Router, then the Shell its Insert mode
    /// with the cell being typed into. Inside an input already, as [`Self::send`] or a Dialog's confirm.
    fn handle(&mut self, event: BudgetsPageEvent, cx: &mut Context<'_, Self>) {
        let (tab, budget) = (self.presenter.tab(), self.presenter.current());
        let cell_event = matches!(
            event,
            BudgetsPageEvent::Plan(
                PlanEvent::Type(_) | PlanEvent::Backspace | PlanEvent::Save { .. }
            )
        );
        // Anything else drops the cell being typed into, so Insert is left before a Dialog the
        // event opens asks for its own mode.
        if self.presenter.editing() && !cell_event {
            cx.emit(ViewEvent::ExitMode);
        }
        let was_editing = self.presenter.editing() && cell_event;
        let mut router = PageRouter {
            ports: self.ports.clone(),
            clock: self.clock.clone(),
            date_style: self.presenter.date_style(),
            cx,
        };
        self.presenter.handle(event, &mut router);
        match (was_editing, self.presenter.editing()) {
            (false, true) => cx.emit(ViewEvent::EnterMode(ViewMode::Insert)),
            (true, false) => cx.emit(ViewEvent::ExitMode),
            _ => {}
        }
        if self.presenter.tab() != tab || self.presenter.current() != budget {
            self.scroll.set_offset(gpui::Point::default());
        }
        cx.notify();
    }

    /// A closed Switcher or Manage budgets, handed back by the Dialog host inside the input that
    /// closed it.
    pub(in crate::budgets) fn dialog_landed(
        &mut self,
        outcome: Option<BudgetsOutcome>,
        cx: &mut Context<'_, Self>,
    ) {
        self.handle(BudgetsPageEvent::DialogClosed(outcome), cx);
    }

    /// The page as drawn.
    pub fn view_model(&self) -> BudgetsPageViewModel {
        self.presenter.view_model()
    }

    /// Follows a change to `[keybindings]`.
    pub fn set_keymaps(&mut self, config: &lib_config::KeyBindingConfig) {
        self.keymap = lib_budgets::keymap::budgets_page_keymap(config);
        self.presenter.set_keymap(self.keymap.clone());
    }

    /// Every chord the Budgets keymap binds on the tab shown, the arrows, `g g` and `enter`
    /// among them. The Shell has already taken its own keys.
    pub fn module_key(&mut self, chord: &str, cx: &mut Context<'_, Self>) -> bool {
        let Some(event) = lib_budgets::keymap::event_on(&self.keymap, chord, self.presenter.tab())
        else {
            return false;
        };
        self.send(event, cx);
        true
    }

    /// A palette Command, by its keymap `command` name: the event its key sends, on the tab it
    /// belongs to. A Command of another tab switches to it first, in the same input, since its
    /// key does nothing there.
    pub fn module_command(&mut self, command: &str, cx: &mut Context<'_, Self>) -> bool {
        let Some(event) =
            lib_budgets::keymap::command_on(&self.keymap, command, self.presenter.tab())
        else {
            return false;
        };
        let other_tab = event.tab().filter(|tab| *tab != self.presenter.tab());
        crate::input::join_or_send(cx, |cx| {
            if let Some(tab) = other_tab {
                self.handle(BudgetsPageEvent::ShowTab(tab), cx);
            }
            self.handle(event, cx);
        });
        true
    }

    /// While a Plan cell is being typed into, every key is the cell's (9b's Insert mode): `enter`
    /// saves onward and `shift+enter` this month only, `tab`/`shift+tab` save and move to the
    /// next or previous month, and a printable key types. `esc` is left to the Shell, which hands
    /// it back as [`Self::escape`].
    pub fn key_ahead(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        if !self.presenter.editing() {
            return false;
        }
        let shift = keystroke.modifiers.shift;
        let events: Vec<PlanEvent> = match keystroke.key.as_str() {
            "escape" => return false,
            "enter" => vec![PlanEvent::Save {
                span: if shift { Span::MonthOnly } else { Span::Onward },
                step: 0,
            }],
            "tab" => vec![PlanEvent::Save {
                span: Span::Onward,
                step: if shift { -1 } else { 1 },
            }],
            "backspace" => vec![PlanEvent::Backspace],
            _ => match (keystroke.key_char.as_deref(), keystroke.modifiers.control) {
                (Some(text), false) => text.chars().map(PlanEvent::Type).collect(),
                _ => Vec::new(),
            },
        };
        crate::input::send(cx, |cx| {
            for event in events {
                self.handle(BudgetsPageEvent::Plan(event), cx);
            }
        });
        true
    }

    /// Every Budgets key is the keymap's.
    pub fn handle_key(&mut self, _keystroke: &Keystroke, _cx: &mut Context<'_, Self>) -> bool {
        false
    }

    /// A movement the keymap doesn't bind does nothing on the page.
    pub fn apply_movement(&mut self, _movement: Movement, _cx: &mut Context<'_, Self>) {}

    /// `Esc` drops the Plan cell being typed into and leaves Insert. Nothing else on the page
    /// holds an `Esc` of its own.
    pub fn escape(&mut self, cx: &mut Context<'_, Self>) -> bool {
        if !self.presenter.editing() {
            return false;
        }
        self.send(BudgetsPageEvent::Plan(PlanEvent::Abandon), cx);
        true
    }

    /// Budgets has no Search, so `/` is inert.
    pub fn start_search(&mut self, _cx: &mut Context<'_, Self>) -> bool {
        false
    }

    pub fn search_key(&mut self, _keystroke: &Keystroke, _cx: &mut Context<'_, Self>) -> bool {
        false
    }

    /// Back to the top of the page as it becomes the destination; an edit left open when the
    /// page was left doesn't come back with it.
    pub fn on_enter(&mut self, cx: &mut Context<'_, Self>) {
        self.scroll.set_offset(gpui::Point::default());
        self.send(BudgetsPageEvent::Entered, cx);
    }

    /// No other View hands a record to Budgets yet.
    pub fn reveal(&mut self, _reveal: Reveal, _cx: &mut Context<'_, Self>) {}

    /// The calendar month turned while the Desktop was open: inside the day poll's input.
    pub fn month_turned(&mut self, cx: &mut Context<'_, Self>) {
        self.handle(BudgetsPageEvent::MonthTurned, cx);
    }

    /// Follows a change to the Display Preferences.
    pub fn set_display(&mut self, date_style: Option<DateStyle>, cx: &mut Context<'_, Self>) {
        self.send(BudgetsPageEvent::SetDateStyle(date_style), cx);
    }

    /// Shows Budget `id`: one the New budget form created.
    pub fn show_budget(&mut self, id: BudgetId, cx: &mut Context<'_, Self>) {
        self.handle(BudgetsPageEvent::ShowBudget(id), cx);
    }

    /// The status line's hints and right-hand text for the tab shown.
    pub fn page_status(&self, _cx: &App) -> Option<PageStatus> {
        let view = self.view_model();
        Some(PageStatus {
            hints: view.hints,
            right: view.status,
        })
    }

    /// The `?` cheat-sheet's Budgets group: the hints the status line shows for the tab shown.
    pub fn help_group(&self, cx: &App) -> Option<HelpGroup> {
        let keys = self
            .page_status(cx)?
            .hints
            .into_iter()
            .map(|hint| (hint.action, hint.chord))
            .collect();
        Some(HelpGroup {
            heading: crate::msg::desktop_help_budgets_label(),
            keys,
        })
    }
}

/// Reached by clicks only (a key goes through the inherent `send`). A tab click also clears the
/// status line after its event, as the key that switches tabs does.
impl PresenterPage<BudgetsPageEvent> for BudgetsPage {
    fn send(&mut self, event: BudgetsPageEvent, cx: &mut Context<'_, Self>) {
        let show_tab = matches!(event, BudgetsPageEvent::ShowTab(_));
        Self::send(self, event, cx);
        if show_tab {
            cx.emit(ViewEvent::SetStatus(None));
        }
    }
}

impl Render for BudgetsPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let clicks = PageClicks::new(cx);
        let view = self.view_model();
        let cx: &App = cx;
        let page = div()
            .id("budgets")
            .flex_1()
            .min_w(px(0.0))
            .h_full()
            .overflow_y_scroll()
            .track_scroll(&self.scroll);
        let Some(header) = view.header.as_ref() else {
            // The Budget on show has gone.
            return page.child(placeholder::render(Noun::Budgets, cx));
        };
        let body = match &view.body {
            TabBody::Progress(progress) => progress::render(progress, &clicks, cx),
            TabBody::Plan(plan) => plan::render(plan, &clicks, cx),
            TabBody::History(history) => history::render(history, &clicks, cx),
            TabBody::Gone => div().into_any_element(),
        };
        page.px(px(28.0))
            .py(px(22.0))
            .child(page_header(header, view.tab, &clicks, cx))
            .child(tab_row(&view, &clicks, cx))
            .child(
                div()
                    .h(px(2.0))
                    .flex_none()
                    .bg(color::structural_rule(cx))
                    .mb(px(24.0)),
            )
            .child(body)
    }
}

/// The title as the Switcher (name, chevron, method tag) with the meta line below it, the Plan's
/// Fill button, and the primary action at the right.
fn page_header(
    header: &HeaderView,
    tab: BudgetsTab,
    clicks: &PageClicks<BudgetsPage>,
    cx: &App,
) -> impl IntoElement {
    let hover = color::muted(cx);
    let on_title = clicks.send(BudgetsPageEvent::Switch);
    let on_action = clicks.send(header.action_event.clone());
    div()
        .flex()
        .items_end()
        .justify_between()
        .gap(px(16.0))
        .mb(px(16.0))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(6.0))
                .child(
                    div()
                        .id("budgets-title")
                        .debug_selector(|| "budgets-title".to_string())
                        .cursor_pointer()
                        .on_click(on_title)
                        .flex()
                        .items_center()
                        .gap(px(10.0))
                        .child(
                            div()
                                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                                .text_size(px(28.0))
                                .text_color(color::foreground(cx))
                                .child(header.name.clone()),
                        )
                        .child(
                            div()
                                .text_size(px(14.0))
                                .text_color(color::muted(cx))
                                .child("\u{25be}"),
                        )
                        .child(method_tag(&header.method, cx)),
                )
                .child(meta_line(&header.meta, cx)),
        )
        .when_some(header.fill.clone(), |this, fill| {
            let on_fill = clicks.send(BudgetsPageEvent::Plan(PlanEvent::Fill));
            let hover = color::hover(cx);
            this.child(div().flex_1()).child(
                div()
                    .id("budgets-fill")
                    .debug_selector(|| "budgets-fill".to_string())
                    .cursor_pointer()
                    .flex_none()
                    .py(px(9.0))
                    .px(px(16.0))
                    .border_1()
                    .border_color(color::border(cx))
                    .font_weight(gpui::FontWeight::EXTRA_BOLD)
                    .whitespace_nowrap()
                    .hover(move |style| style.bg(hover))
                    .on_click(on_fill)
                    .child(fill),
            )
        })
        .child(
            div()
                .id("budgets-primary-action")
                .debug_selector(|| "budgets-primary-action".to_string())
                .cursor_pointer()
                .flex_none()
                .py(px(10.0))
                .px(px(16.0))
                .bg(color::foreground(cx))
                .text_color(color::selection_text(cx))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .whitespace_nowrap()
                .hover(move |style| style.bg(hover))
                .on_click(on_action)
                .child(header.action.clone()),
        )
        // The tab is part of what a click on the action means, so a test can tell them apart.
        .debug_selector(move || format!("budgets-header-{tab:?}"))
}

/// The method's solid tag: 10px, extra bold, tracked out.
pub(super) fn method_tag(method: &str, cx: &App) -> impl IntoElement {
    div()
        .py(px(4.0))
        .px(px(7.0))
        .bg(color::foreground(cx))
        .text_color(color::selection_text(cx))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(10.0))
        .child(upper(method))
}

/// The meta line's clauses, separated by a middle dot; a `strong` run is the accent.
fn meta_line(clauses: &[Vec<lib_locale::Segment>], cx: &App) -> impl IntoElement {
    let accent = color::accent_text(cx);
    let clause = |segments: &Vec<lib_locale::Segment>| {
        div()
            .flex()
            .children(segments.iter().map(move |segment| {
                match segment.tag.as_deref() {
                    Some("strong") => div()
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .text_color(accent)
                        .child(segment.text.clone())
                        .into_any_element(),
                    _ => div().child(segment.text.clone()).into_any_element(),
                }
            }))
            .into_any_element()
    };
    div()
        .flex()
        .flex_wrap()
        .gap(px(6.0))
        .text_size(px(12.0))
        .text_color(color::muted(cx))
        .children(clauses.iter().enumerate().flat_map(|(index, segments)| {
            let separator = (index > 0).then(|| div().child("\u{b7}").into_any_element());
            separator
                .into_iter()
                .chain(std::iter::once(clause(segments)))
        }))
}

/// The active tab a dark filled cell, the rest plain text; the Progress legend and the period nav
/// at the right.
fn tab_row(
    view: &BudgetsPageViewModel,
    clicks: &PageClicks<BudgetsPage>,
    cx: &App,
) -> impl IntoElement {
    let (previous, next) = match view.tab {
        BudgetsTab::Progress => (
            BudgetsPageEvent::Progress(ProgressEvent::PreviousPeriod),
            BudgetsPageEvent::Progress(ProgressEvent::NextPeriod),
        ),
        BudgetsTab::Plan => (
            BudgetsPageEvent::Plan(PlanEvent::PreviousRange),
            BudgetsPageEvent::Plan(PlanEvent::NextRange),
        ),
        BudgetsTab::History => (
            BudgetsPageEvent::History(HistoryEvent::PreviousRange),
            BudgetsPageEvent::History(HistoryEvent::NextRange),
        ),
    };
    div()
        .flex()
        .items_center()
        .children(view.tabs.iter().map(|tab| {
            let id = tab.tab;
            // A click isn't a key, so the Shell keeps the status line through it: the tab clears
            // a message about the tab it leaves, as the key that switches tabs does.
            let on_click = clicks.send(BudgetsPageEvent::ShowTab(id));
            let hover = color::hover(cx);
            div()
                .id(SharedString::from(format!("budgets-tab-{id:?}")))
                .debug_selector(move || format!("budgets-tab-{id:?}"))
                .cursor_pointer()
                .py(px(8.0))
                .px(px(16.0))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .when(tab.active, |this| {
                    this.bg(color::selection_background(cx))
                        .text_color(color::selection_text(cx))
                })
                .when(!tab.active, |this| {
                    this.text_color(color::muted(cx))
                        .hover(move |style| style.bg(hover))
                })
                .on_click(on_click)
                .child(tab.label.clone())
        }))
        .when_some(
            match &view.body {
                TabBody::Progress(progress) => Some(progress.legend.clone()),
                _ => None,
            },
            |this, legend| this.child(progress::legend(&legend, cx)),
        )
        .child(div().flex_1())
        .child(period_nav(
            view.nav.clone(),
            clicks.send(previous),
            clicks.send(next),
            cx,
        ))
}

/// A period arrow's click, boxed so the two arrows share one builder.
type ArrowClick = Box<dyn Fn(&gpui::ClickEvent, &mut Window, &mut App)>;

/// `‹ Sep 2026 ›`, or the Plan's `‹ Jul – Dec 2026 ›`.
fn period_nav(
    label: String,
    on_prev: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
    on_next: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> AnyElement {
    let arrow = |id: &'static str, glyph: &'static str, on_click: ArrowClick| {
        let hover = color::foreground(cx);
        div()
            .id(id)
            .debug_selector(move || id.to_string())
            .cursor_pointer()
            .px(px(8.0))
            .text_color(color::muted(cx))
            .hover(move |style| style.text_color(hover))
            .on_click(on_click)
            .child(glyph)
    };
    div()
        .flex()
        .items_center()
        .gap(px(4.0))
        .text_size(px(12.0))
        .child(arrow("budgets-period-prev", "\u{2039}", Box::new(on_prev)))
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_color(color::foreground(cx))
                .whitespace_nowrap()
                .child(label),
        )
        .child(arrow("budgets-period-next", "\u{203a}", Box::new(on_next)))
        .into_any_element()
}
