//! The Transactions page's View: `lib-transactions`' page Presenter in a gpui Entity, which the
//! Shell hosts as the Transactions destination (`docs/ux/desktop-mockups/03-transactions/`). It
//! looks up each key the Shell passes on, as chord text, in the Transactions keymap, turns clicks,
//! Search keys, hand-offs and the popover's keys into events, and draws the view model (`mod.rs`,
//! `header.rs`, `table.rs`, `footer.rs`, `filter_popover.rs`). Each key and click is one input
//! (`crate::input::send`), which commits what it wrote; the page writes nothing yet.
//!
//! What stays here is gpui's: the table's scroll and the half page it measures by the row height,
//! where each chip was painted, and a popover select's open list and highlight, while the
//! Presenter holds the chosen option. The Display Preferences reach the Presenter as
//! [`TransactionsPageEvent::DisplayChanged`], so the glyphs and the density it draws by are in the
//! view model; only density to pixels is the bin's (`format::row_height_px`). The popover reads
//! its keys through the Shell's Dialog keymap, pushed here by `Root`. Search and the popover stay
//! modes in the bin (`InputMode::Search`, `InputMode::Filter`) until their own slice: the page
//! enters and leaves them through [`ViewEvent`] as the Presenter opens and closes them.
//!
//! The rows name Accounts, Categories, Payees, Tags and Units, read through the stores. A module
//! that writes any of them (an Import commit, a Tag merge) announces it through the `Announcer`, so
//! the page redraws on every `LedgerChanged` naming any of them.

use gpui::UniformListScrollHandle;
use gpui::{App, Context, Entity, EventEmitter, Keystroke, ScrollStrategy, Subscription};

use crate::announcer::Domain;
use lib_core::{Clock, DateStyle, TransactionId};
use lib_list_cursor::Step;
use lib_shell::DialogCommand;
use lib_transactions::SharedPorts;
use lib_transactions::presenter::{
    ChipField, FilterEvent, FilterField, FilterViewModel, TransactionsPageEvent,
    TransactionsPagePresenter, TransactionsPageViewModel,
};
use lib_transactions::query::TransactionFilters;
use lib_viper::keymap::Keymap;

use super::header::ChipBounds;
use crate::{
    announcer::{Announcer, LedgerChanged},
    form::select::SelectState,
    form::viper::{chosen_label, labels},
    navigation::{chord::chord, movement::Movement, nav::Noun},
    notices::Queue,
    shell::view::{page_clicks::PresenterPage, statusline::PageStatus},
    view::{
        event::{ViewEvent, ViewMode},
        format,
        help::HelpGroup,
    },
};
use lib_viper::display::{RowDensity, StatusGlyphs};

impl EventEmitter<ViewEvent> for TransactionsPage {}

/// The domains the rows read, any change to which redraws the page.
const READS: [Domain; 6] = [
    Domain::Accounts,
    Domain::Categories,
    Domain::Payees,
    Domain::Tags,
    Domain::Transactions,
    Domain::Units,
];

pub struct TransactionsPage {
    presenter: TransactionsPagePresenter,
    keymap: Keymap<TransactionsPageEvent>,
    /// The Shell's Dialog keymap, which the filter popover reads its keys through.
    dialog_keymap: Keymap<DialogCommand>,
    /// The table's `uniform_list` scroll state: scrolls the selected row into view and reports the
    /// viewport height for half-page moves.
    pub(super) scroll: UniformListScrollHandle,
    /// A popover select's open list and highlight, the View's widget state: the field it belongs
    /// to and the list, which shows the Presenter's choice.
    pub(super) filter_list: Option<(FilterField, SelectState)>,
    /// The chip that opened the popover, which it anchors under.
    pub(super) filter_anchor: ChipField,
    /// Where each chip was last painted; the header writes it, the popover reads it.
    pub(super) chip_bounds: ChipBounds,
    _ledger_changed: Subscription,
}

