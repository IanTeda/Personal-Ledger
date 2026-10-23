//! The Bills page's View: the `lib-bills` page Presenter in a gpui Entity, which the Shell hosts as
//! the Bills destination (`docs/ux/desktop-mockups/12-bills/`). It looks up each key the
//! Shell passes on, as chord text, in the Bills keymap, turns clicks into events, draws the view model (`schedule.rs`, `filters.rs`,
//! `planner.rs`), and keeps the Ledger read copy in line after every event ([`ports::refresh`]).
//! Each key and click is one input (`crate::input::send`), which commits what it wrote.
//!
//! A focused scope select's open list and highlight are gpui's widget state, so they are kept here:
//! while one has focus, `j`/`k` and `Enter` drive it instead of the rows.

use crate::navigation::nav::NounLabel;
use std::rc::Rc;

use gpui::{
    AnyElement, App, Context, Entity, EventEmitter, Keystroke, ScrollHandle, SharedString,
    Subscription, Window, div, prelude::*, px,
};
use lib_bill_payments::{PayOutcome, SharedPayments};
use lib_bills::BillsOutcome;
use lib_bills::presenter::{
    BillsPageEvent, BillsPagePresenter, BillsPageViewModel, BillsTab, FilterField, ScheduleEvent,
};
use lib_core::Clock;
use lib_ledger::Domain;
use lib_list_cursor::Step;
use lib_viper::keymap::Keymap;
use lib_viper::view::SelectView;

use super::{OnPlainClick, OnRowClick, filters, planner, schedule};
use crate::bills::ports::{self, DesktopPorts};
use crate::bills::router::PageRouter;
use crate::chrome::dialog_host::DialogKey;
use crate::form::select::SelectState;
use crate::form::viper::{choose_option, select_key, synced, toggle_select};
use crate::ledger_store::{LedgerChanged, LedgerStore};
use crate::navigation::{movement::Movement, nav::Noun};
use crate::settings::display::StatusGlyphs;
use crate::{
    chrome::statusline::PageStatus,
    theme::color,
    view::event::{Reveal, ViewEvent},
};

impl EventEmitter<ViewEvent> for BillsPage {}

pub struct BillsPage {
    presenter: BillsPagePresenter,
    /// The day the page's Dialogs are opened on.
    clock: Clock,
    ports: DesktopPorts,
    /// What the Pay Dialog the page opens writes Pay it directly through.
    payments: SharedPayments,
    ledger: Entity<LedgerStore>,
    keymap: Keymap<BillsPageEvent>,
    /// The status glyphs Display Preference, which flags a row in Needs Attention.
    glyphs: StatusGlyphs,
    /// The page's own scroll, reset as it becomes the destination and as its tab changes.
    scroll: ScrollHandle,
    /// The focused scope select and its open list and highlight.
    filter_select: Option<(FilterField, SelectState)>,
    _ledger_changed: Subscription,
}

impl BillsPage {
    pub fn new(
        ports: DesktopPorts,
        payments: SharedPayments,
        ledger: Entity<LedgerStore>,
        keymap: Keymap<BillsPageEvent>,
        clock: Clock,
        cx: &mut Context<'_, Self>,
    ) -> Self {
        ports::copy_reads(&ports, ledger.read(cx).ledger());
        let presenter = BillsPagePresenter::new(ports.clone(), clock.clone(), keymap.clone());
        // The rows name Accounts, Categories, Payees, Transactions and Units, which the Ledger
        // still holds: a change to any is copied into the ports and redrawn, as is a Bills write
        // `LedgerStore` made (Account delete's Unmatch).
        let _ledger_changed = cx.subscribe(&ledger, |page, ledger, event: &LedgerChanged, cx| {
            let reads = [
                Domain::Accounts,
                Domain::Categories,
                Domain::Payees,
                Domain::Transactions,
                Domain::Units,
            ];
            if reads.iter().any(|domain| event.what.contains(domain)) {
                ports::copy_reads(&page.ports, ledger.read(cx).ledger());
                cx.notify();
            } else if event.what.contains(&Domain::Bills) {
                cx.notify();
            }
        });
        Self {
            presenter,
            clock,
            ports,
            payments,
            ledger,
            keymap,
            glyphs: StatusGlyphs::default(),
            scroll: ScrollHandle::new(),
            filter_select: None,
            _ledger_changed,
        }
    }

