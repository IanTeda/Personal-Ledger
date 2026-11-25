//! Settings › Ledger's Presenter: the Ledger name and Owner edited in place, and the Financial year
//! starts and Base unit selects, the Ledger's own Preferences (ADR-0061), read through
//! [`LedgerReads`] and written through [`LedgerPreferences`] by the Interactors ([`RenameLedger`],
//! [`SetLedgerOwner`], [`SetFinancialYearStart`]), so it holds no copy of them and a
//! change made anywhere (an Undo among them) shows at once. The THIS LEDGER summary counts through
//! [`LedgerSummaries`] on every view model, caching nothing.
//!
//! Its keys are the decision on #1343: `j`/`k` walk the four fields, stopping at the ends. A text
//! field edits on `enter`, a click or a typed character, in Insert mode, where every key types but
//! `enter` (save), `esc` (discard), `tab` (save and move on) and `backspace`. A select opens on
//! `enter` or a click, `j`/`k` move its highlight without saving, and it saves as it collapses
//! (`enter`, a click on an option, a click away); `esc` collapses it unsaved. Leaving a field
//! however the page is left saves it. The Presenter, not the ports, refuses a blank or over-long
//! name, since [`LedgerPreferences`]' writes are unrefusable.

use std::convert::Infallible;
use std::rc::Rc;

use lib_core::{FinancialYearStart, LedgerName, LedgerNameError, Owner, UnitId};
use lib_list_cursor::Step;
use lib_toast::Notice;
use lib_viper::keymap::{KeyBindingConfig, Keymap, MOVE_DOWN, MOVE_UP, SELECT};
use lib_viper::presenter::{PageMode, PagePresenter, Presenter, TextKey};
use lib_viper::view::{Choice, Hint, SelectView, TextFieldView};

use lib_locale::format::format_count;

use crate::interactor::{RenameLedger, SetFinancialYearStart, SetLedgerOwner};
use crate::msg;
use crate::protocols::{
    BaseUnitAnswer, LedgerPreferences, LedgerReads, LedgerRouter, LedgerSummaries,
};

/// One of the page's fields, top to bottom.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedgerField {
    Name,
    Owner,
    FinancialYear,
    BaseUnit,
}

impl LedgerField {
    /// The fields in the `j`/`k` walk's order.
    pub const ALL: [Self; 4] = [Self::Name, Self::Owner, Self::FinancialYear, Self::BaseUnit];

    /// Whether the field is typed into rather than chosen from a list.
    fn is_text(self) -> bool {
        matches!(self, Self::Name | Self::Owner)
    }
}

/// What Settings › Ledger is told and the person asks of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedgerEvent {
    /// `j`/`k`: the field below or above, or an open list's highlight, stopping at the ends.
    Step(Step),
    /// `enter`: edits a text field, opens a select's list, or saves an open list's highlight.
    Open,
    /// A click on a field: edits a text field, opens a select's list or saves one open. Whatever
    /// another field had open is saved first.
    Press(LedgerField),
    /// A click on a Unit of the open Base unit list.
    ChooseBaseUnit(UnitId),
    /// A click on a month of the open Financial year starts list.
    ChooseMonth(FinancialYearStart),
    /// A click on the page away from the fields: saves what is open.
    ClickAway,
    /// `esc` with a field open: discards the draft, or collapses the list unsaved.
    Cancel,
    /// A printable character typed on a focused text field in Normal mode: starts its edit with
    /// the character as the draft's first new character.
    StartTyping(char),
    /// A character typed into the draft.
    Type(char),
    Backspace,
    /// `enter` or `tab` while editing: saves the draft, then on `tab` moves the focus by the step.
    Save(Option<Step>),
    /// Whether the page, not Settings' index, has the keyboard, told as the frame draws the page,
    /// which can be every draw: a value the Presenter already holds is not taken. Losing it saves
    /// what is open.
    Focused(bool),
}

/// What the Client hands Settings › Ledger as it builds it.
pub struct LedgerSetup {
    pub ledger: Rc<dyn LedgerReads>,
    pub summaries: Rc<dyn LedgerSummaries>,
    pub preferences: Rc<dyn LedgerPreferences>,
}

/// One Unit of the select.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseUnitOption {
    pub id: UnitId,
    pub code: String,
    /// `code — name`.
    pub label: String,
}

/// The Base unit select: its label, the Units in code order and the one checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseUnitView {
    pub label: String,
    pub options: Vec<BaseUnitOption>,
    pub chosen: Option<UnitId>,
    /// Whether the keyboard is on the select, which draws it in the accent.
    pub focused: bool,
    /// The open list's highlighted option, `None` while it is closed.
    pub highlight: Option<usize>,
}

/// The Financial year starts select: the twelve months, each labelled as the Locale writes it,
/// January first, the one chosen, and its open list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinancialYearView {
    pub select: SelectView<FinancialYearStart>,
    /// The open list's highlighted month, `None` while it is closed.
    pub highlight: Option<usize>,
}

/// A text field: what is stored, or the draft while it is being edited, its placeholder shown
/// while the text is empty and the field isn't being edited.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerTextView {
    pub field: TextFieldView,
    pub editing: bool,
    /// The last save of the draft was refused, which draws the field in the danger colour.
    pub refused: bool,
}

/// Settings › Ledger as the Client draws it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerViewModel {
    pub name: LedgerTextView,
    pub owner: LedgerTextView,
    pub financial_year: FinancialYearView,
    pub base_unit: BaseUnitView,
    pub summary_title: String,
    pub summary: Vec<(String, String)>,
    pub note: String,
    /// The page's keys, for the status line and the `?` cheat sheet.
    pub hints: Vec<Hint>,
}