impl TransactionsPage {
    /// The page on `ports`, filtered to this year by `clock`'s today, under the default keys until
    /// `Root` pushes `[keybindings]`.
    pub fn new(
        ports: SharedPorts,
        announcer: &Entity<Announcer>,
        clock: Clock,
        cx: &mut Context<'_, Self>,
    ) -> Self {
        let config = lib_config::KeyBindingConfig::default();
        let keymap = lib_transactions::keymap::transactions_page_keymap(&config);
        let presenter = TransactionsPagePresenter::new(
            ports,
            clock,
            keymap.clone(),
            lib_transactions::keymap::transactions_filter_keymap(&config),
            search_chord(&config),
        );
        let _ledger_changed = cx.subscribe(announcer, |_page, _, event: &LedgerChanged, cx| {
            if READS.iter().any(|domain| event.what.contains(domain)) {
                cx.notify();
            }
        });
        Self {
            presenter,
            keymap,
            dialog_keymap: lib_shell::dialog_keymap(&config),
            scroll: UniformListScrollHandle::new(),
            filter_list: None,
            filter_anchor: ChipField::Account,
            chip_bounds: ChipBounds::default(),
            _ledger_changed,
        }
    }

    /// Follows a change to `[keybindings]`: the page's own keys, the Dialog keymap the popover
    /// reads, and the Shell's Search key the legend names.
    pub fn set_keymaps(&mut self, config: &lib_config::KeyBindingConfig) {
        self.keymap = lib_transactions::keymap::transactions_page_keymap(config);
        self.dialog_keymap = lib_shell::dialog_keymap(config);
        self.presenter.set_keymap(
            self.keymap.clone(),
            lib_transactions::keymap::transactions_filter_keymap(config),
            search_chord(config),
        );
    }

    /// The page as drawn.
    pub fn view_model(&self) -> TransactionsPageViewModel {
        self.presenter.view_model()
    }

    /// A key or click on the page, a hand-off, or the page being entered: `event` as one input,
    /// committed once it is handled.
    fn send(&mut self, event: TransactionsPageEvent, cx: &mut Context<'_, Self>) {
        crate::input::join_or_send(cx, |cx| self.handle(event, cx));
    }

    /// Hands `event` to the Presenter, then follows what it opened or closed: the popover's and
    /// the Search's modes, and the table's scroll. Inside an input already, as [`Self::send`].
    fn handle(&mut self, event: TransactionsPageEvent, cx: &mut Context<'_, Self>) {
        let filtering = self.presenter.is_filtering();
        let searching = self.presenter.is_searching();
        let selected = self.presenter.selected();
        if let TransactionsPageEvent::Move(_) = event {
            self.presenter.set_half_page(self.half_page());
        }
        let scroll = match &event {
            TransactionsPageEvent::Move(step) => Some(match step {
                Step::Next | Step::Last => ScrollStrategy::Bottom,
                Step::Prev | Step::First => ScrollStrategy::Top,
                Step::HalfPageDown | Step::HalfPageUp => ScrollStrategy::Center,
            }),
            _ => None,
        };
        let reveal = matches!(event, TransactionsPageEvent::RevealTransaction(_));
        self.presenter.handle(event, &mut Queue);

        // A popover opened over the Search takes its mode's place, so only the popover's is
        // entered: leaving the Search's too would drop the Shell to Normal under the popover.
        if !filtering && self.presenter.is_filtering() {
            self.filter_list = None;
            cx.emit(ViewEvent::EnterMode(ViewMode::Filter));
        } else if (filtering && !self.presenter.is_filtering())
            || (searching && !self.presenter.is_searching())
        {
            self.filter_list = None;
            cx.emit(ViewEvent::ExitMode);
        }
        // A non-strict scroll applies its strategy only when the row was off-screen, so a step
        // down lands the row at the bottom and a step up at the top; a reveal centres its row.
        let now = self.presenter.selected();
        if reveal {
            self.scroll
                .scroll_to_item_strict(now, ScrollStrategy::Center);
        } else if let Some(strategy) = scroll {
            self.scroll.scroll_to_item(now, strategy);
        } else if now != selected || now == 0 {
            self.scroll.scroll_to_item(now, ScrollStrategy::Top);
        }
        self.close_stale_list();
        cx.notify();
    }

