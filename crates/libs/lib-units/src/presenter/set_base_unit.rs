//! The Set base unit Dialog: a select over the active Units, by code, on the base Unit to start.

use lib_core::UnitId;
use lib_viper::dialog::{DialogInput, DialogRouter};
use lib_viper::view::{Choice, DialogFrame, SelectList, SelectMove, SelectView};

use super::{UnitChange, UnitsOutcome};
use crate::interactor::SetBaseUnit;
use crate::protocols::SharedPorts;
use crate::{Error, msg, units_by_code};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetBaseUnitEvent {
    /// A Dialog command or typed key, as the Desktop sends them: `select` commits the open list,
    /// else confirms, and `submit` only confirms; `Space` and the arrows open and step the select;
    /// typing and the field keys go nowhere.
    Input(DialogInput),
    /// The Unit picked, closing the list.
    Choose(UnitId),
    /// A click on the select's field: opens or closes its list.
    ToggleList,
    /// Closes the open list without picking: the Desktop host's `back`.
    CloseList,
    Confirm,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetBaseUnitViewModel {
    pub frame: DialogFrame,
    pub base: SelectView<UnitId>,
    /// The select's list: open, and its highlight.
    pub list: SelectList,
    pub refusal: Option<String>,
}

pub struct SetBaseUnitPresenter {
    ports: SharedPorts,
    options: Vec<Choice<UnitId>>,
    chosen: Option<UnitId>,
    list: SelectList,
    refusal: Option<String>,
}

impl SetBaseUnitPresenter {
    pub fn new(ports: SharedPorts) -> Self {
        let (options, chosen) = {
            let ports = ports.borrow();
            let units = ports.units().units();
            // The pickers offer only active Units, as deactivating one hides it from them.
            let options = units_by_code(&units)
                .into_iter()
                .filter(|unit| unit.is_active)
                .map(|unit| Choice {
                    value: unit.id,
                    label: msg::units_base_unit_option(&unit.code, &unit.name),
                })
                .collect::<Vec<_>>();
            let base = ports.base_unit().base_unit();
            (options, Some(base))
        };
        Self {
            ports,
            options,
            chosen,
            list: SelectList::default(),
            refusal: None,
        }
    }

    pub fn handle(&mut self, event: SetBaseUnitEvent, router: &mut dyn DialogRouter<UnitsOutcome>) {
        match event {
            SetBaseUnitEvent::Input(input) => self.input(input, router),
            SetBaseUnitEvent::Choose(id) => {
                self.list.close();
                self.choose(id);
            }
            SetBaseUnitEvent::ToggleList => self.list.toggle(&self.options, self.chosen.as_ref()),
            SetBaseUnitEvent::CloseList => {
                self.list.close();
            }
            SetBaseUnitEvent::Confirm => self.confirm(router),
            SetBaseUnitEvent::Cancel => router.close(None),
        }
    }

    /// Reads `input`: the select is the only field.
    fn input(&mut self, input: DialogInput, router: &mut dyn DialogRouter<UnitsOutcome>) {
        match input {
            DialogInput::Submit => self.confirm(router),
            DialogInput::Select if !self.list.is_open() => self.confirm(router),
            _ => {
                if let SelectMove::Chose(id) =
                    self.list.input(input, &self.options, self.chosen.as_ref())
                {
                    self.choose(id);
                }
            }
        }
    }

    fn choose(&mut self, id: UnitId) {
        if self.options.iter().any(|choice| choice.value == id) {
            self.chosen = Some(id);
        }
    }

    pub fn view_model(&self) -> SetBaseUnitViewModel {
        SetBaseUnitViewModel {
            frame: DialogFrame {
                title: msg::units_set_base_title(),
                submit: msg::units_set_base_submit(),
                can_confirm: self.chosen.is_some(),
                destructive: false,
                hints: vec![super::choose_hint(), super::cancel_hint()],
            },
            base: SelectView {
                label: msg::units_set_base_label(),
                options: self.options.clone(),
                chosen: self.chosen,
                focused: true,
            },
            list: self.list,
            refusal: self.refusal.clone(),
        }
    }

    fn confirm(&mut self, router: &mut dyn DialogRouter<UnitsOutcome>) {
        let Some(id) = self.chosen else {
            return;
        };
        // Read before the write: a gone Unit is refused, so a set base always has its code.
        let (code, set) = {
            let mut ports = self.ports.borrow_mut();
            let ports = ports.split_mut();
            let code = ports.units.unit(id).map(|unit| unit.code);
            (code, SetBaseUnit(id).run(ports.units, ports.base_unit))
        };
        self.refusal = match (set, code) {
            (Ok(()), Some(code)) => {
                router.close(Some(UnitsOutcome::Unit {
                    change: UnitChange::BaseSet,
                    code,
                }));
                None
            }
            (Err(Error::InactiveBase), _) => Some(msg::units_refusal_inactive_base()),
            (Err(Error::UnitGone), _) => Some(msg::units_refusal_unit_gone()),
            (Err(refusal), _) => {
                debug_assert!(false, "SetBaseUnit doesn't return {refusal:?}");
                Some(msg::units_refusal_unit_gone())
            }
            (Ok(()), None) => Some(msg::units_refusal_unit_gone()),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{hints, pairs, sample};
    use super::*;
    use crate::fake::{RecordingRouter, RouterCall};

    fn send(dialog: &mut SetBaseUnitPresenter, events: Vec<SetBaseUnitEvent>) -> Vec<RouterCall> {
        let mut router = RecordingRouter::default();
        for event in events {
            dialog.handle(event, &mut router);
        }
        router.calls
    }

    #[test]
    fn the_select_offers_the_active_units_by_code_starting_on_the_base() {
        let (ports, shared) = sample();
        ports.borrow_mut().units.units[2].is_active = false;
        let view = SetBaseUnitPresenter::new(shared).view_model();

        let labels: Vec<_> = view.base.options.iter().map(|o| o.label.as_str()).collect();
        assert_eq!(labels, ["aud — Australian Dollar", "btc — Bitcoin"]);
        assert_eq!(view.base.chosen, Some(UnitId::SEED_AUD));
    }

    #[test]
    fn confirming_moves_the_base_and_closes_with_its_code() {
        let (ports, shared) = sample();
        let mut dialog = SetBaseUnitPresenter::new(shared);

        let calls = send(
            &mut dialog,
            vec![
                SetBaseUnitEvent::Choose(UnitId::SEED_BTC),
                SetBaseUnitEvent::Confirm,
            ],
        );

        assert_eq!(
            calls,
            [RouterCall::CloseDialog(Some(UnitsOutcome::Unit {
                change: UnitChange::BaseSet,
                code: "btc".into()
            }))]
        );
        assert_eq!(ports.borrow().base_unit.0, UnitId::SEED_BTC);
    }

    #[test]
    fn a_unit_deactivated_or_removed_under_the_dialog_is_refused_as_text() {
        let (ports, shared) = sample();
        let mut dialog = SetBaseUnitPresenter::new(shared);
        send(
            &mut dialog,
            vec![SetBaseUnitEvent::Choose(UnitId::SEED_BTC)],
        );
        ports.borrow_mut().units.units[1].is_active = false;

        assert_eq!(send(&mut dialog, vec![SetBaseUnitEvent::Confirm]), []);
        assert_eq!(
            dialog.view_model().refusal.as_deref(),
            Some("A deactivated unit can't be the base unit.")
        );

        ports.borrow_mut().units.units.remove(1);
        send(&mut dialog, vec![SetBaseUnitEvent::Confirm]);
        assert_eq!(
            dialog.view_model().refusal.as_deref(),
            Some("This unit was removed while the dialog was open.")
        );
        assert_eq!(ports.borrow().base_unit.0, UnitId::SEED_AUD);
    }

    #[test]
    fn its_one_select_takes_choose_and_cancel_and_no_next_field() {
        let (_, shared) = sample();
        let view = SetBaseUnitPresenter::new(shared).view_model();

        assert_eq!(
            hints(&view.frame),
            pairs(&[
                ("select_previous/select_next", "choose"),
                ("back", "cancel")
            ])
        );
    }

    #[test]
    fn dialog_input_opens_steps_and_commits_the_list_then_select_confirms() {
        let (_, shared) = sample();
        let mut dialog = SetBaseUnitPresenter::new(shared);
        let calls = send(
            &mut dialog,
            vec![
                SetBaseUnitEvent::Input(DialogInput::Char(' ')),
                SetBaseUnitEvent::Input(DialogInput::SelectNext),
                SetBaseUnitEvent::Input(DialogInput::Select),
            ],
        );
        assert_eq!(calls, [], "Enter on the open list commits it");
        let view = dialog.view_model();
        assert!(!view.list.is_open());
        assert_eq!(view.base.chosen, Some(UnitId::SEED_BTC));

        let calls = send(
            &mut dialog,
            vec![SetBaseUnitEvent::Input(DialogInput::Select)],
        );
        assert!(matches!(
            calls.as_slice(),
            [RouterCall::CloseDialog(Some(UnitsOutcome::Unit { .. }))]
        ));
    }

    #[test]
    fn a_click_toggles_the_list_and_back_closes_it_without_picking() {
        let (_, shared) = sample();
        let mut dialog = SetBaseUnitPresenter::new(shared);
        send(&mut dialog, vec![SetBaseUnitEvent::ToggleList]);
        assert!(dialog.view_model().list.is_open());

        send(
            &mut dialog,
            vec![
                SetBaseUnitEvent::Input(DialogInput::SelectNext),
                SetBaseUnitEvent::CloseList,
            ],
        );
        let view = dialog.view_model();
        assert!(!view.list.is_open());
        assert_eq!(view.base.chosen, Some(UnitId::SEED_AUD));
    }
}
