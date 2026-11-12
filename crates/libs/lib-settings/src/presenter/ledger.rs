//! Settings › Ledger's Presenter: the Base unit select, the Ledger's one Preference here, read
//! through [`LedgerUnits`] and written through [`LedgerPreferences`], so it holds no Units of its
//! own and a change made anywhere shows at once. The name, owner and financial-year fields and the
//! THIS LEDGER summary are the mockup's placeholder values until those Preferences persist.

use std::convert::Infallible;
use std::rc::Rc;

use lib_core::UnitId;
use lib_list_cursor::Step;
use lib_viper::keymap::{Keymap, MOVE_DOWN, MOVE_UP};
use lib_viper::presenter::{PagePresenter, Presenter};
use lib_viper::view::Hint;

use crate::msg;
use crate::protocols::{BaseUnitAnswer, LedgerPreferences, LedgerRouter, LedgerUnits};

/// What Settings › Ledger is told and the person asks of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedgerEvent {
    /// `j`/`k`: the Unit after or before the base, stopping at the ends.
    Step(Step),
    /// A click on a Unit of the select.
    ChooseBaseUnit(UnitId),
}

/// What the Client hands Settings › Ledger as it builds it.
pub struct LedgerSetup {
    pub units: Rc<dyn LedgerUnits>,
    pub preferences: Rc<dyn LedgerPreferences>,
    pub keymap: Keymap<LedgerEvent>,
    /// The chord of the Shell's `back`, for the legend.
    pub back_chord: String,
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
    /// Whether the keyboard is on the page, which draws the select in the accent.
    pub focused: bool,
}

/// Settings › Ledger as the Client draws it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerViewModel {
    /// `(label, value)` of the placeholder identity fields, top to bottom.
    pub fields: Vec<(String, String)>,
    /// Whether the last field (Financial year starts) is drawn as a select.
    pub financial_year_select: bool,
    pub base_unit: BaseUnitView,
    pub summary_title: String,
    pub summary: Vec<(String, String)>,
    pub note: String,
    /// The page's keys, for the status line and the `?` cheat sheet.
    pub hints: Vec<Hint>,
}

/// Settings › Ledger's Presenter.
pub struct LedgerPresenter {
    units: Rc<dyn LedgerUnits>,
    preferences: Rc<dyn LedgerPreferences>,
    keymap: Keymap<LedgerEvent>,
    back_chord: String,
    page_focused: bool,
}

impl LedgerPresenter {
    pub fn new(setup: LedgerSetup) -> Self {
        Self {
            units: setup.units,
            preferences: setup.preferences,
            keymap: setup.keymap,
            back_chord: setup.back_chord,
            page_focused: false,
        }
    }

    /// Replaces the keymap and the `back` chord, for a `[keybindings]` change.
    pub fn set_keymap(&mut self, keymap: Keymap<LedgerEvent>, back_chord: String) {
        self.keymap = keymap;
        self.back_chord = back_chord;
    }

    /// Whether the page, not Settings' index, has the keyboard. Answers whether that changed, so
    /// the View redraws only then: the frame tells the page on every draw.
    pub fn set_page_focused(&mut self, focused: bool) -> bool {
        let changed = focused != self.page_focused;
        self.page_focused = focused;
        changed
    }