    /// How many rows a half-page move steps, from the list's measured viewport height.
    fn half_page(&self) -> usize {
        let viewport = f32::from(self.scroll.0.borrow().base_handle.bounds().size.height);
        half_page_rows(viewport, format::row_height_px(self.presenter.density()))
    }

    /// Every chord the Transactions keymap binds, the arrows, `g g` and `enter` among them. The
    /// Shell has already taken its own keys.
    pub fn module_key(&mut self, chord: &str, cx: &mut Context<'_, Self>) -> bool {
        let Some(event) = self.keymap.event(chord) else {
            return false;
        };
        self.take(event, cx)
    }

    /// A palette Command, by its keymap `command` name: the event its key sends, the filter
    /// opening at Account as `f` opens it.
    pub fn module_command(&mut self, command: &str, cx: &mut Context<'_, Self>) -> bool {
        let Some(event) = self.keymap.event_for(command) else {
            return false;
        };
        self.take(event, cx)
    }

    /// The one dispatch a keymap event takes into the page, from a key or a palette Command: the
    /// filter opens at Account as `f` opens it.
    fn take(&mut self, event: TransactionsPageEvent, cx: &mut Context<'_, Self>) -> bool {
        if matches!(event, TransactionsPageEvent::OpenFilter(_)) {
            self.filter_anchor = ChipField::Account;
        }
        self.send(event, cx);
        true
    }

    /// Every key while the filter popover is open (the page entered `InputMode::Filter` for it),
    /// but `Esc`, which comes back as [`Self::escape`].
    pub fn key_ahead(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        if keystroke.key == "escape" || !self.presenter.is_filtering() {
            return false;
        }
        crate::input::send(cx, |cx| self.filter_key(keystroke, cx))
    }

    /// Every page key is the keymap's.
    pub fn handle_key(&mut self, _keystroke: &Keystroke, _cx: &mut Context<'_, Self>) -> bool {
        false
    }