impl LedgerViewModel {
    /// Whether a text field is being edited or a list is open, which `esc` closes.
    pub fn open(&self) -> bool {
        self.name.editing
            || self.owner.editing
            || self.financial_year.highlight.is_some()
            || self.base_unit.highlight.is_some()
    }
}

/// The text field being typed into.
#[derive(Debug, Clone, PartialEq, Eq)]
struct TextEdit {
    field: LedgerField,
    draft: String,
    refused: bool,
}

/// The select whose list is open, and its highlighted option.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OpenList {
    field: LedgerField,
    highlight: usize,
}

/// The page's keymaps: its own, and the editor's, which binds nothing so that every key types.
struct Keys {
    page: Keymap<LedgerEvent>,
    editing: Keymap<LedgerEvent>,
    back_chord: String,
}

impl Keys {
    /// The page's keymap and the back chord its legend shows under `config`, as
    /// [`LedgerPresenter::new`] and [`PagePresenter::rebind`] build them.
    fn new(config: &KeyBindingConfig) -> Self {
        let page = crate::keymap::ledger_keymap(config);
        Self {
            editing: page.scoped(|_| false),
            page,
            back_chord: crate::keymap::back_chord(config),
        }
    }
}

/// Settings › Ledger's Presenter.
pub struct LedgerPresenter {
    units: Rc<dyn LedgerReads>,
    summaries: Rc<dyn LedgerSummaries>,
    preferences: Rc<dyn LedgerPreferences>,
    keys: Keys,
    page_focused: bool,
    focus: LedgerField,
    edit: Option<TextEdit>,
    list: Option<OpenList>,
}

impl LedgerPresenter {
    pub fn new(setup: LedgerSetup, config: &KeyBindingConfig) -> Self {
        Self {
            units: setup.ledger,
            summaries: setup.summaries,
            preferences: setup.preferences,
            keys: Keys::new(config),
            page_focused: false,
            focus: LedgerField::Name,
            edit: None,
            list: None,
        }
    }

    /// Whether a text field is being typed into: the Client's Insert mode.
    pub fn editing(&self) -> bool {
        self.edit.is_some()
    }

    /// What `field` holds now, as its text field shows it.
    fn stored_text(&self, field: LedgerField) -> String {
        match field {
            LedgerField::Name => self.units.ledger_name().to_string(),
            LedgerField::Owner => self
                .units
                .ledger_owner()
                .map(|owner| owner.to_string())
                .unwrap_or_default(),
            LedgerField::FinancialYear | LedgerField::BaseUnit => String::new(),
        }
    }

    /// The active Units' ids, the Base unit list's options in order.
    fn unit_ids(&self) -> Vec<UnitId> {
        self.units
            .ledger_choices()
            .into_iter()
            .map(|unit| unit.id)
            .collect()
    }

    /// The chosen option's place in `field`'s list.
    fn chosen_index(&self, field: LedgerField) -> usize {
        match field {
            LedgerField::FinancialYear => {
                let start = self.units.ledger_financial_year_start();
                FinancialYearStart::ALL
                    .iter()
                    .position(|month| *month == start)
                    .unwrap_or(0)
            }
            LedgerField::BaseUnit => self
                .units
                .ledger_base_unit()
                .and_then(|base| self.unit_ids().iter().position(|id| *id == base))
                .unwrap_or(0),
            LedgerField::Name | LedgerField::Owner => 0,
        }
    }

    fn option_count(&self, field: LedgerField) -> usize {
        match field {
            LedgerField::FinancialYear => FinancialYearStart::ALL.len(),
            LedgerField::BaseUnit => self.unit_ids().len(),
            LedgerField::Name | LedgerField::Owner => 0,
        }
    }

    /// Opens the focused field: a text field's edit with `seed` after what is stored, or a
    /// select's list on its chosen option.
    fn open_field(&mut self, field: LedgerField, seed: Option<char>) {
        self.focus = field;
        if field.is_text() {
            let mut draft = self.stored_text(field);
            draft.extend(seed);
            self.edit = Some(TextEdit {
                field,
                draft,
                refused: false,
            });
        } else if self.option_count(field) > 0 {
            self.list = Some(OpenList {
                field,
                highlight: self.chosen_index(field),
            });
        }
    }

    /// Saves whatever is open as the field is left: a draft, or an open list's highlight.
    fn leave(&mut self, router: &mut dyn LedgerRouter) {
        if self.edit.is_some() {
            self.save(true, router);
        }
        if let Some(list) = self.list.take() {
            self.choose(list.field, list.highlight, router);
        }
    }