    /// A key or click on the page, or the page being entered: `event` as one input, committed
    /// once it is handled.
    pub(in crate::bills) fn send(&mut self, event: BillsPageEvent, cx: &mut Context<'_, Self>) {
        crate::input::send(cx, |cx| self.handle(event, cx));
    }

    /// Hands `event` to the Presenter with the Desktop's Router, then brings the Ledger read copy
    /// in line with what it wrote. Inside an input already, as [`Self::send`] or a Dialog's
    /// confirm.
    fn handle(&mut self, event: BillsPageEvent, cx: &mut Context<'_, Self>) {
        let tab = self.presenter.tab();
        let mut router = PageRouter {
            ports: self.ports.clone(),
            payments: self.payments.clone(),
            clock: self.clock.clone(),
            date_style: self.presenter.date_style(),
            ledger: self.ledger.clone(),
            cx,
        };
        self.presenter.handle(event, &mut router);
        ports::refresh(&self.ports, &self.ledger, cx);
        if self.presenter.tab() != tab {
            self.scroll.set_offset(gpui::Point::default());
        }
        cx.notify();
    }

    /// A confirmed Dialog's outcome, handed back by the Dialog host inside the confirm's input.
    pub(in crate::bills) fn dialog_landed(
        &mut self,
        outcome: BillsOutcome,
        cx: &mut Context<'_, Self>,
    ) {
        self.handle(BillsPageEvent::DialogClosed(Some(outcome)), cx);
    }

    /// A landed Pay Dialog, handed back by the Dialog host: the page redraws from its ports, the
    /// entry Paid. A Pay raises no Toast, since the row turning Paid is the feedback. Inside the
    /// confirm's input.
    pub(in crate::bills) fn pay_landed(
        &mut self,
        _outcome: PayOutcome,
        cx: &mut Context<'_, Self>,
    ) {
        self.handle(BillsPageEvent::DialogClosed(None), cx);
    }

    /// The page as drawn.
    pub fn view_model(&self) -> BillsPageViewModel {
        self.presenter.view_model()
    }

    /// The scope select `field`'s view, from the view model.
    fn filter_view(
        view: &BillsPageViewModel,
        field: FilterField,
    ) -> Option<SelectView<lib_bills::presenter::FilterChoice>> {
        view.schedule
            .filters
            .selects
            .iter()
            .find(|select| select.field == field)
            .map(|select| select.view.clone())
    }

    /// The widget state of the select `field`, closed and fresh when another had it.
    fn select_state(&mut self, field: FilterField) -> &mut SelectState {
        if self.filter_select.as_ref().map(|(at, _)| *at) != Some(field) {
            self.filter_select = Some((field, SelectState::default()));
        }
        &mut self
            .filter_select
            .get_or_insert_with(|| (field, SelectState::default()))
            .1
    }

    /// A key on the focused scope select: steps or opens its list, and applies a new choice.
    fn filter_key(&mut self, key: DialogKey, cx: &mut Context<'_, Self>) {
        let view = self.view_model();
        let Some(field) = view.schedule.filters.focused else {
            return;
        };
        let Some(select) = Self::filter_view(&view, field) else {
            return;
        };
        let (_, chosen) = select_key(self.select_state(field), &select, key);
        match chosen {
            Some(choice) => self.send(
                BillsPageEvent::Schedule(ScheduleEvent::Filter(field, choice)),
                cx,
            ),
            None => cx.notify(),
        }
    }

    fn filter_focused(&self) -> bool {
        self.presenter.filter_focus().is_some()
    }

    /// A click on a scope select: focuses it and opens its list, or closes its own open list.
    fn click_filter_field(&mut self, field: FilterField, cx: &mut Context<'_, Self>) {
        if self.presenter.filter_focus() != Some(field) {
            self.send(
                BillsPageEvent::Schedule(ScheduleEvent::FocusFilter(field)),
                cx,
            );
        }
        let view = self.view_model();
        if let Some(select) = Self::filter_view(&view, field) {
            toggle_select(self.select_state(field), &select);
        }
        cx.notify();
    }