    /// A refusal means the Unit was removed or deactivated under the select, which repaints
    /// without it, so there is nothing to report.
    fn set_base_unit(&mut self, id: UnitId, router: &mut dyn LedgerRouter) {
        if self.units.base_unit() == Some(id) {
            return;
        }
        if self.preferences.set_base_unit(id) == BaseUnitAnswer::Saved {
            router.base_unit_changed();
        }
    }
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
            LedgerEvent::Step(step) => {
                let Some(base) = self.units.base_unit() else {
                    return;
                };
                let ids: Vec<UnitId> = self.units.choices().into_iter().map(|c| c.id).collect();
                // A Base unit that isn't a choice stays where it is.
                if !ids.contains(&base) {
                    return;
                }
                if let Some(next) = lib_list_cursor::step_id(&ids, Some(base), step) {
                    self.set_base_unit(next, router);
                }
            }
            LedgerEvent::ChooseBaseUnit(id) => self.set_base_unit(id, router),
        }
    }

    fn view_model(&self) -> LedgerViewModel {
        LedgerViewModel {
            fields: vec![
                (msg::settings_ledger_name(), "Personal Ledger".to_string()),
                (msg::settings_ledger_owner(), "alex@teda.id.au".to_string()),
                (msg::settings_ledger_financial_year(), "july".to_string()),
            ],
            financial_year_select: true,
            base_unit: BaseUnitView {
                label: msg::settings_ledger_base_unit(),
                options: self
                    .units
                    .choices()
                    .into_iter()
                    .map(|unit| BaseUnitOption {
                        label: msg::settings_ledger_base_unit_option(&unit.code, &unit.name),
                        id: unit.id,
                        code: unit.code,
                    })
                    .collect(),
                chosen: self.units.base_unit(),
                focused: self.page_focused,
            },
            summary_title: msg::settings_ledger_summary_title(),
            summary: vec![
                (msg::settings_ledger_summary_accounts(), "7".to_string()),
                (
                    msg::settings_ledger_summary_transactions(),
                    "680".to_string(),
                ),
                (msg::settings_ledger_summary_units(), "3".to_string()),
                (msg::settings_ledger_summary_institutions(), "7".to_string()),
            ],
            note: msg::settings_ledger_note(),
            hints: vec![
                Hint::new(
                    self.keymap.hint_chords(&[MOVE_DOWN, MOVE_UP]),
                    msg::settings_hint_change(),
                ),
                Hint::new(self.back_chord.clone(), msg::settings_hint_index()),
            ],
        }
    }

    /// Whether the Presenter takes `event` as things stand, so a key it declines goes on to the
    /// frame: a step only while the page has the keyboard.
    fn takes(&self, event: &LedgerEvent) -> bool {
        match event {
            LedgerEvent::Step(_) => self.page_focused,
            LedgerEvent::ChooseBaseUnit(_) => true,
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

    fn keymap(&self) -> &Keymap<LedgerEvent> {
        &self.keymap
    }
}

#[cfg(test)]
mod tests {
    use lib_viper::keymap::KeyBindingConfig;

    use super::*;
    use crate::fake::{InMemoryLedgerUnits, RecordingRouter, Routed};
    use crate::keymap::ledger_keymap;

    fn presenter(units: &InMemoryLedgerUnits) -> LedgerPresenter {
        let mut presenter = LedgerPresenter::new(LedgerSetup {
            units: Rc::new(units.clone()),
            preferences: Rc::new(units.clone()),
            keymap: ledger_keymap(&KeyBindingConfig::default()),
            back_chord: "esc".to_string(),
        });
        presenter.set_page_focused(true);
        presenter
    }

    fn base(presenter: &LedgerPresenter) -> Option<UnitId> {
        presenter.view_model().base_unit.chosen
    }

    #[test]
    fn the_select_lists_the_active_units_and_checks_the_base() {
        let units = InMemoryLedgerUnits::sample();
        units.deactivate(UnitId::SEED_VAS);

        let view = presenter(&units).view_model().base_unit;

        let codes: Vec<_> = view.options.iter().map(|o| o.code.as_str()).collect();
        assert_eq!(codes, ["aud", "btc"]);
        assert_eq!(view.chosen, Some(UnitId::SEED_AUD));
        assert!(view.options[0].label.starts_with("aud — "));
    }

    #[test]
    fn steps_set_the_base_and_stop_at_the_ends() {
        let units = InMemoryLedgerUnits::sample();
        let mut presenter = presenter(&units);
        let mut router = RecordingRouter::default();

        presenter.handle(LedgerEvent::Step(Step::Prev), &mut router);
        assert_eq!(base(&presenter), Some(UnitId::SEED_AUD));
        assert!(router.routed.is_empty(), "no write at the end");

        presenter.handle(LedgerEvent::Step(Step::Next), &mut router);
        presenter.handle(LedgerEvent::Step(Step::Next), &mut router);
        presenter.handle(LedgerEvent::Step(Step::Next), &mut router);
        assert_eq!(base(&presenter), Some(UnitId::SEED_VAS));
        assert_eq!(
            router.routed,
            [Routed::BaseUnitChanged, Routed::BaseUnitChanged]
        );
    }

    #[test]
    fn a_step_is_left_to_the_frame_while_the_index_has_the_keyboard() {
        let units = InMemoryLedgerUnits::sample();
        let mut presenter = presenter(&units);
        presenter.set_page_focused(false);
        let mut router = RecordingRouter::default();

        assert!(!presenter.takes(&LedgerEvent::Step(Step::Next)));
        presenter.handle(LedgerEvent::Step(Step::Next), &mut router);

        assert_eq!(base(&presenter), Some(UnitId::SEED_AUD));
    }

    #[test]
    fn clicking_a_unit_makes_it_the_base_and_a_gone_unit_is_ignored() {
        let units = InMemoryLedgerUnits::sample();
        let mut presenter = presenter(&units);
        let mut router = RecordingRouter::default();

        presenter.handle(LedgerEvent::ChooseBaseUnit(UnitId::SEED_BTC), &mut router);
        assert_eq!(base(&presenter), Some(UnitId::SEED_BTC));

        presenter.handle(LedgerEvent::ChooseBaseUnit(UnitId::new()), &mut router);
        assert_eq!(base(&presenter), Some(UnitId::SEED_BTC));
        assert_eq!(router.routed, [Routed::BaseUnitChanged]);
    }

    #[test]
    fn the_legend_names_the_steps_and_the_back_chord() {
        let units = InMemoryLedgerUnits::sample();
        let hints = presenter(&units).view_model().hints;

        let chords: Vec<_> = hints.iter().map(|h| h.chord.as_str()).collect();
        assert_eq!(chords, ["j/k", "esc"]);
    }
}