    /// Saves the draft, trimmed: text equal to what is stored writes nothing. A blank or
    /// over-long name is refused, keeping the draft open, unless the field is being left, which
    /// keeps the stored name instead. Returns whether the edit closed.
    fn save(&mut self, leaving: bool, router: &mut dyn LedgerRouter) -> bool {
        let Some(edit) = self.edit.take() else {
            return true;
        };
        match edit.field {
            LedgerField::Name => match LedgerName::new(&edit.draft) {
                Ok(name) => {
                    if name != self.units.ledger_name() {
                        let changed = changed(&msg::settings_ledger_name(), name.as_str());
                        RenameLedger { name }.run(self.preferences.as_ref());
                        router.notify(Notice::Changed(changed));
                    }
                }
                Err(error) => {
                    let refusal = match (error, leaving) {
                        (LedgerNameError::Blank, true) => msg::settings_ledger_name_kept(),
                        _ => lib_locale::format::ledger_name_error_message(error),
                    };
                    router.notify(Notice::Status(refusal));
                    if !leaving {
                        self.edit = Some(TextEdit {
                            refused: true,
                            ..edit
                        });
                        return false;
                    }
                }
            },
            LedgerField::Owner => {
                let owner = Owner::new(&edit.draft);
                if owner != self.units.ledger_owner() {
                    let changed = match &owner {
                        Some(owner) => changed(&msg::settings_ledger_owner(), owner.as_str()),
                        None => msg::settings_ledger_owner_cleared(),
                    };
                    SetLedgerOwner { owner }.run(self.preferences.as_ref());
                    router.notify(Notice::Changed(changed));
                }
            }
            LedgerField::FinancialYear | LedgerField::BaseUnit => {}
        }
        true
    }

    /// Saves option `index` of `field`'s list; the one chosen already writes nothing.
    fn choose(&mut self, field: LedgerField, index: usize, router: &mut dyn LedgerRouter) {
        match field {
            LedgerField::FinancialYear => {
                if let Some(start) = FinancialYearStart::ALL.get(index).copied() {
                    self.set_financial_year_start(start, router);
                }
            }
            LedgerField::BaseUnit => {
                if let Some(id) = self.unit_ids().get(index).copied() {
                    self.set_base_unit(id, router);
                }
            }
            LedgerField::Name | LedgerField::Owner => {}
        }
    }

    fn set_financial_year_start(
        &mut self,
        start: FinancialYearStart,
        router: &mut dyn LedgerRouter,
    ) {
        if self.units.ledger_financial_year_start() == start {
            return;
        }
        SetFinancialYearStart { start }.run(self.preferences.as_ref());
        router.notify(Notice::Changed(changed(
            &msg::settings_ledger_financial_year(),
            &lib_locale::format::format_month_name(start.month()),
        )));
    }

    /// A refusal means the Unit was removed or deactivated under the select, which repaints
    /// without it, so there is nothing to report.
    fn set_base_unit(&mut self, id: UnitId, router: &mut dyn LedgerRouter) {
        if self.units.ledger_base_unit() == Some(id) {
            return;
        }
        if self.preferences.set_base_unit(id) == BaseUnitAnswer::Saved {
            let code = self
                .units
                .ledger_choices()
                .into_iter()
                .find(|unit| unit.id == id)
                .map(|unit| unit.code)
                .unwrap_or_default();
            router.notify(Notice::Changed(msg::settings_changed(
                &msg::settings_ledger_base_unit(),
                &code,
            )));
            router.base_unit_changed();
        }
    }

    fn step_focus(&mut self, step: Step) {
        if let Some(next) = lib_list_cursor::step_id(&LedgerField::ALL, Some(self.focus), step) {
            self.focus = next;
        }
    }

    /// Whether the keyboard is on `field`, or it is the one open.
    fn has_focus(&self, field: LedgerField) -> bool {
        self.edit.as_ref().is_some_and(|edit| edit.field == field)
            || self.list.is_some_and(|list| list.field == field)
            || (self.page_focused && self.focus == field)
    }

    fn text_view(&self, field: LedgerField, label: String, placeholder: String) -> LedgerTextView {
        let edit = self.edit.as_ref().filter(|edit| edit.field == field);
        LedgerTextView {
            field: TextFieldView {
                label,
                text: edit.map_or_else(|| self.stored_text(field), |edit| edit.draft.clone()),
                placeholder,
                focused: self.has_focus(field),
            },
            editing: edit.is_some(),
            refused: edit.is_some_and(|edit| edit.refused),
        }
    }

    fn highlight(&self, field: LedgerField) -> Option<usize> {
        self.list
            .filter(|list| list.field == field)
            .map(|list| list.highlight)
    }

    /// The legend for what the keyboard is on: the editor's keys, an open list's, or the walk's
    /// and the focused field's.
    fn hints(&self) -> Vec<Hint> {
        let keymap = &self.keys.page;
        let back = || self.keys.back_chord.clone();
        if self.edit.is_some() {
            return vec![
                Hint::new(keymap.hint_chord(SELECT), msg::settings_hint_save()),
                Hint::new(back(), msg::settings_hint_cancel()),
                Hint::new("tab", msg::settings_hint_save_next()),
            ];
        }
        let walk = keymap.hint_chords(&[MOVE_DOWN, MOVE_UP]);
        if self.list.is_some() {
            return vec![
                Hint::new(walk, msg::settings_hint_choose()),
                Hint::new(keymap.hint_chord(SELECT), msg::settings_hint_pick()),
                Hint::new(back(), msg::settings_hint_close()),
            ];
        }
        let open = if self.focus.is_text() {
            msg::settings_hint_edit()
        } else {
            msg::settings_hint_list()
        };
        vec![
            Hint::new(walk, msg::settings_hint_field()),
            Hint::new(keymap.hint_chord(SELECT), open),
            Hint::new(back(), msg::settings_hint_index()),
        ]
    }
}

/// What History and the Changed Toast call a saved field.
fn changed(label: &str, value: &str) -> String {
    msg::settings_changed(label, value)
}

impl Presenter for LedgerPresenter {
    type Event = LedgerEvent;
    type ViewModel = LedgerViewModel;
    type Router<'r> = dyn LedgerRouter + 'r;