    /// A click on row `index` of a scope select's open list.
    fn click_filter_option(
        &mut self,
        field: FilterField,
        index: usize,
        cx: &mut Context<'_, Self>,
    ) {
        let view = self.view_model();
        let Some(select) = Self::filter_view(&view, field) else {
            return;
        };
        match choose_option(self.select_state(field), &select, index) {
            Some(choice) => self.send(
                BillsPageEvent::Schedule(ScheduleEvent::Filter(field, choice)),
                cx,
            ),
            None => cx.notify(),
        }
    }

    /// Every chord the Bills keymap binds, the arrows, `g g` and `enter` among them. The Shell has
    /// already taken its own keys. While a scope select has focus, the moves up and down and
    /// `enter` drive it and the other movements wait.
    pub fn module_key(&mut self, chord: &str, cx: &mut Context<'_, Self>) -> bool {
        let Some(event) = self.keymap.event(chord) else {
            return false;
        };
        match event {
            BillsPageEvent::Move(step) if self.filter_focused() => match step {
                Step::Next => self.filter_key(DialogKey::Down, cx),
                Step::Prev => self.filter_key(DialogKey::Up, cx),
                _ => {}
            },
            BillsPageEvent::Open if self.filter_focused() => self.filter_key(DialogKey::Enter, cx),
            event => self.send(event, cx),
        }
        true
    }

    /// Every Bills key is the keymap's.
    pub fn key_ahead(&mut self, _keystroke: &Keystroke, _cx: &mut Context<'_, Self>) -> bool {
        false
    }

    /// Every Bills key is the keymap's.
    pub fn handle_key(&mut self, _keystroke: &Keystroke, _cx: &mut Context<'_, Self>) -> bool {
        false
    }