    /// A movement the keymap doesn't bind does nothing on the page.
    pub fn apply_movement(&mut self, _movement: Movement, _cx: &mut Context<'_, Self>) {}

    /// `Esc`: the popover's open list first, then the popover itself (its draft discarded), then
    /// the Search (its text cleared).
    pub fn escape(&mut self, cx: &mut Context<'_, Self>) -> bool {
        if self.presenter.is_filtering() {
            crate::input::join_or_send(cx, |cx| self.filter_back(cx));
            return true;
        }
        if self.presenter.is_searching() {
            self.send(TransactionsPageEvent::ClearSearch, cx);
            return true;
        }
        false
    }

    /// `/` opens the Search box; `Root` enters `InputMode::Search` for it. Not an input of its own:
    /// the Shell's global keymap took `/` as one, which has committed.
    pub fn start_search(&mut self, cx: &mut Context<'_, Self>) -> bool {
        self.handle(TransactionsPageEvent::StartSearch, cx);
        true
    }

    /// Keys while the Search is open: typing filters live, `Backspace` edits, and `Enter` keeps the
    /// text and returns to browsing the rows. `Esc` comes back as [`Self::escape`].
    pub fn search_key(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        let event = match keystroke.key.as_str() {
            "enter" => TransactionsPageEvent::KeepSearch,
            "backspace" => TransactionsPageEvent::SearchBackspace,
            _ => match typed(keystroke) {
                Some(ch) => TransactionsPageEvent::SearchType(ch),
                None => return false,
            },
        };
        self.send(event, cx);
        true
    }

    /// The page has just become the destination. A Search left open when the page was left doesn't
    /// come back with it. Not an input of its own: it writes nothing, and the navigation that
    /// entered the page is the input (or follows the hand-off's, whose commit has landed).
    pub fn on_enter(&mut self, cx: &mut Context<'_, Self>) {
        self.handle(TransactionsPageEvent::Entered, cx);
    }

    /// Transaction `id` selected, the date range widened to reach it: [`router::show`]'s half on
    /// the page, which then asks the Shell to make the page the destination.
    ///
    /// [`router::show`]: crate::transactions::router::show
    pub(crate) fn show(&mut self, id: TransactionId, cx: &mut Context<'_, Self>) {
        self.hand_off(TransactionsPageEvent::RevealTransaction(id), cx);
    }

    /// The table filtered to `filters` from its first row, with no Search or draft left over:
    /// [`router::show_filtered`]'s half on the page.
    ///
    /// [`router::show_filtered`]: crate::transactions::router::show_filtered
    pub(crate) fn show_filtered(
        &mut self,
        filters: TransactionFilters,
        cx: &mut Context<'_, Self>,
    ) {
        self.hand_off(TransactionsPageEvent::ShowFiltered(filters), cx);
    }

    /// Hands `event` to the Presenter, then asks the Shell to show the page. Another module's
    /// input is already being sent, so the hand-off runs inside it and its one commit; one from a
    /// View that isn't a module yet is an input of its own.
    fn hand_off(&mut self, event: TransactionsPageEvent, cx: &mut Context<'_, Self>) {
        crate::input::join_or_send(cx, |cx| self.handle(event, cx));
        cx.emit(ViewEvent::Navigate(Noun::Transactions));
    }

    /// Puts the table back on its first row, after an Import commit lands on the page
    /// ([`router::first_row`]). Not an input of its own: it runs inside the commit's input, and a
    /// move writes nothing.
    ///
    /// [`router::first_row`]: crate::transactions::router::first_row
    pub(crate) fn reset_selection(&mut self, cx: &mut Context<'_, Self>) {
        self.handle(TransactionsPageEvent::Move(Step::First), cx);
    }

    /// Follows a change to the Display Preferences: the date style, and the glyphs and density
    /// the rows draw with, handed to the Presenter as one input.
    pub fn set_display(
        &mut self,
        date_style: Option<DateStyle>,
        glyphs: StatusGlyphs,
        density: RowDensity,
        cx: &mut Context<'_, Self>,
    ) {
        if density != self.presenter.density() {
            // The table's scroll offset is in pixels, so a new row height would leave it pointing
            // at a different row: re-anchor on the selected one for its next paint.
            self.scroll
                .scroll_to_item_strict(self.presenter.selected(), ScrollStrategy::Center);
        }
        crate::input::join_or_send(cx, |cx| {
            self.handle(TransactionsPageEvent::SetDateStyle(date_style), cx);
            self.handle(
                TransactionsPageEvent::DisplayChanged { glyphs, density },
                cx,
            );
        });
    }

    /// The status line's legend (the filter popover's or the page's), and its right-hand text.
    pub fn page_status(&self, _cx: &App) -> Option<PageStatus> {
        Some(PageStatus {
            hints: self.presenter.hints(&self.dialog_keymap),
            right: self.presenter.legend(),
        })
    }

    /// The `?` cheat-sheet's Transactions group: the keys the status line shows.
    pub fn help_group(&self, cx: &App) -> Option<HelpGroup> {
        let keys = self
            .page_status(cx)?
            .hints
            .into_iter()
            .map(|hint| (hint.action, hint.chord))
            .collect();
        Some(HelpGroup {
            heading: crate::msg::desktop_help_transactions_label(),
            keys,
        })
    }

    /// The open popover's view model.
    pub(super) fn filter_view(&self) -> Option<FilterViewModel> {
        self.presenter.filter().map(|filter| filter.view_model())
    }

    /// The focused popover field, while the popover is open.
    fn focused(&self) -> Option<FilterField> {
        self.presenter.filter().map(|filter| filter.focused())
    }

    /// An open list on a field that has lost focus closes.
    fn close_stale_list(&mut self) {
        let focused = self.focused();
        if self
            .filter_list
            .as_ref()
            .is_some_and(|(field, _)| Some(*field) != focused)
        {
            self.filter_list = None;
        }
    }

    fn filter(&mut self, event: FilterEvent, cx: &mut Context<'_, Self>) {
        self.handle(TransactionsPageEvent::Filter(event), cx);
    }

    /// Keys while the filter popover is open, read through the Dialog keymap: `next_field` and
    /// `previous_field` move between fields, `select_next` and `select_previous` step a closed
    /// select or move an open list's highlight, `select` opens or commits a select's list and
    /// otherwise applies, and `submit` applies from anywhere. Any other key is the field's own:
    /// `Space` on a select or the Status control, `Left` / `Right` on Status, and typing or
    /// `Backspace` in a text field. Between the two, the Presenter's filter keymap takes its own
    /// command (`transactions.reset_filters`).
    fn filter_key(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        let chord = chord(keystroke);
        // The filter steps no list on the list step's letters, so they type.
        if let Some(command) = chord
            .as_ref()
            .and_then(|c| self.dialog_keymap.event(c))
            .filter(|command| !command.is_list_step())
        {
            self.filter_command(command, cx);
            return true;
        }
        if let Some(chord) = &chord
            && self.presenter.filter_key(chord)
        {
            cx.notify();
            return true;
        }
        self.filter_typed(keystroke, cx)
    }

    fn filter_command(&mut self, command: DialogCommand, cx: &mut Context<'_, Self>) {
        let Some(focused) = self.focused() else {
            return;
        };
        match command {
            DialogCommand::Back => self.filter_back(cx),
            DialogCommand::Submit => self.filter(FilterEvent::Apply, cx),
            DialogCommand::Select if focused.is_select() => self.activate_list(focused, cx),
            DialogCommand::Select => self.filter(FilterEvent::Apply, cx),
            DialogCommand::NextField | DialogCommand::PreviousField => {
                self.commit_list(cx);
                let event = if command == DialogCommand::NextField {
                    FilterEvent::NextField
                } else {
                    FilterEvent::PreviousField
                };
                self.filter(event, cx);
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
                    self.filter(FilterEvent::SelectNext, cx);
                } else {
                    self.filter(FilterEvent::SelectPrevious, cx);
                }
            }
            // `filter_key` types the list step's letters instead.
            DialogCommand::ListNext | DialogCommand::ListPrevious => {}
        }
    }