    fn handle(&mut self, event: LedgerEvent, router: &mut Self::Router<'_>) {
        if !self.takes(&event) {
            return;
        }
        match event {
            LedgerEvent::Step(step) => match self.list {
                Some(list) => {
                    let len = self.option_count(list.field);
                    self.list = Some(OpenList {
                        highlight: lib_list_cursor::step(list.highlight, len, step),
                        ..list
                    });
                }
                None => self.step_focus(step),
            },
            LedgerEvent::Open => match self.list.take() {
                Some(list) => self.choose(list.field, list.highlight, router),
                None => self.open_field(self.focus, None),
            },
            LedgerEvent::Press(field) => {
                let reopened = self.list.is_some_and(|list| list.field == field);
                if self.edit.as_ref().is_some_and(|edit| edit.field == field) {
                    return;
                }
                self.leave(router);
                self.focus = field;
                if !reopened {
                    self.open_field(field, None);
                }
            }
            LedgerEvent::ChooseBaseUnit(id) => {
                self.edit = None;
                self.list = None;
                self.focus = LedgerField::BaseUnit;
                self.set_base_unit(id, router);
            }
            LedgerEvent::ChooseMonth(start) => {
                self.edit = None;
                self.list = None;
                self.focus = LedgerField::FinancialYear;
                self.set_financial_year_start(start, router);
            }
            LedgerEvent::ClickAway => self.leave(router),
            LedgerEvent::Cancel => {
                self.edit = None;
                self.list = None;
            }
            LedgerEvent::StartTyping(ch) => self.open_field(self.focus, Some(ch)),
            LedgerEvent::Type(ch) => {
                if let Some(edit) = &mut self.edit {
                    edit.draft.push(ch);
                    edit.refused = false;
                }
            }
            LedgerEvent::Backspace => {
                if let Some(edit) = &mut self.edit {
                    edit.draft.pop();
                    edit.refused = false;
                }
            }
            LedgerEvent::Save(then) => {
                if self.save(false, router)
                    && let Some(step) = then
                {
                    self.step_focus(step);
                }
            }
            LedgerEvent::Focused(focused) => {
                if focused {
                    // Arriving puts the keyboard on the Ledger name, unless a click on a field
                    // that handed the page focus has just opened it.
                    if self.edit.is_none() && self.list.is_none() {
                        self.focus = LedgerField::Name;
                    }
                } else {
                    self.leave(router);
                }
                self.page_focused = focused;
            }
        }
    }

    fn view_model(&self) -> LedgerViewModel {
        LedgerViewModel {
            name: self.text_view(
                LedgerField::Name,
                msg::settings_ledger_name(),
                String::new(),
            ),
            owner: self.text_view(
                LedgerField::Owner,
                msg::settings_ledger_owner(),
                msg::settings_ledger_owner_none(),
            ),
            financial_year: FinancialYearView {
                select: SelectView {
                    label: msg::settings_ledger_financial_year(),
                    options: FinancialYearStart::ALL
                        .into_iter()
                        .map(|start| Choice {
                            value: start,
                            label: lib_locale::format::format_month_name(start.month()),
                        })
                        .collect(),
                    chosen: Some(self.units.ledger_financial_year_start()),
                    focused: self.has_focus(LedgerField::FinancialYear),
                },
                highlight: self.highlight(LedgerField::FinancialYear),
            },
            base_unit: BaseUnitView {
                label: msg::settings_ledger_base_unit(),
                options: self
                    .units
                    .ledger_choices()
                    .into_iter()
                    .map(|unit| BaseUnitOption {
                        label: msg::settings_ledger_base_unit_option(&unit.code, &unit.name),
                        id: unit.id,
                        code: unit.code,
                    })
                    .collect(),
                chosen: self.units.ledger_base_unit(),
                focused: self.has_focus(LedgerField::BaseUnit),
                highlight: self.highlight(LedgerField::BaseUnit),
            },
            summary_title: msg::settings_ledger_summary_title(),
            summary: {
                let counts = self.summaries.figures();
                vec![
                    (
                        msg::settings_ledger_summary_accounts(),
                        format_count(counts.accounts),
                    ),
                    (
                        msg::settings_ledger_summary_transactions(),
                        format_count(counts.transactions),
                    ),
                    (
                        msg::settings_ledger_summary_units(),
                        format_count(counts.units),
                    ),
                    (
                        msg::settings_ledger_summary_institutions(),
                        format_count(counts.institutions),
                    ),
                ]
            },
            note: msg::settings_ledger_note(),
            hints: self.hints(),
        }
    }

    /// Whether the Presenter takes `event` as things stand, so a key it declines goes on to the
    /// frame: the walk, `enter` and a typed start only while the page has the keyboard, the
    /// editor's keys only while editing, and `esc` only with something open. A focus it already
    /// holds is declined, so the page redraws only on a change.
    fn takes(&self, event: &LedgerEvent) -> bool {
        match event {
            LedgerEvent::Step(_) | LedgerEvent::Open => self.page_focused,
            LedgerEvent::StartTyping(_) => {
                self.page_focused
                    && self.edit.is_none()
                    && self.list.is_none()
                    && self.focus.is_text()
            }
            LedgerEvent::Type(_) | LedgerEvent::Backspace | LedgerEvent::Save(_) => {
                self.edit.is_some()
            }
            LedgerEvent::Cancel => self.edit.is_some() || self.list.is_some(),
            LedgerEvent::Press(_)
            | LedgerEvent::ChooseBaseUnit(_)
            | LedgerEvent::ChooseMonth(_)
            | LedgerEvent::ClickAway => true,
            LedgerEvent::Focused(focused) => *focused != self.page_focused,
        }
    }
}