    /// A movement the keymap doesn't bind does nothing on the page.
    pub fn apply_movement(&mut self, _movement: Movement, _cx: &mut Context<'_, Self>) {}

    /// The scope selects: the first `Esc` closes an open list, the next leaves the filter row.
    /// Returns whether it took the `Esc`.
    pub fn escape(&mut self, cx: &mut Context<'_, Self>) -> bool {
        if !self.filter_focused() {
            return false;
        }
        if let Some((_, select)) = self.filter_select.as_mut()
            && select.is_open()
        {
            select.cancel();
            cx.notify();
            return true;
        }
        self.filter_select = None;
        self.send(BillsPageEvent::Schedule(ScheduleEvent::LeaveFilters), cx);
        true
    }

    /// The Bills page has no Search, so `/` is inert on it.
    pub fn start_search(&mut self, _cx: &mut Context<'_, Self>) -> bool {
        false
    }

    /// The Bills page has no Search box to type into.
    pub fn search_key(&mut self, _keystroke: &Keystroke, _cx: &mut Context<'_, Self>) -> bool {
        false
    }

    /// The page becomes the destination: it starts scrolled to the top, and generates a month the
    /// Desktop has crossed into since.
    pub fn on_enter(&mut self, cx: &mut Context<'_, Self>) {
        self.scroll.set_offset(gpui::Point::default());
        self.send(BillsPageEvent::Entered, cx);
    }

    /// Shows a record handed off from another View.
    pub fn reveal(&mut self, reveal: Reveal, cx: &mut Context<'_, Self>) {
        let event = match reveal {
            Reveal::BillEntry(id) => BillsPageEvent::RevealEntry(id),
            Reveal::BillPlan(id) => BillsPageEvent::RevealPlan(id),
            Reveal::BillsPeriod(period) => BillsPageEvent::RevealPeriod(period),
            Reveal::Transaction(_)
            | Reveal::Account(_)
            | Reveal::AccountTransactions(_)
            | Reveal::Payee(_) => return,
        };
        self.send(event, cx);
    }

    /// Follows a change to the Display Preferences.
    pub fn set_display(
        &mut self,
        date_style: Option<lib_core::DateStyle>,
        glyphs: StatusGlyphs,
        cx: &mut Context<'_, Self>,
    ) {
        self.glyphs = glyphs;
        self.send(BillsPageEvent::SetDateStyle(date_style), cx);
    }

    /// The status line's hints and right-hand text for the tab shown.
    pub fn page_status(&self, _cx: &App) -> Option<PageStatus> {
        let view = self.view_model();
        Some(PageStatus {
            hints: view
                .hints
                .into_iter()
                .map(|hint| (hint.chord, hint.action))
                .collect(),
            right: view.status,
        })
    }

    /// The focused scope select's widget state, a closed one on its choice for any other.
    fn filter_states(&self, view: &BillsPageViewModel) -> Vec<SelectState> {
        view.schedule
            .filters
            .selects
            .iter()
            .map(|select| {
                let own = self
                    .filter_select
                    .as_ref()
                    .filter(|(field, _)| *field == select.field && select.view.focused)
                    .map(|(_, state)| state.clone())
                    .unwrap_or_default();
                synced(&own, &select.view)
            })
            .collect()
    }
}

/// A click that sends `event` to the page.
fn on_event(entity: &Entity<BillsPage>, event: BillsPageEvent) -> OnPlainClick {
    let entity = entity.clone();
    Rc::new(move |_window, cx| {
        let event = event.clone();
        entity.update(cx, |page, cx| page.send(event, cx));
    })
}

/// A row click that sends `make(index)` to the page.
fn on_row(entity: &Entity<BillsPage>, make: fn(usize) -> BillsPageEvent) -> OnRowClick {
    let entity = entity.clone();
    Rc::new(move |index, _window, cx| {
        entity.update(cx, |page, cx| page.send(make(index), cx));
    })
}

impl Render for BillsPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let entity = cx.entity();
        let view = self.view_model();
        let schedule_handlers = schedule::Handlers {
            on_row_click: on_row(&entity, |i| {
                BillsPageEvent::Schedule(ScheduleEvent::Select(i))
            }),
            on_pay_click: on_row(&entity, |i| {
                BillsPageEvent::Schedule(ScheduleEvent::PayRow(i))
            }),
            on_skip_click: on_row(&entity, |i| {
                BillsPageEvent::Schedule(ScheduleEvent::SkipRow(i))
            }),
            on_view_transaction_click: on_row(&entity, |i| {
                BillsPageEvent::Schedule(ScheduleEvent::ViewTransaction(i))
            }),
            filters: filters::Handlers {
                on_chip_click: {
                    let entity = entity.clone();
                    Rc::new(move |status, _window, cx| {
                        entity.update(cx, |page, cx| {
                            page.send(
                                BillsPageEvent::Schedule(ScheduleEvent::ToggleChip(status)),
                                cx,
                            );
                        });
                    })
                },
                on_field_click: {
                    let entity = entity.clone();
                    Rc::new(move |field, _window, cx| {
                        entity.update(cx, |page, cx| page.click_filter_field(field, cx));
                    })
                },
                on_option_click: {
                    let entity = entity.clone();
                    Rc::new(move |field, index, _window, cx| {
                        entity.update(cx, |page, cx| page.click_filter_option(field, index, cx));
                    })
                },
            },
        };
        let planner_handlers = planner::Handlers {
            on_row_click: on_row(&entity, |i| {
                BillsPageEvent::Planner(lib_bills::presenter::PlannerEvent::Select(i))
            }),
            on_edit_click: on_row(&entity, |i| {
                BillsPageEvent::Planner(lib_bills::presenter::PlannerEvent::EditRow(i))
            }),
        };
        let cx: &App = cx;
        let (meta, body) = match view.tab {
            BillsTab::Schedule => (
                schedule::meta_line(&view.schedule.meta, cx),
                schedule::render(
                    &view.schedule,
                    &self.filter_states(&view),
                    self.glyphs,
                    schedule_handlers,
                    cx,
                ),
            ),
            BillsTab::Planner => (
                planner::meta_line(&view.planner, cx),
                planner::render(&view.planner, planner_handlers, cx),
            ),
        };
        div()
            .id("bills")
            .flex_1()
            .min_w(px(0.0))
            .h_full()
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .px(px(28.0))
            .py(px(22.0))
            .child(page_header(
                meta,
                &view.add_button,
                on_event(&entity, BillsPageEvent::New),
                cx,
            ))
            .child(tab_row(&view, &entity, cx))
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

fn page_header(
    meta: AnyElement,
    add_button: &str,
    on_add_click: OnPlainClick,
    cx: &App,
) -> impl IntoElement {
    let hover = color::muted(cx);
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
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .text_size(px(28.0))
                        .text_color(color::foreground(cx))
                        .child(Noun::Bills.label()),
                )
                .child(meta),
        )
        .child(
            div()
                .id("bills-add")
                .debug_selector(|| "bills-add".to_string())
                .cursor_pointer()
                .flex_none()
                .py(px(10.0))
                .px(px(16.0))
                .bg(color::foreground(cx))
                .text_color(color::selection_text(cx))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .whitespace_nowrap()
                .hover(move |style| style.bg(hover))
                .on_click(move |_event, window, cx| on_add_click(window, cx))
                .child(add_button.to_string()),
        )
}

