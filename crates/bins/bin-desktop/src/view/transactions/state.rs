//! The Transactions destination's Entity (ADR-0032, ADR-0037). [`TransactionsView`] owns the page's
//! own state: the selected row, the table's scroll, the filters, the search and the filter
//! popover, which runs on `lib_transactions`' `FilterPresenter` (#928): the Entity opens it on a
//! draft of the applied filters, hands it the popover's keys (read through the Shell's Dialog
//! keymap) and clicks, and takes the filters it applies. It reads the rows through `LedgerStore`, and the Accounts, Categories, Payees
//! and Tags its chips and filter choices draw on through theirs. The Import View is its own Entity
//! (`view::import`). It renders itself (`impl Render` in `super`), handles
//! its own keys, `Esc`, Search and movement, and reaches `Root` only through [`ViewEvent`].

use chrono::NaiveDate;
use gpui::{
    App, Context, Entity, EventEmitter, Keystroke, ScrollStrategy, Subscription,
    UniformListScrollHandle,
};

use lib_core::DateStyle;
use lib_list_cursor::Step;
use lib_shell::DialogCommand;
use lib_transactions::presenter::{
    ChipField, FilterEvent, FilterField, FilterOutcome, FilterPresenter, FilterViewModel,
};
use lib_viper::keymap::Keymap;

use super::{
    header::ChipBounds,
    hints::{filter_hints, transactions_hints},
};
use lib_payees::Payee;

use crate::{
    form::select::SelectState,
    form::viper::{chosen_label, labels},
    ledger_store::{LedgerStore, notify_on},
    navigation::{chord::chord, movement::Movement},
    settings::display::{RowDensity, StatusGlyphs},
    shell::view::statusline::{PageStatus, owned_hints},
    transactions::{
        self, Transaction,
        query::{Ledger, StatusFilter, TransactionFilters},
        rows::DisplayPrefs,
    },
    view::{
        event::{Reveal, ViewEvent, ViewMode},
        format,
    },
};
use lib_ledger::Domain;

impl EventEmitter<ViewEvent> for TransactionsView {}

/// The Transactions page's own state, owned by its Entity.
pub struct TransactionsState {
    /// The selected row, a position in the filtered list; clamped wherever it is read.
    pub selected: usize,
    /// The table's `uniform_list` scroll state: scrolls the selected row into view and reports the
    /// viewport height for half-page moves.
    pub scroll: UniformListScrollHandle,
    /// What the table is filtered by; starts at the defaults (this year, everything else empty).
    pub filters: TransactionFilters,
    /// The search box's text, separate from the filters.
    pub search: String,
    /// The filter popover's Presenter, holding its draft, `Some` while it is open
    /// (`InputMode::Filter`, which the View entered). Kept apart from [`Self::filters`] until
    /// **apply**.
    pub filter: Option<FilterPresenter>,
    /// A popover select's open list and highlight, the View's widget state: the field it belongs
    /// to and the list, which shows the Presenter's choice.
    pub filter_list: Option<(FilterField, SelectState)>,
    /// The chip that opened the popover, which it anchors under.
    pub filter_anchor: ChipField,
    /// Where each chip was last painted; the header writes it, the popover reads it.
    pub chip_bounds: ChipBounds,
}

impl TransactionsState {
    pub fn new(today: NaiveDate) -> Self {
        Self {
            selected: 0,
            scroll: UniformListScrollHandle::new(),
            filters: TransactionFilters::defaults(today),
            search: String::new(),
            filter: None,
            filter_list: None,
            filter_anchor: ChipField::Account,
            chip_bounds: ChipBounds::default(),
        }
    }
}

/// The store Entities the Transactions page reads.
#[derive(Clone)]
pub struct TransactionsSources {
    pub ledger: Entity<LedgerStore>,
}