impl PagePresenter for LedgerPresenter {
    /// Settings › Ledger opens no Dialog.
    type Outcome = Infallible;

    /// Settings › Ledger opens no Dialog, so there is nothing to tell it.
    fn dialog_closed(_outcome: Option<Infallible>) -> Option<LedgerEvent> {
        None
    }

    /// The page's keys; none while a text field is being typed into, so every key types.
    fn keymap(&self) -> &Keymap<LedgerEvent> {
        if self.editing() {
            &self.keys.editing
        } else {
            &self.keys.page
        }
    }

    fn bindings(&self) -> Vec<lib_viper::keymap::KeyBound<'_>> {
        self.keys.page.bindings().collect()
    }

    /// The keymaps and the `back` chord the legend shows.
    fn rebind(&mut self, keys: &KeyBindingConfig) {
        self.keys = Keys::new(keys);
    }

    /// The editor's keys: `enter` saves, `tab` saves and moves on (`shift+tab` back), and a
    /// character types.
    fn typed(&self, key: TextKey) -> Option<LedgerEvent> {
        if !self.editing() {
            return None;
        }
        Some(match key {
            TextKey::Char(ch) if !ch.is_control() => LedgerEvent::Type(ch),
            TextKey::Char(_) => return None,
            TextKey::Backspace => LedgerEvent::Backspace,
            TextKey::Enter { .. } => LedgerEvent::Save(None),
            TextKey::Tab { shift } => {
                LedgerEvent::Save(Some(if shift { Step::Prev } else { Step::Next }))
            }
        })
    }

    /// A character typed on a focused text field starts its edit.
    fn starts_typing(&self, ch: char) -> Option<LedgerEvent> {
        let event = LedgerEvent::StartTyping(ch);
        (!ch.is_control() && self.takes(&event)).then_some(event)
    }

    /// A text field being typed into is the page's Insert mode. An open list is not a mode: its
    /// keys are the page's own, as General's dropdowns are.
    fn mode(&self) -> Option<PageMode> {
        self.editing().then_some(PageMode::Insert)
    }
}

#[cfg(test)]
mod tests {
    use lib_viper::keymap::KeyBindingConfig;

    use super::*;
    use crate::fake::{FixedLedgerSummaries, InMemoryLedgerUnits, RecordingRouter, Routed};
    use crate::protocols::LedgerCounts;

    fn presenter(units: &InMemoryLedgerUnits) -> LedgerPresenter {
        lib_locale::init_with_layers("en-US", &[msg::LAYER]);
        let mut presenter = LedgerPresenter::new(
            LedgerSetup {
                ledger: Rc::new(units.clone()),
                summaries: Rc::new(FixedLedgerSummaries::sample()),
                preferences: Rc::new(units.clone()),
            },
            &KeyBindingConfig::default(),
        );
        presenter.handle(LedgerEvent::Focused(true), &mut RecordingRouter::default());
        presenter
    }

    #[test]
    fn the_summary_counts_afresh_and_groups_by_locale() {
        lib_locale::init_with_layers("en-US", &[msg::LAYER]);
        let units = InMemoryLedgerUnits::sample();
        let summaries = Rc::new(std::cell::RefCell::new(LedgerCounts::default()));
        struct Live(Rc<std::cell::RefCell<LedgerCounts>>);
        impl LedgerSummaries for Live {
            fn counts(&self) -> LedgerCounts {
                self.0.borrow().clone()
            }
        }
        let presenter = LedgerPresenter::new(
            LedgerSetup {
                ledger: Rc::new(units.clone()),
                summaries: Rc::new(Live(summaries.clone())),
                preferences: Rc::new(units),
            },
            &KeyBindingConfig::default(),
        );
        let values = |p: &LedgerPresenter| {
            p.view_model()
                .summary
                .into_iter()
                .map(|(_, v)| v)
                .collect::<Vec<_>>()
        };
        assert_eq!(values(&presenter), ["0", "0", "0", "0"]);
        *summaries.borrow_mut() = LedgerCounts {
            accounts: 8,
            transactions: 1680,
            units: 3,
            institutions: 7,
            ..LedgerCounts::default()
        };
        assert_eq!(values(&presenter), ["8", "1,680", "3", "7"]);
    }

    fn send(presenter: &mut LedgerPresenter, events: &[LedgerEvent]) -> Vec<Routed> {
        let mut router = RecordingRouter::default();
        for event in events {
            presenter.handle(*event, &mut router);
        }
        router.routed
    }

    fn typing(text: &str) -> Vec<LedgerEvent> {
        text.chars().map(LedgerEvent::Type).collect()
    }

    /// Clears the draft of the field being edited.
    fn clear(presenter: &mut LedgerPresenter) {
        let len = presenter
            .view_model()
            .name
            .field
            .text
            .chars()
            .count()
            .max(presenter.view_model().owner.field.text.chars().count());
        send(presenter, &vec![LedgerEvent::Backspace; len]);
    }

    fn base(presenter: &LedgerPresenter) -> Option<UnitId> {
        presenter.view_model().base_unit.chosen
    }