    /// A key the Dialog keymap doesn't bind: the focused field's own.
    fn filter_typed(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        let Some(focused) = self.focused() else {
            return false;
        };
        let modifiers = &keystroke.modifiers;
        let status = focused == FilterField::Status;
        match keystroke.key.as_str() {
            "left" if status => self.filter(FilterEvent::StatusPrevious, cx),
            "right" | "space" if status => self.filter(FilterEvent::StatusNext, cx),
            "space" if focused.is_select() => self.activate_list(focused, cx),
            "backspace" => self.filter(FilterEvent::Backspace, cx),
            _ => {
                if modifiers.control || modifiers.alt || modifiers.platform || modifiers.function {
                    return false;
                }
                if let Some(text) = keystroke.key_char.as_deref()
                    && text.chars().count() == 1
                    && let Some(ch) = text.chars().next()
                {
                    self.filter(FilterEvent::Type(ch), cx);
                }
            }
        }
        true
    }

    /// `back`: an open list closes first; otherwise the popover is cancelled, its draft discarded.
    fn filter_back(&mut self, cx: &mut Context<'_, Self>) {
        match self.filter_list.as_mut() {
            Some((_, list)) if list.is_open() => {
                list.cancel();
                cx.notify();
            }
            _ => self.filter(FilterEvent::Cancel, cx),
        }
    }

