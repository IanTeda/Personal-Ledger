//! The Set base unit Dialog: a select over the active Units, by code, on the base Unit to start.

use lib_core::UnitId;

use super::{Choice, DialogFrame, SelectView, UnitsOutcome};
use crate::interactor::{SetBaseUnit, SetBaseUnitRefusal};
use crate::protocols::{SharedPorts, UnitsRouter};
use crate::{msg, units_by_code};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetBaseUnitEvent {
    Choose(UnitId),
    Confirm,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetBaseUnitViewModel {
    pub frame: DialogFrame,
    pub base: SelectView<UnitId>,
    pub refusal: Option<String>,
}

pub struct SetBaseUnitPresenter {
    ports: SharedPorts,
    options: Vec<Choice<UnitId>>,
    chosen: Option<UnitId>,
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
            refusal: None,
        }
    }

    pub fn handle(&mut self, event: SetBaseUnitEvent, router: &mut dyn UnitsRouter) {
        match event {
            SetBaseUnitEvent::Choose(id) => {
                if self.options.iter().any(|choice| choice.value == id) {
                    self.chosen = Some(id);
                }
            }
            SetBaseUnitEvent::Confirm => self.confirm(router),
            SetBaseUnitEvent::Cancel => router.close_dialog(None),
        }
    }

    pub fn view_model(&self) -> SetBaseUnitViewModel {
        SetBaseUnitViewModel {
            frame: DialogFrame {
                title: msg::units_set_base_title(),
                submit: msg::units_set_base_submit(),
                can_confirm: self.chosen.is_some(),
                destructive: false,
            },
            base: SelectView {
                label: msg::units_set_base_label(),
                options: self.options.clone(),
                chosen: self.chosen,
                focused: true,
            },
            refusal: self.refusal.clone(),
        }
    }

    fn confirm(&mut self, router: &mut dyn UnitsRouter) {
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
                router.close_dialog(Some(UnitsOutcome::BaseSet { code }));
                None
            }
            (Err(SetBaseUnitRefusal::UnitGone), _) | (Ok(()), None) => {
                Some(msg::units_refusal_unit_gone())
            }
            (Err(SetBaseUnitRefusal::Inactive), _) => Some(msg::units_refusal_inactive_base()),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::stub;
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
        let (ports, shared) = stub();
        ports.borrow_mut().units.units[2].is_active = false;
        let view = SetBaseUnitPresenter::new(shared).view_model();

        let labels: Vec<_> = view.base.options.iter().map(|o| o.label.as_str()).collect();
        assert_eq!(labels, ["aud — Australian Dollar", "btc — Bitcoin"]);
        assert_eq!(view.base.chosen, Some(UnitId::SEED_AUD));
    }

    #[test]
    fn confirming_moves_the_base_and_closes_with_its_code() {
        let (ports, shared) = stub();
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
            [RouterCall::CloseDialog(Some(UnitsOutcome::BaseSet {
                code: "btc".into()
            }))]
        );
        assert_eq!(ports.borrow().base_unit.0, UnitId::SEED_BTC);
    }

    #[test]
    fn a_unit_deactivated_or_removed_under_the_dialog_is_refused_as_text() {
        let (ports, shared) = stub();
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
}