/// The handoff's tab switcher: the active tab a dark filled cell, the rest plain text; the period
/// nav at the right on the Schedule tab only.
fn tab_row(view: &BillsPageViewModel, entity: &Entity<BillsPage>, cx: &App) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .children(view.tabs.iter().map(|tab| {
            let id = tab.tab;
            let on_click = on_event(entity, BillsPageEvent::ShowTab(id));
            let hover = color::hover(cx);
            div()
                .id(SharedString::from(format!("bills-tab-{id:?}")))
                .debug_selector(move || format!("bills-tab-{id:?}"))
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
                .on_click(move |_event, window, cx| on_click(window, cx))
                .child(tab.label.clone())
        }))
        .child(div().flex_1())
        .when(view.tab == BillsTab::Schedule, |this| {
            this.child(period_nav(view, entity, cx))
        })
}

/// `‹ Sep 2026 ›  All`: the arrows step the Schedule a calendar month (from All, back to the month
/// last viewed); `All` toggles every row ever generated.
fn period_nav(view: &BillsPageViewModel, entity: &Entity<BillsPage>, cx: &App) -> impl IntoElement {
    let schedule = &view.schedule;
    let arrow = |id: &'static str, glyph: &'static str, event: ScheduleEvent| {
        let hover = color::foreground(cx);
        let on_click = on_event(entity, BillsPageEvent::Schedule(event));
        div()
            .id(id)
            .debug_selector(move || id.to_string())
            .cursor_pointer()
            .px(px(8.0))
            .text_color(color::muted(cx))
            .hover(move |style| style.text_color(hover))
            .on_click(move |_event, window, cx| on_click(window, cx))
            .child(glyph)
    };
    let on_all_click = on_event(entity, BillsPageEvent::Schedule(ScheduleEvent::ToggleAll));
    let hover = color::hover(cx);
    div()
        .flex()
        .items_center()
        .gap(px(4.0))
        .text_size(px(12.0))
        .child(arrow(
            "bills-period-prev",
            "\u{2039}",
            ScheduleEvent::PrevPeriod,
        ))
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_color(if schedule.all {
                    color::muted(cx)
                } else {
                    color::foreground(cx)
                })
                .whitespace_nowrap()
                .child(schedule.period_label.clone()),
        )
        .child(arrow(
            "bills-period-next",
            "\u{203a}",
            ScheduleEvent::NextPeriod,
        ))
        .child(
            div()
                .id("bills-period-all")
                .debug_selector(|| "bills-period-all".to_string())
                .cursor_pointer()
                .ml(px(8.0))
                .py(px(4.0))
                .px(px(10.0))
                .border_1()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .when(schedule.all, |this| {
                    this.bg(color::selection_background(cx))
                        .border_color(color::selection_background(cx))
                        .text_color(color::selection_text(cx))
                })
                .when(!schedule.all, |this| {
                    this.border_color(color::border(cx))
                        .text_color(color::muted(cx))
                        .hover(move |style| style.bg(hover))
                })
                .on_click(move |_event, window, cx| on_all_click(window, cx))
                .child(schedule.all_label.clone()),
        )
}