/// The Transactions Entity.
pub struct TransactionsView {
    pub(super) sources: TransactionsSources,
    pub(super) today: NaiveDate,
    /// The Display Preferences the table's text and row height follow. Pushed by `Root`, which
    /// owns the Settings page that changes them.
    pub(super) date_style: Option<DateStyle>,
    pub(super) glyphs: StatusGlyphs,
    pub(super) density: RowDensity,
    /// The Shell's Dialog keymap, which the filter popover reads its keys through. Pushed by
    /// `Root`, which builds it under `[keybindings]`.
    dialog_keymap: Keymap<DialogCommand>,
    /// Whether the View's Search is open (`InputMode::Search`, which the View entered).
    pub(super) searching: bool,
    pub(super) state: TransactionsState,
    /// Repaints the page when the Accounts its rows and filters name change.
    _accounts_changed: Subscription,
    /// Repaints the page when the Payees its rows and filters name change.
    _payees_changed: Subscription,
    _categories_changed: Subscription,
    _tags_changed: Subscription,
}

impl TransactionsView {
    pub fn new(
        sources: TransactionsSources,
        clock: &lib_core::Clock,
        cx: &mut Context<'_, Self>,
    ) -> Self {
        let today = clock.today();
        let _accounts_changed = notify_on(&sources.ledger, Domain::Accounts, cx);
        let _payees_changed = notify_on(&sources.ledger, Domain::Payees, cx);
        let _categories_changed = notify_on(&sources.ledger, Domain::Categories, cx);
        let _tags_changed = notify_on(&sources.ledger, Domain::Tags, cx);
        Self {
            sources,
            today,
            date_style: None,
            glyphs: StatusGlyphs::default(),
            density: RowDensity::default(),
            dialog_keymap: lib_shell::dialog_keymap(&lib_config::KeyBindingConfig::default()),
            searching: false,
            state: TransactionsState::new(today),
            _accounts_changed,
            _payees_changed,
            _categories_changed,
            _tags_changed,
        }
    }

    pub fn state(&self) -> &TransactionsState {
        &self.state
    }

    /// Follows a change to the Display Preferences.
    pub fn set_display(
        &mut self,
        date_style: Option<DateStyle>,
        glyphs: StatusGlyphs,
        density: RowDensity,
        cx: &mut Context<'_, Self>,
    ) {
        if density != self.density {
            // The table's scroll offset is in pixels, so a new row height would leave it pointing
            // at a different row: re-anchor on the selected one for its next paint.
            self.state
                .scroll
                .scroll_to_item_strict(self.state.selected, ScrollStrategy::Center);
        }
        self.date_style = date_style;
        self.glyphs = glyphs;
        self.density = density;
        cx.notify();
    }

    /// Follows a change to the Dialog keymap's `[keybindings]`.
    pub fn set_dialog_keymap(&mut self, keymap: Keymap<DialogCommand>) {
        self.dialog_keymap = keymap;
    }