    /// `field`'s list, showing the Presenter's choice: kept while it still does, else a closed
    /// one.
    fn list(&mut self, view: &FilterViewModel, field: FilterField) -> &mut SelectState {
        let chosen = select_labels(view, field).1;
        let kept = self
            .filter_list
            .as_ref()
            .is_some_and(|(at, list)| *at == field && list.value() == chosen.as_deref());
        if !kept {
            self.filter_list = Some((field, SelectState::new(chosen)));
        }
        &mut self
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
            Some(event) => self.filter(event, cx),
            None => cx.notify(),
        }
    }

    /// A click on a popover select's field opens its list, or closes it when it is open: the
    /// list is the View's, so the Presenter's focus event leaves it to the page.
    fn toggle_list(&mut self, field: FilterField) {
        let Some(view) = self.filter_view() else {
            return;
        };
        let options = select_labels(&view, field).0;
        let list = self.list(&view, field);
        if list.is_open() {
            list.cancel();
        } else {
            list.open(&options);
        }
    }
}

/// The page's clicks: its header, its rows and the filter popover. What a click leaves to the View
/// follows the event here: the chip a popover opened from anchors it, a click on the search box
/// enters the Search mode (a key's `Root` enters itself), and a select's field toggles its list
/// while a chosen option closes it.
impl PresenterPage<TransactionsPageEvent> for TransactionsPage {
    fn send(&mut self, event: TransactionsPageEvent, cx: &mut Context<'_, Self>) {
        if let TransactionsPageEvent::OpenFilter(Some(field)) = event {
            self.filter_anchor = field;
        }
        let start_search = matches!(event, TransactionsPageEvent::StartSearch);
        let focused_select = match &event {
            TransactionsPageEvent::Filter(FilterEvent::Focus(field)) if field.is_select() => {
                Some(*field)
            }
            _ => None,
        };
        let chosen = matches!(
            event,
            TransactionsPageEvent::Filter(FilterEvent::Account(_) | FilterEvent::Category(_))
        );
        crate::input::send(cx, |cx| {
            self.handle(event, cx);
            if let Some(field) = focused_select {
                self.toggle_list(field);
            }
            if chosen {
                self.filter_list = None;
            }
        });
        if start_search {
            cx.emit(ViewEvent::EnterMode(ViewMode::Search));
        }
    }
}

/// The Shell's chord for Search under `config`, which the page's legend names.
fn search_chord(config: &lib_config::KeyBindingConfig) -> String {
    lib_shell::global_keymap(config)
        .chord(lib_shell::keymap::SEARCH)
        .unwrap_or_default()
        .to_string()
}

/// A single printable character typed with no modifier but `Shift`.
fn typed(keystroke: &Keystroke) -> Option<char> {
    let modifiers = &keystroke.modifiers;
    if modifiers.control || modifiers.alt || modifiers.platform || modifiers.function {
        return None;
    }
    let text = keystroke.key_char.as_deref()?;
    let mut chars = text.chars();
    let ch = chars.next()?;
    (chars.next().is_none() && !ch.is_control()).then_some(ch)
}

/// How many rows a half-page move steps, from the list's measured viewport height. Before the first
/// paint the height is zero, so it falls back to a small fixed step rather than moving nowhere.
fn half_page_rows(viewport_px: f32, row_px: f32) -> usize {
    if viewport_px <= 0.0 || row_px <= 0.0 {
        return lib_list_cursor::HALF_PAGE;
    }
    (((viewport_px / row_px).floor() as usize) / 2).max(1)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_half_page_is_half_the_rows_that_fit_and_never_zero() {
        assert_eq!(half_page_rows(680.0, 34.0), 10);
        assert_eq!(half_page_rows(60.0, 34.0), 1);
        assert_eq!(half_page_rows(0.0, 34.0), 5, "before the first paint");
        assert_eq!(half_page_rows(680.0, 0.0), 5);
    }
}