    fn focused(presenter: &LedgerPresenter) -> Vec<&'static str> {
        let view = presenter.view_model();
        [
            ("name", view.name.field.focused),
            ("owner", view.owner.field.focused),
            ("financial year", view.financial_year.select.focused),
            ("base unit", view.base_unit.focused),
        ]
        .into_iter()
        .filter(|(_, focused)| *focused)
        .map(|(name, _)| name)
        .collect()
    }

    fn changed(label: &str) -> Routed {
        Routed::Notice(Notice::Changed(label.to_string()))
    }

    fn status(text: &str) -> Routed {
        Routed::Notice(Notice::Status(text.to_string()))
    }

    #[test]
    fn arriving_focuses_the_name_and_j_k_walk_the_four_fields_stopping_at_the_ends() {
        let units = InMemoryLedgerUnits::sample();
        let mut presenter = presenter(&units);
        assert_eq!(focused(&presenter), ["name"]);

        send(&mut presenter, &[LedgerEvent::Step(Step::Prev)]);
        assert_eq!(focused(&presenter), ["name"]);
        send(&mut presenter, &[LedgerEvent::Step(Step::Next); 2]);
        assert_eq!(focused(&presenter), ["financial year"]);
        let routed = send(&mut presenter, &[LedgerEvent::Step(Step::Next); 3]);
        assert_eq!(focused(&presenter), ["base unit"]);
        assert!(routed.is_empty(), "walking writes nothing");
        assert_eq!(
            base(&presenter),
            Some(UnitId::SEED_AUD),
            "j/k no longer set it"
        );
    }

    #[test]
    fn enter_edits_the_name_in_insert_mode_and_enter_saves_it_trimmed() {
        let units = InMemoryLedgerUnits::sample();
        let mut presenter = presenter(&units);

        send(&mut presenter, &[LedgerEvent::Open]);
        assert_eq!(presenter.mode(), Some(PageMode::Insert));
        assert_eq!(presenter.view_model().name.field.text, "Sample Ledger");
        clear(&mut presenter);
        send(&mut presenter, &typing("  Teda household "));
        let routed = send(&mut presenter, &[LedgerEvent::Save(None)]);

        assert_eq!(routed, [changed("Ledger name changed to Teda household")]);
        assert_eq!(units.ledger_name().as_str(), "Teda household");
        assert_eq!(presenter.mode(), None);
        assert_eq!(presenter.view_model().name.field.text, "Teda household");
    }

    #[test]
    fn a_typed_character_starts_the_edit_after_the_stored_text() {
        let units = InMemoryLedgerUnits::sample();
        let mut presenter = presenter(&units);
        send(&mut presenter, &[LedgerEvent::Step(Step::Next)]);

        let event = presenter.starts_typing('!').expect("a text field takes it");
        send(&mut presenter, &[event, LedgerEvent::Save(None)]);

        assert_eq!(
            units.ledger_owner().map(|o| o.to_string()).as_deref(),
            Some("Sam Sample!")
        );
        send(&mut presenter, &[LedgerEvent::Step(Step::Next)]);
        assert_eq!(
            presenter.starts_typing('x'),
            None,
            "a select starts no edit"
        );
    }

    #[test]
    fn saving_what_is_stored_writes_nothing() {
        let units = InMemoryLedgerUnits::sample();
        let mut presenter = presenter(&units);

        let mut events = vec![
            LedgerEvent::Open,
            LedgerEvent::Type(' '),
            LedgerEvent::Save(None),
        ];
        events.extend([
            LedgerEvent::Step(Step::Next),
            LedgerEvent::Open,
            LedgerEvent::Save(None),
        ]);
        assert_eq!(send(&mut presenter, &events), []);
    }

    #[test]
    fn esc_discards_the_draft() {
        let units = InMemoryLedgerUnits::sample();
        let mut presenter = presenter(&units);

        send(&mut presenter, &[LedgerEvent::Open, LedgerEvent::Type('x')]);
        assert!(presenter.view_model().open());
        let routed = send(&mut presenter, &[LedgerEvent::Cancel]);

        assert!(routed.is_empty());
        assert_eq!(presenter.view_model().name.field.text, "Sample Ledger");
        assert!(!presenter.view_model().open());
        assert!(
            !presenter.takes(&LedgerEvent::Cancel),
            "esc goes on to the index"
        );
    }

    #[test]
    fn an_empty_owner_saves_as_owner_cleared() {
        let units = InMemoryLedgerUnits::sample();
        let mut presenter = presenter(&units);

        send(
            &mut presenter,
            &[LedgerEvent::Step(Step::Next), LedgerEvent::Open],
        );
        clear(&mut presenter);
        let routed = send(&mut presenter, &[LedgerEvent::Save(None)]);

        assert_eq!(routed, [changed("Owner cleared")]);
        assert_eq!(units.ledger_owner(), None);
        assert_eq!(presenter.view_model().owner.field.placeholder, "None");
    }

    #[test]
    fn a_blank_name_is_refused_on_enter_and_kept_on_leaving() {
        let units = InMemoryLedgerUnits::sample();
        let mut presenter = presenter(&units);
        send(&mut presenter, &[LedgerEvent::Open]);
        clear(&mut presenter);
        send(&mut presenter, &typing("  "));

        let routed = send(&mut presenter, &[LedgerEvent::Save(None)]);
        assert_eq!(routed, [status("A Ledger needs a name")]);
        let name = presenter.view_model().name;
        assert!(name.editing && name.refused);
        assert_eq!(name.field.text, "  ", "the draft stays");

        let routed = send(&mut presenter, &[LedgerEvent::ClickAway]);
        assert_eq!(routed, [status("Ledger name kept: it can't be blank")]);
        assert_eq!(units.ledger_name().as_str(), "Sample Ledger");
        assert!(!presenter.editing());
    }

    #[test]
    fn a_name_over_sixty_characters_is_refused_keeping_the_text() {
        let units = InMemoryLedgerUnits::sample();
        let mut presenter = presenter(&units);
        send(&mut presenter, &[LedgerEvent::Open]);
        clear(&mut presenter);
        send(&mut presenter, &typing(&"x".repeat(61)));

        let routed = send(&mut presenter, &[LedgerEvent::Save(None)]);

        assert_eq!(routed, [status("A ledger name is at most 60 characters")]);
        assert_eq!(presenter.view_model().name.field.text, "x".repeat(61));
        assert_eq!(units.ledger_name().as_str(), "Sample Ledger");
    }

    #[test]
    fn tab_saves_and_moves_on_and_leaving_the_page_saves() {
        let units = InMemoryLedgerUnits::sample();
        let mut presenter = presenter(&units);

        send(&mut presenter, &[LedgerEvent::Open, LedgerEvent::Type('!')]);
        let routed = send(&mut presenter, &[LedgerEvent::Save(Some(Step::Next))]);
        assert_eq!(routed, [changed("Ledger name changed to Sample Ledger!")]);
        assert_eq!(focused(&presenter), ["owner"]);

        send(&mut presenter, &[LedgerEvent::Open, LedgerEvent::Type('?')]);
        let routed = send(&mut presenter, &[LedgerEvent::Focused(false)]);
        assert_eq!(routed, [changed("Owner changed to Sam Sample?")]);
        assert!(!presenter.editing());
    }

    #[test]
    fn clicking_another_field_saves_the_draft_and_opens_that_field() {
        let units = InMemoryLedgerUnits::sample();
        let mut presenter = presenter(&units);

        send(&mut presenter, &[LedgerEvent::Press(LedgerField::Owner)]);
        assert!(presenter.view_model().owner.editing);
        send(&mut presenter, &[LedgerEvent::Type('!')]);
        let routed = send(&mut presenter, &[LedgerEvent::Press(LedgerField::BaseUnit)]);

        assert_eq!(routed, [changed("Owner changed to Sam Sample!")]);
        assert_eq!(presenter.view_model().base_unit.highlight, Some(0));
        assert_eq!(presenter.mode(), None);
    }

    #[test]
    fn the_financial_year_list_moves_without_saving_and_saves_as_it_collapses() {
        let units = InMemoryLedgerUnits::sample();
        let mut presenter = presenter(&units);
        send(&mut presenter, &[LedgerEvent::Step(Step::Next); 2]);

        send(&mut presenter, &[LedgerEvent::Open]);
        let view = presenter.view_model().financial_year;
        assert_eq!(view.highlight, Some(6), "the list opens on July");
        assert_eq!(view.select.options[0].label, "January");
        let routed = send(&mut presenter, &[LedgerEvent::Step(Step::Next); 3]);
        assert!(routed.is_empty());
        assert_eq!(
            units.ledger_financial_year_start(),
            FinancialYearStart::JULY
        );

        let routed = send(&mut presenter, &[LedgerEvent::Open]);
        assert_eq!(
            routed,
            [changed("Financial year starts changed to October")]
        );
        assert_eq!(units.ledger_financial_year_start().month(), 10);
        assert_eq!(presenter.view_model().financial_year.highlight, None);
    }

    #[test]
    fn esc_collapses_a_list_unsaved_and_a_click_away_saves_its_highlight() {
        let units = InMemoryLedgerUnits::sample();
        let mut presenter = presenter(&units);
        send(&mut presenter, &[LedgerEvent::Step(Step::Next); 2]);

        let routed = send(
            &mut presenter,
            &[
                LedgerEvent::Open,
                LedgerEvent::Step(Step::Prev),
                LedgerEvent::Cancel,
            ],
        );
        assert!(routed.is_empty());
        assert_eq!(
            units.ledger_financial_year_start(),
            FinancialYearStart::JULY
        );

        let routed = send(
            &mut presenter,
            &[
                LedgerEvent::Open,
                LedgerEvent::Step(Step::Prev),
                LedgerEvent::ClickAway,
            ],
        );
        assert_eq!(routed, [changed("Financial year starts changed to June")]);
    }

    #[test]
    fn a_clicked_month_saves_at_once() {
        let units = InMemoryLedgerUnits::sample();
        let mut presenter = presenter(&units);

        send(
            &mut presenter,
            &[LedgerEvent::Press(LedgerField::FinancialYear)],
        );
        let routed = send(
            &mut presenter,
            &[LedgerEvent::ChooseMonth(FinancialYearStart::APRIL)],
        );

        assert_eq!(routed, [changed("Financial year starts changed to April")]);
        assert_eq!(
            presenter.view_model().financial_year.select.chosen,
            Some(FinancialYearStart::APRIL)
        );
    }

    #[test]
    fn the_select_lists_the_active_units_and_checks_the_base() {
        let units = InMemoryLedgerUnits::sample();
        units.deactivate(UnitId::SEED_VAS);

        let view = presenter(&units).view_model().base_unit;

        let codes: Vec<_> = view.options.iter().map(|o| o.code.as_str()).collect();
        assert_eq!(codes, ["AUD", "BTC"]);
        assert_eq!(view.chosen, Some(UnitId::SEED_AUD));
        assert!(view.options[0].label.starts_with("AUD — "));
    }

    #[test]
    fn the_base_unit_list_saves_its_highlight_as_it_collapses() {
        let units = InMemoryLedgerUnits::sample();
        let mut presenter = presenter(&units);
        send(&mut presenter, &[LedgerEvent::Step(Step::Next); 3]);

        let routed = send(
            &mut presenter,
            &[
                LedgerEvent::Open,
                LedgerEvent::Step(Step::Next),
                LedgerEvent::Step(Step::Next),
                LedgerEvent::Step(Step::Next),
                LedgerEvent::Open,
            ],
        );

        assert_eq!(base(&presenter), Some(UnitId::SEED_VAS));
        assert_eq!(
            routed,
            [changed("Base unit changed to VAS"), Routed::BaseUnitChanged]
        );
    }

    #[test]
    fn a_focus_it_already_has_is_not_taken() {
        let units = InMemoryLedgerUnits::sample();
        let mut presenter = presenter(&units);

        assert!(!presenter.takes(&LedgerEvent::Focused(true)));
        assert!(presenter.takes(&LedgerEvent::Focused(false)));
        presenter.handle(LedgerEvent::Focused(false), &mut RecordingRouter::default());
        assert_eq!(focused(&presenter), Vec::<&str>::new());
    }

    #[test]
    fn a_step_is_left_to_the_frame_while_the_index_has_the_keyboard() {
        let units = InMemoryLedgerUnits::sample();
        let mut presenter = presenter(&units);
        presenter.handle(LedgerEvent::Focused(false), &mut RecordingRouter::default());

        assert!(!presenter.takes(&LedgerEvent::Step(Step::Next)));
        assert!(!presenter.takes(&LedgerEvent::Open));
        assert_eq!(presenter.starts_typing('x'), None);
    }

    #[test]
    fn clicking_a_unit_makes_it_the_base_and_a_gone_unit_is_ignored() {
        let units = InMemoryLedgerUnits::sample();
        let mut presenter = presenter(&units);

        let mut routed = send(
            &mut presenter,
            &[LedgerEvent::ChooseBaseUnit(UnitId::SEED_BTC)],
        );
        assert_eq!(base(&presenter), Some(UnitId::SEED_BTC));
        routed.extend(send(
            &mut presenter,
            &[LedgerEvent::ChooseBaseUnit(UnitId::new())],
        ));
        assert_eq!(base(&presenter), Some(UnitId::SEED_BTC));
        assert_eq!(
            routed,
            [changed("Base unit changed to BTC"), Routed::BaseUnitChanged]
        );
    }

    #[test]
    fn while_editing_every_key_types_through_typed() {
        let units = InMemoryLedgerUnits::sample();
        let mut presenter = presenter(&units);
        assert_eq!(
            presenter.typed(TextKey::Char('u')),
            None,
            "Normal mode types nothing"
        );

        send(&mut presenter, &[LedgerEvent::Open]);
        assert_eq!(
            presenter.keymap().event("j"),
            None,
            "no chord is the page's"
        );
        assert_eq!(
            presenter.typed(TextKey::Char('u')),
            Some(LedgerEvent::Type('u'))
        );
        assert_eq!(
            presenter.typed(TextKey::Tab { shift: false }),
            Some(LedgerEvent::Save(Some(Step::Next)))
        );
    }

    #[test]
    fn the_legend_follows_what_the_keyboard_is_on() {
        let units = InMemoryLedgerUnits::sample();
        let mut presenter = presenter(&units);
        let legend = |presenter: &LedgerPresenter| -> Vec<(String, String)> {
            presenter
                .view_model()
                .hints
                .into_iter()
                .map(|hint| (hint.chord, hint.action))
                .collect()
        };
        let pair = |chord: &str, action: &str| (chord.to_string(), action.to_string());

        assert_eq!(
            legend(&presenter),
            [
                pair("j/k", "field"),
                pair("enter", "edit"),
                pair("esc", "index")
            ]
        );
        send(&mut presenter, &[LedgerEvent::Open]);
        assert_eq!(
            legend(&presenter),
            [
                pair("enter", "save"),
                pair("esc", "cancel"),
                pair("tab", "save · next")
            ]
        );
        send(
            &mut presenter,
            &[
                LedgerEvent::Cancel,
                LedgerEvent::Step(Step::Last),
                LedgerEvent::Open,
            ],
        );
        assert_eq!(
            legend(&presenter),
            [
                pair("j/k", "choose"),
                pair("enter", "pick"),
                pair("esc", "close")
            ]
        );
    }

    #[test]
    fn the_view_reads_the_figures_and_never_the_counts() {
        lib_locale::init_with_layers("en-US", &[msg::LAYER]);
        struct FiguresOnly;
        impl LedgerSummaries for FiguresOnly {
            fn counts(&self) -> LedgerCounts {
                panic!("the Ledger page read the full counts");
            }

            fn figures(&self) -> crate::LedgerFigures {
                crate::LedgerFigures {
                    accounts: 1,
                    transactions: 2,
                    units: 3,
                    institutions: 4,
                }
            }
        }
        let presenter = LedgerPresenter::new(
            LedgerSetup {
                ledger: Rc::new(InMemoryLedgerUnits::sample()),
                summaries: Rc::new(FiguresOnly),
                preferences: Rc::new(InMemoryLedgerUnits::sample()),
            },
            &KeyBindingConfig::default(),
        );
        let values: Vec<_> = presenter
            .view_model()
            .summary
            .into_iter()
            .map(|(_, v)| v)
            .collect();
        assert_eq!(values, ["1", "2", "3", "4"]);
    }
}