    pub fn transactions<'a>(&self, cx: &'a App) -> &'a [Transaction] {
        self.sources.ledger.read(cx).ledger().transactions()
    }

    fn payees<'a>(&self, cx: &'a App) -> &'a [Payee] {
        self.sources.ledger.read(cx).ledger().payees()
    }

    /// The reference data the Transactions engine reads, borrowed from the shared stores.
    pub(super) fn ledger<'a>(&self, cx: &'a App) -> Ledger<'a> {
        Ledger {
            accounts: self.sources.ledger.read(cx).ledger().accounts(),
            categories: self.sources.ledger.read(cx).ledger().categories(),
            payees: self.payees(cx),
            tags: self.sources.ledger.read(cx).ledger().tags(),
            units: self.sources.ledger.read(cx).ledger().units(),
        }
    }

    /// The Display preferences the table's text is formatted with.
    pub(super) fn prefs(&self) -> DisplayPrefs {
        DisplayPrefs {
            date_style: self.date_style,
            glyphs: self.glyphs,
        }
    }

    /// How many rows the current filters and search leave visible.
    fn visible_len(&self, cx: &App) -> usize {
        transactions::query::query(
            &self.ledger(cx),
            self.transactions(cx),
            &self.state.filters,
            &self.state.search,
        )
        .rows
        .len()
    }

    /// Keys the page claims before the Shell's fallback: every key but `Esc` while the filter
    /// popover is open (the View entered `InputMode::Filter` for it).
    pub fn key_ahead(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        if keystroke.key == "escape" {
            return false;
        }
        if self.state.filter.is_some() {
            return self.filter_key(keystroke, cx);
        }
        false
    }

    /// The page's `f`, `n` and `e`. `f` opens the filter popover; the bundle designs no add or edit
    /// flow, so `n` and `e` say so.
    pub fn handle_key(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        if keystroke.modifiers.shift {
            return false;
        }
        let message = match keystroke.key.as_str() {
            "f" => {
                self.open_filter_popover(None, cx);
                return true;
            }
            "n" => crate::msg::desktop_status_add_transaction_not_yet_built(),
            "e" => crate::msg::desktop_status_edit_transaction_not_yet_built(),
            _ => return false,
        };
        cx.emit(ViewEvent::SetStatus(Some(message)));
        true
    }

    /// `j`/`k`/`g`/`G`/`Ctrl-d`/`Ctrl-u` move the table's row selection and keep it in view;
    /// `Enter` would open the transaction, which the bundle designs no screen for.
    ///
    /// The scroll strategy follows the research note: a non-strict `scroll_to_item` applies its
    /// strategy only when the row was off-screen, so stepping down uses `Bottom` and stepping up
    /// `Top` (the new row lands at the edge it came from), and jumps use `Center`.
    pub fn apply_movement(&mut self, movement: Movement, cx: &mut Context<'_, Self>) {
        let len = self.visible_len(cx);
        if len == 0 {
            return;
        }
        let selected = transactions::rows::clamp_selection(self.state.selected, len);
        let viewport = f32::from(
            self.state
                .scroll
                .0
                .borrow()
                .base_handle
                .bounds()
                .size
                .height,
        );
        let half =
            transactions::rows::half_page_rows(viewport, format::row_height_px(self.density));
        let Some(step) = movement.step() else {
            cx.emit(ViewEvent::SetStatus(Some(
                crate::msg::desktop_status_open_transaction_not_yet_built(),
            )));
            return;
        };
        let strategy = match step {
            Step::Next | Step::Last => ScrollStrategy::Bottom,
            Step::Prev | Step::First => ScrollStrategy::Top,
            Step::HalfPageDown | Step::HalfPageUp => ScrollStrategy::Center,
        };
        let next = lib_list_cursor::step_by(selected, len, step, half);
        self.state.selected = next;
        self.state.scroll.scroll_to_item(next, strategy);
        cx.notify();
    }

    /// `Esc`: the filter popover's open list first, then the popover itself (its draft is
    /// discarded), then the Search (its text is cleared, the map's decision: search is cleared by
    /// `Esc` or by emptying the box).
    pub fn escape(&mut self, cx: &mut Context<'_, Self>) -> bool {
        if self.state.filter.is_some() {
            self.filter_back(cx);
            return true;
        }
        if self.searching {
            self.state.search.clear();
            self.reset_selection();
            self.stop_search(cx);
            return true;
        }
        false
    }

    /// `/` opens the Search box; `Root` enters `InputMode::Search` for it.
    pub fn start_search(&mut self, cx: &mut Context<'_, Self>) -> bool {
        self.searching = true;
        cx.notify();
        true
    }

    /// Keys while the Search is open: typing filters live, `Backspace` edits, and `Enter` keeps the
    /// text and returns to browsing the (filtered) rows. `Esc` comes back as [`Self::escape`].
    pub fn search_key(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        match keystroke.key.as_str() {
            "enter" => self.stop_search(cx),
            "backspace" => {
                self.state.search.pop();
                self.reset_selection();
            }
            _ => {
                let modifiers = &keystroke.modifiers;
                if modifiers.control || modifiers.alt || modifiers.platform || modifiers.function {
                    return false;
                }
                match keystroke.key_char.as_deref() {
                    Some(text)
                        if text.chars().count() == 1
                            && text.chars().next().is_some_and(|ch| !ch.is_control()) =>
                    {
                        self.state.search.push_str(text);
                        self.reset_selection();
                    }
                    _ => return false,
                }
            }
        }
        cx.notify();
        true
    }

    /// The page has just become the destination. A Search left open when the page was left doesn't
    /// come back with it.
    pub fn on_enter(&mut self, cx: &mut Context<'_, Self>) {
        if self.searching {
            self.stop_search(cx);
        }
        cx.notify();
    }

    /// Shows what another View handed off: an Account's ledger (the table filtered to it), or a
    /// Transaction, selected, with the date range widened to reach it when it falls outside this
    /// year's and nothing else filtering it out. A no-op when the Transaction has gone.
    pub fn reveal(&mut self, reveal: Reveal, cx: &mut Context<'_, Self>) {
        if let Reveal::AccountTransactions(account_id) = reveal {
            self.show_filtered(TransactionFilters::for_account(self.today, account_id), cx);
            return;
        }
        let Reveal::Transaction(transaction_id) = reveal else {
            return;
        };
        let Some(date) = self
            .transactions(cx)
            .iter()
            .find(|t| t.id == transaction_id)
            .map(|t| t.date)
        else {
            return;
        };
        let mut filters = TransactionFilters::defaults(self.today);
        filters.from = filters.from.map(|from| from.min(date));
        filters.to = filters.to.map(|to| to.max(date));
        self.state.filters = filters;
        self.state.search.clear();
        self.close_filter();
        let index = transactions::query::query(
            &self.ledger(cx),
            self.transactions(cx),
            &self.state.filters,
            &self.state.search,
        )
        .rows
        .iter()
        .position(|row| row.transaction.id == transaction_id)
        .unwrap_or(0);
        self.state.selected = index;
        self.state
            .scroll
            .scroll_to_item_strict(index, ScrollStrategy::Center);
        cx.notify();
    }

    /// The status line's legend (the filter popover's or the page's), and its right-hand text.
    pub fn page_status(&self, _cx: &App) -> Option<PageStatus> {
        Some(PageStatus {
            hints: owned_hints(if self.state.filter.is_some() {
                filter_hints()
            } else {
                transactions_hints()
            }),
            right: format::status_legend(self.glyphs),
        })
    }

    /// The table filtered to `filters` from its first row, with no search or draft left over: the
    /// "View transactions" links of Accounts, Payees, Tags and the Budgets detail.
    pub fn show_filtered(&mut self, filters: TransactionFilters, cx: &mut Context<'_, Self>) {
        self.state.filters = filters;
        self.state.search.clear();
        self.close_filter();
        self.reset_selection();
        cx.notify();
    }

    /// Leaves the Search, keeping its text.
    fn stop_search(&mut self, cx: &mut Context<'_, Self>) {
        self.searching = false;
        cx.emit(ViewEvent::ExitMode);
    }

    /// Puts the table back on its first row: called whenever the filters or the search change what
    /// is visible, since the old selection no longer points at the same row, and after an Import
    /// commit lands on the page.
    pub(crate) fn reset_selection(&mut self) {
        self.state.selected = 0;
        self.state.scroll.scroll_to_item(0, ScrollStrategy::Top);
    }

    /// Opens the filter popover on a draft of the applied filters. A chip focuses its own field
    /// (the date chip focuses From) and the card anchors under it; `f` (no chip) focuses the first
    /// field and anchors under the first chip.
    pub(super) fn open_filter_popover(
        &mut self,
        chip: Option<ChipField>,
        cx: &mut Context<'_, Self>,
    ) {
        let presenter = FilterPresenter::open(
            &self.ledger(cx),
            &self.state.filters,
            chip,
            self.today,
            self.date_style,
        );
        self.state.filter_anchor = chip.unwrap_or(ChipField::Account);
        self.state.filter = Some(presenter);
        self.state.filter_list = None;
        cx.emit(ViewEvent::EnterMode(ViewMode::Filter));
        cx.notify();
    }

    /// Discards the popover's draft and leaves the mode it opened.
    pub(super) fn close_filter_popover(&mut self, cx: &mut Context<'_, Self>) {
        self.close_filter();
        cx.emit(ViewEvent::ExitMode);
        cx.notify();
    }

    /// Drops the popover and its open list, without leaving a mode.
    fn close_filter(&mut self) {
        self.state.filter = None;
        self.state.filter_list = None;
    }

    /// The open popover's view model.
    pub(super) fn filter_view(&self) -> Option<FilterViewModel> {
        self.state.filter.as_ref().map(FilterPresenter::view_model)
    }

    /// Hands `event` to the popover's Presenter: **apply** commits the filters it hands back,
    /// closes the popover and puts the table back on its first row, and a cancel discards the
    /// draft. An open list on a field that has lost focus closes.
    fn send_filter(&mut self, event: FilterEvent, cx: &mut Context<'_, Self>) {
        let Some(presenter) = self.state.filter.as_mut() else {
            return;
        };
        match presenter.handle(event) {
            Some(FilterOutcome::Applied(filters)) => {
                self.state.filters = filters;
                self.close_filter_popover(cx);
                self.reset_selection();
            }
            Some(FilterOutcome::Cancelled) => self.close_filter_popover(cx),
            None => {
                let focused = presenter.focused();
                if self
                    .state
                    .filter_list
                    .as_ref()
                    .is_some_and(|(field, _)| *field != focused)
                {
                    self.state.filter_list = None;
                }
            }
        }
        cx.notify();
    }

    /// Keys while the filter popover is open, read through the Dialog keymap: `next_field` and
    /// `previous_field` move between fields, `select_next` and `select_previous` step a closed
    /// select or move an open list's highlight, `select` opens or commits a select's list and
    /// otherwise applies, and `submit` applies from anywhere. Any other key is the field's own:
    /// `Space` on a select or the Status control, `Left` / `Right` on Status, `Ctrl-r` reset,
    /// and typing or `Backspace` in a text field. `Esc` comes back as [`Self::escape`].
    fn filter_key(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        match chord(keystroke).and_then(|chord| self.dialog_keymap.event(&chord)) {
            Some(command) => {
                self.filter_command(command, cx);
                true
            }
            None => self.filter_typed(keystroke, cx),
        }
    }

    fn filter_command(&mut self, command: DialogCommand, cx: &mut Context<'_, Self>) {
        let Some(focused) = self.state.filter.as_ref().map(FilterPresenter::focused) else {
            return;
        };
        match command {
            DialogCommand::Back => self.filter_back(cx),
            DialogCommand::Submit => self.send_filter(FilterEvent::Apply, cx),
            DialogCommand::Select if focused.is_select() => self.activate_list(focused, cx),
            DialogCommand::Select => self.send_filter(FilterEvent::Apply, cx),
            DialogCommand::NextField | DialogCommand::PreviousField => {
                self.commit_list(cx);
                let event = if command == DialogCommand::NextField {
                    FilterEvent::NextField
                } else {
                    FilterEvent::PreviousField
                };
                self.send_filter(event, cx);
            }
            DialogCommand::SelectNext | DialogCommand::SelectPrevious => {
                let delta = if command == DialogCommand::SelectNext {
                    1
                } else {
                    -1
                };
                if let Some(view) = self.filter_view()
                    && let Some(list) = self.open_list(&view, focused)
                {
                    let options = select_labels(&view, focused).0;
                    list.move_highlight(&options, delta);
                    cx.notify();
                } else if delta > 0 {
                    self.send_filter(FilterEvent::SelectNext, cx);
                } else {
                    self.send_filter(FilterEvent::SelectPrevious, cx);
                }
            }
        }
    }

    /// A key the Dialog keymap doesn't bind: the focused field's own.
    fn filter_typed(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        let Some(focused) = self.state.filter.as_ref().map(FilterPresenter::focused) else {
            return false;
        };
        let modifiers = &keystroke.modifiers;
        if modifiers.control && keystroke.key == "r" {
            self.send_filter(FilterEvent::Reset, cx);
            return true;
        }
        let status = focused == FilterField::Status;
        match keystroke.key.as_str() {
            "left" if status => self.send_filter(FilterEvent::StatusPrevious, cx),
            "right" | "space" if status => self.send_filter(FilterEvent::StatusNext, cx),
            "space" if focused.is_select() => self.activate_list(focused, cx),
            "backspace" => self.send_filter(FilterEvent::Backspace, cx),
            _ => {
                if modifiers.control || modifiers.alt || modifiers.platform || modifiers.function {
                    return false;
                }
                if let Some(text) = keystroke.key_char.as_deref()
                    && text.chars().count() == 1
                    && let Some(ch) = text.chars().next()
                {
                    self.send_filter(FilterEvent::Type(ch), cx);
                }
            }
        }
        true
    }

    /// `back`: an open list closes first; otherwise the popover is cancelled, its draft discarded.
    fn filter_back(&mut self, cx: &mut Context<'_, Self>) {
        match self.state.filter_list.as_mut() {
            Some((_, list)) if list.is_open() => {
                list.cancel();
                cx.notify();
            }
            _ => self.send_filter(FilterEvent::Cancel, cx),
        }
    }

    /// `field`'s list, showing the Presenter's choice: kept while it still does, else a closed
    /// one.
    fn list(&mut self, view: &FilterViewModel, field: FilterField) -> &mut SelectState {
        let chosen = select_labels(view, field).1;
        let kept = self
            .state
            .filter_list
            .as_ref()
            .is_some_and(|(at, list)| *at == field && list.value() == chosen.as_deref());
        if !kept {
            self.state.filter_list = Some((field, SelectState::new(chosen)));
        }
        &mut self
            .state
            .filter_list
            .get_or_insert_with(|| (field, SelectState::default()))
            .1
    }

    /// `field`'s list, while it is open.
    fn open_list(
        &mut self,
        view: &FilterViewModel,
        field: FilterField,
    ) -> Option<&mut SelectState> {
        Some(self.list(view, field)).filter(|list| list.is_open())
    }

    /// `select` or `Space` on a select: opens its list, or commits an open list's highlight.
    fn activate_list(&mut self, field: FilterField, cx: &mut Context<'_, Self>) {
        let Some(view) = self.filter_view() else {
            return;
        };
        if self.open_list(&view, field).is_some() {
            self.commit_list(cx);
        } else {
            let options = select_labels(&view, field).0;
            self.list(&view, field).open(&options);
            cx.notify();
        }
    }

    /// Commits the focused select's open list, if any, sending the highlighted choice.
    fn commit_list(&mut self, cx: &mut Context<'_, Self>) {
        let Some(view) = self.filter_view() else {
            return;
        };
        let field = view.focused;
        let options = select_labels(&view, field).0;
        let Some(list) = self.open_list(&view, field) else {
            return;
        };
        list.commit(&options);
        let chosen = list.value().map(str::to_string);
        self.send_choice(&view, field, chosen, cx);
    }

    /// Sends the Presenter the choice labelled `label` on `field`'s select.
    fn send_choice(
        &mut self,
        view: &FilterViewModel,
        field: FilterField,
        label: Option<String>,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(label) = label else {
            return;
        };
        let event = match field {
            FilterField::Account => find(&view.account.options, &label).map(FilterEvent::Account),
            FilterField::Category => {
                find(&view.category.options, &label).map(FilterEvent::Category)
            }
            _ => None,
        };
        match event {
            Some(event) => self.send_filter(event, cx),
            None => cx.notify(),
        }
    }

    /// A click on a filter chip (or its `▾`): opens the popover on that chip's field.
    pub(super) fn click_chip(&mut self, field: ChipField, cx: &mut Context<'_, Self>) {
        self.open_filter_popover(Some(field), cx);
    }

    /// The `✕` on an accent chip: resets just that filter.
    pub(super) fn clear_chip(&mut self, field: ChipField, cx: &mut Context<'_, Self>) {
        FilterPresenter::clear_chip(&mut self.state.filters, field, self.today);
        self.reset_selection();
        cx.notify();
    }

    /// `clear filters`: every filter back to its default. The search text is separate state and is
    /// left alone.
    pub(super) fn clear_all(&mut self, cx: &mut Context<'_, Self>) {
        FilterPresenter::clear_all(&mut self.state.filters, self.today);
        self.reset_selection();
        cx.notify();
    }

    /// The header's **add transaction** button: the same message `n` gives.
    pub(super) fn click_add(&mut self, cx: &mut Context<'_, Self>) {
        cx.emit(ViewEvent::SetStatus(Some(
            crate::msg::desktop_status_add_transaction_not_yet_built(),
        )));
    }

    /// A click on the search box: the same as pressing `/`.
    pub(super) fn click_search(&mut self, cx: &mut Context<'_, Self>) {
        self.start_search(cx);
        cx.emit(ViewEvent::EnterMode(ViewMode::Search));
    }

    /// A click on a table row selects it.
    pub(super) fn click_row(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        self.state.selected = index;
        cx.notify();
    }

    /// A click on a popover field: a text field takes focus; a select takes focus and toggles its
    /// list.
    pub(super) fn click_filter_field(&mut self, field: FilterField, cx: &mut Context<'_, Self>) {
        self.send_filter(FilterEvent::Focus(field), cx);
        let Some(view) = self.filter_view() else {
            return;
        };
        if field.is_select() {
            let options = select_labels(&view, field).0;
            let list = self.list(&view, field);
            if list.is_open() {
                list.cancel();
            } else {
                list.open(&options);
            }
        }
        cx.notify();
    }

    /// A click on row `index` of a select's open list.
    pub(super) fn click_filter_option(
        &mut self,
        field: FilterField,
        index: usize,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(view) = self.filter_view() else {
            return;
        };
        let options = select_labels(&view, field).0;
        let list = self.list(&view, field);
        list.choose(&options, index);
        let chosen = list.value().map(str::to_string);
        self.send_choice(&view, field, chosen, cx);
    }

    pub(super) fn click_filter_status(&mut self, status: StatusFilter, cx: &mut Context<'_, Self>) {
        self.send_filter(FilterEvent::Status(status), cx);
    }

    /// `reset`: the draft back to the defaults; the applied filters are untouched.
    pub(super) fn reset_filter_form(&mut self, cx: &mut Context<'_, Self>) {
        self.send_filter(FilterEvent::Reset, cx);
    }

    /// **apply**: a no-op while a date does not parse.
    pub(super) fn apply_filter_form(&mut self, cx: &mut Context<'_, Self>) {
        self.send_filter(FilterEvent::Apply, cx);
    }
}

/// `field`'s option labels and its chosen option's label, as the list keys them; empty for a
/// field that isn't a select.
pub(super) fn select_labels(
    view: &FilterViewModel,
    field: FilterField,
) -> (Vec<String>, Option<String>) {
    match field {
        FilterField::Account => (labels(&view.account), chosen_label(&view.account)),
        FilterField::Category => (labels(&view.category), chosen_label(&view.category)),
        _ => (Vec::new(), None),
    }
}

/// The value of the option labelled `label`.
fn find<T: Clone>(options: &[lib_viper::view::Choice<T>], label: &str) -> Option<T> {
    options
        .iter()
        .find(|choice| choice.label == label)
        .map(|choice| choice.value.clone())
}
