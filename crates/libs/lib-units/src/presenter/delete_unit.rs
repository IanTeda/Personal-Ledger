//! The Delete unit Dialog, and the Deactivate unit Dialog it turns into.
//!
//! It reads the Unit's usage when it opens: a Unit in use opens as Deactivate with the counts up
//! front, and the base Unit's Deactivate is disabled with its reason. The Interactors keep their
//! refusals for a Unit that came into use, or became the base, under the open Dialog. Deleting the
//! base Unit also asks for the Unit that takes over as base, since the Ledger always has one.

use lib_core::UnitId;
use lib_viper::view::{Choice, DialogFrame, SelectView, TextFieldView};

use super::{UnitChange, UnitsOutcome, count, uses_text};
use crate::interactor::{DeactivateUnit, DeleteUnit};
use crate::protocols::{SharedPorts, UnitsRouter};
use crate::{Error, UnitRow, UnitUse, msg, units_by_code};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeleteUnitEvent {
    /// The confirmation field's whole new text.
    ConfirmText(String),
    NewBase(UnitId),
    Confirm,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteUnitViewModel {
    pub frame: DialogFrame,
    /// `Type btc to confirm`; `None` once it deactivates, which loses nothing to retype for.
    pub confirm: Option<TextFieldView>,
    /// The New base unit select, for the base Unit while it can still be deleted.
    pub new_base: Option<NewBaseView>,
    /// Why it can't be deleted, for a Unit in use.
    pub in_use: Option<InUseView>,
    /// Why the last confirm was refused, for any other refusal.
    pub refusal: Option<String>,
}

impl DeleteUnitViewModel {
    /// The Dialog while Unit `code` can still be deleted: the destructive frame, which confirms once
    /// `confirm_text` [`confirms_delete`], the confirmation field, and the New base unit select
    /// offering `candidates` on `new_base` when there are any, which is only for the base Unit.
    /// `lib-unit-deletion`'s Dialog draws its Unit through this too.
    pub fn deleting(
        code: &str,
        confirm_text: &str,
        candidates: &[Choice<UnitId>],
        new_base: Option<UnitId>,
        refusal: Option<String>,
    ) -> Self {
        // Typing the code takes no field key; only the New base unit select adds one.
        let mut hints = Vec::new();
        if !candidates.is_empty() {
            hints.push(super::choose_hint());
        }
        hints.push(super::cancel_hint());
        Self {
            frame: DialogFrame {
                title: msg::units_delete_title(code),
                submit: msg::units_delete_submit(),
                can_confirm: confirms_delete(confirm_text, code),
                destructive: true,
                hints,
            },
            confirm: Some(TextFieldView {
                label: msg::units_delete_confirm_label(code),
                text: confirm_text.to_owned(),
                placeholder: code.to_owned(),
                focused: true,
            }),
            new_base: (!candidates.is_empty()).then(|| NewBaseView {
                select: SelectView {
                    label: msg::units_delete_new_base_label(),
                    options: candidates.to_vec(),
                    chosen: new_base,
                    focused: false,
                },
                help: msg::units_delete_new_base_help(code),
            }),
            in_use: None,
            refusal,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewBaseView {
    pub select: SelectView<UnitId>,
    /// `aud is the base unit. Choose the unit that takes over before it is deleted.`
    pub help: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InUseView {
    /// `btc is used by 2 accounts and 1 budget, so it can't be deleted.`
    pub text: String,
    pub help: String,
    /// Why Deactivate is disabled: `It is also the base unit, so it stays active until ...`.
    pub base: Option<String>,
}

pub struct DeleteUnitPresenter {
    ports: SharedPorts,
    id: UnitId,
    code: String,
    confirm_text: String,
    /// The active Units that could take over as base, labelled; empty unless the Unit was the base
    /// when the Dialog opened.
    candidates: Vec<Choice<UnitId>>,
    new_base: Option<UnitId>,
    /// Read when the Dialog opens, or set by the Interactor's unit-in-use refusal if the Unit came
    /// into use under it: confirming then deactivates.
    in_use: Option<UnitUse>,
    /// Whether the Unit was the base when the Dialog opened, which Deactivate can't touch.
    is_base: bool,
    refusal: Option<String>,
}

impl DeleteUnitPresenter {
    /// The Dialog for Unit `id`, or `None` when it is gone.
    pub fn new(ports: SharedPorts, id: UnitId) -> Option<Self> {
        let (code, candidates, in_use, is_base) = {
            let ports = ports.borrow();
            let units = ports.units().units();
            let unit = units.iter().find(|unit| unit.id == id)?;
            // The pickers offer only active Units, as deactivating one hides it from them.
            let is_base = ports.base_unit().base_unit() == id;
            let candidates = if is_base {
                active_unit_choices(&units, id)
            } else {
                Vec::new()
            };
            let uses = ports.usage().unit_usage(id);
            (
                unit.code.clone(),
                candidates,
                uses.is_used().then_some(uses),
                is_base,
            )
        };
        let new_base = candidates
            .first()
            .map(|choice: &Choice<UnitId>| choice.value);
        Some(Self {
            ports,
            id,
            code,
            confirm_text: String::new(),
            candidates,
            new_base,
            in_use,
            is_base,
            refusal: None,
        })
    }

    pub fn handle(&mut self, event: DeleteUnitEvent, router: &mut dyn UnitsRouter) {
        match event {
            DeleteUnitEvent::ConfirmText(text) => self.confirm_text = text,
            DeleteUnitEvent::NewBase(id) => {
                if self.candidates.iter().any(|choice| choice.value == id) {
                    self.new_base = Some(id);
                }
            }
            DeleteUnitEvent::Confirm if self.in_use.is_some() => {
                if !self.is_base {
                    self.deactivate(router);
                }
            }
            DeleteUnitEvent::Confirm => self.delete(router),
            DeleteUnitEvent::Cancel => router.close_dialog(None),
        }
    }

    pub fn view_model(&self) -> DeleteUnitViewModel {
        let code = self.code.as_str();
        match self.in_use {
            Some(uses) => DeleteUnitViewModel {
                frame: DialogFrame {
                    title: msg::units_deactivate_title(code),
                    submit: msg::units_deactivate_submit(),
                    can_confirm: !self.is_base,
                    destructive: false,
                    hints: vec![super::cancel_hint()],
                },
                confirm: None,
                new_base: None,
                in_use: Some(InUseView {
                    text: msg::units_in_use(code, &uses_text(uses)),
                    help: msg::units_in_use_help(),
                    base: self.is_base.then(msg::units_in_use_base),
                }),
                refusal: self.refusal.clone(),
            },
            None => DeleteUnitViewModel::deleting(
                code,
                &self.confirm_text,
                &self.candidates,
                self.new_base,
                self.refusal.clone(),
            ),
        }
    }

    fn delete(&mut self, router: &mut dyn UnitsRouter) {
        if !confirms_delete(&self.confirm_text, &self.code) {
            return;
        }
        let deleted = {
            let mut ports = self.ports.borrow_mut();
            let ports = ports.split_mut();
            DeleteUnit {
                id: self.id,
                new_base: self.new_base,
            }
            .run(ports.units, ports.base_unit, ports.usage)
        };
        let code = self.code.as_str();
        self.refusal = match deleted {
            Ok(unit) => {
                router.close_dialog(Some(UnitsOutcome::Unit {
                    change: UnitChange::Deleted,
                    code: unit.code,
                }));
                None
            }
            Err(Error::UnitInUse(uses)) => {
                self.in_use = Some(uses);
                None
            }
            Err(refusal) => Some(delete_refusal(refusal, code)),
        };
    }

    fn deactivate(&mut self, router: &mut dyn UnitsRouter) {
        let deactivated = {
            let mut ports = self.ports.borrow_mut();
            let ports = ports.split_mut();
            DeactivateUnit(self.id).run(ports.units, ports.base_unit)
        };
        self.refusal = match deactivated {
            Ok(unit) => {
                router.close_dialog(Some(UnitsOutcome::Unit {
                    change: UnitChange::Deactivated,
                    code: unit.code,
                }));
                None
            }
            Err(Error::DeactivateBase) => Some(msg::units_in_use_base()),
            Err(Error::UnitGone) => Some(msg::units_refusal_unit_gone()),
            Err(refusal) => {
                debug_assert!(false, "DeactivateUnit doesn't return {refusal:?}");
                Some(msg::units_refusal_unit_gone())
            }
        };
    }
}

/// Every active Unit but `except`, by code, labelled `aud — Australian Dollar`: what a select
/// offers in place of a Unit being deleted.
pub fn active_unit_choices(units: &[UnitRow], except: UnitId) -> Vec<Choice<UnitId>> {
    units_by_code(units)
        .into_iter()
        .filter(|other| other.id != except && other.is_active)
        .map(|other| Choice {
            value: other.id,
            label: msg::units_base_unit_option(&other.code, &other.name),
        })
        .collect()
}

/// Whether `typed` confirms deleting Unit `code`: it must match exactly, case and all.
pub fn confirms_delete(typed: &str, code: &str) -> bool {
    typed == code
}

/// Why deleting Unit `code` was refused, for every refusal but a Unit in use, which turns the
/// Dialog into Deactivate rather than saying so.
pub fn delete_refusal(refusal: Error, code: &str) -> String {
    match refusal {
        Error::OnlyUnit => msg::units_refusal_only_unit(code),
        Error::BaseUnitNeeded => msg::units_refusal_base_unit_needed(code),
        Error::NewBaseGone => msg::units_refusal_new_base_gone(),
        Error::InactiveBase => msg::units_refusal_inactive_base(),
        Error::QuotedBy(quoting) => msg::units_refusal_quoted(count(quoting), code),
        Error::UnitGone => msg::units_refusal_unit_gone(),
        refusal => {
            debug_assert!(false, "DeleteUnit doesn't return {refusal:?}");
            msg::units_refusal_unit_gone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{hints, pairs, stub, unquote_aud};
    use super::*;
    use crate::UnitRepository as _;
    use crate::fake::{RecordingRouter, RouterCall};

    fn send(dialog: &mut DeleteUnitPresenter, events: Vec<DeleteUnitEvent>) -> Vec<RouterCall> {
        let mut router = RecordingRouter::default();
        for event in events {
            dialog.handle(event, &mut router);
        }
        router.calls
    }

    #[test]
    fn deleting_needs_the_exact_code_typed_and_closes_with_it() {
        let (ports, shared) = stub();
        let mut dialog = DeleteUnitPresenter::new(shared, UnitId::SEED_BTC).unwrap();
        let view = dialog.view_model();
        assert_eq!(view.frame.title, "Delete unit — btc");
        assert_eq!(view.confirm.unwrap().label, "Type btc to confirm");
        assert!(
            view.new_base.is_none(),
            "only the base Unit asks for a new base"
        );

        assert_eq!(
            send(
                &mut dialog,
                vec![
                    DeleteUnitEvent::ConfirmText("BTC".into()),
                    DeleteUnitEvent::Confirm
                ]
            ),
            []
        );
        assert!(
            !dialog.view_model().frame.can_confirm,
            "the match is case-sensitive"
        );

        let calls = send(
            &mut dialog,
            vec![
                DeleteUnitEvent::ConfirmText("btc".into()),
                DeleteUnitEvent::Confirm,
            ],
        );
        assert_eq!(
            calls,
            [RouterCall::CloseDialog(Some(UnitsOutcome::Unit {
                change: UnitChange::Deleted,
                code: "btc".into()
            }))]
        );
        assert!(ports.borrow().units.unit(UnitId::SEED_BTC).is_none());
    }

    #[test]
    fn deleting_the_base_unit_moves_the_base_to_the_chosen_active_unit() {
        let (ports, shared) = stub();
        unquote_aud(&mut ports.borrow_mut());
        ports.borrow_mut().units.units[1].is_active = false;
        let mut dialog = DeleteUnitPresenter::new(shared, UnitId::SEED_AUD).unwrap();
        let new_base = dialog.view_model().new_base.expect("aud is the base");
        let labels: Vec<_> = new_base
            .select
            .options
            .iter()
            .map(|o| o.label.as_str())
            .collect();
        assert_eq!(
            labels,
            ["vas — Vanguard Aus Shares"],
            "deactivated btc isn't offered"
        );
        assert_eq!(
            new_base.help,
            "aud is the base unit. Choose the unit that takes over before it is deleted."
        );

        send(
            &mut dialog,
            vec![
                DeleteUnitEvent::NewBase(UnitId::SEED_VAS),
                DeleteUnitEvent::ConfirmText("aud".into()),
                DeleteUnitEvent::Confirm,
            ],
        );

        assert_eq!(ports.borrow().base_unit.0, UnitId::SEED_VAS);
    }

    #[test]
    fn deleting_a_unit_other_units_quote_is_refused_as_text_with_the_count() {
        let (ports, shared) = stub();
        let before = ports.borrow().clone();
        let mut dialog = DeleteUnitPresenter::new(shared.clone(), UnitId::SEED_AUD).unwrap();
        send(
            &mut dialog,
            vec![
                DeleteUnitEvent::ConfirmText("aud".into()),
                DeleteUnitEvent::Confirm,
            ],
        );
        assert_eq!(
            dialog.view_model().refusal.as_deref(),
            Some(
                "aud is the quote unit of 2 other units, so it can't be deleted. Change their \
                 quotes first."
            )
        );
        assert_eq!(*ports.borrow(), before, "a refused delete writes nothing");

        ports.borrow_mut().units.units[2].link.quote = UnitId::SEED_AUD;
        ports.borrow_mut().units.units[1].link.quote = UnitId::SEED_VAS;
        let mut dialog = DeleteUnitPresenter::new(shared, UnitId::SEED_AUD).unwrap();
        send(
            &mut dialog,
            vec![
                DeleteUnitEvent::ConfirmText("aud".into()),
                DeleteUnitEvent::Confirm,
            ],
        );
        assert_eq!(
            dialog.view_model().refusal.as_deref(),
            Some(
                "aud is the quote unit of 1 other unit, so it can't be deleted. Change that \
                 unit's quote first."
            )
        );
    }

    #[test]
    fn a_unit_in_use_opens_offering_deactivate_with_its_counts() {
        let (ports, shared) = stub();
        ports.borrow_mut().usage.uses.insert(
            UnitId::SEED_BTC,
            UnitUse {
                accounts: 2,
                budgets: 1,
                ..UnitUse::default()
            },
        );
        let mut dialog = DeleteUnitPresenter::new(shared, UnitId::SEED_BTC).unwrap();

        let view = dialog.view_model();
        assert_eq!(view.frame.title, "Deactivate unit — btc");
        assert_eq!(view.frame.submit, "Deactivate unit");
        assert!(view.frame.can_confirm && !view.frame.destructive);
        assert_eq!(view.confirm, None);
        let in_use = view.in_use.unwrap();
        assert_eq!(
            in_use.text,
            "btc is used by 2 accounts, 1 budget, so it can't be deleted."
        );
        assert_eq!(in_use.base, None);

        let calls = send(&mut dialog, vec![DeleteUnitEvent::Confirm]);
        assert_eq!(
            calls,
            [RouterCall::CloseDialog(Some(UnitsOutcome::Unit {
                change: UnitChange::Deactivated,
                code: "btc".into()
            }))]
        );
        let ports = ports.borrow();
        assert!(!ports.units.unit(UnitId::SEED_BTC).unwrap().is_active);
    }

    #[test]
    fn a_unit_unused_when_opened_but_used_by_confirm_still_turns_into_deactivate() {
        let (ports, shared) = stub();
        let mut dialog = DeleteUnitPresenter::new(shared, UnitId::SEED_BTC).unwrap();
        assert!(dialog.view_model().in_use.is_none());
        ports.borrow_mut().usage.uses.insert(
            UnitId::SEED_BTC,
            UnitUse {
                accounts: 1,
                ..UnitUse::default()
            },
        );

        let calls = send(
            &mut dialog,
            vec![
                DeleteUnitEvent::ConfirmText("btc".into()),
                DeleteUnitEvent::Confirm,
            ],
        );

        assert_eq!(calls, [], "the refused delete keeps the Dialog open");
        assert_eq!(dialog.view_model().frame.title, "Deactivate unit — btc");
    }

    #[test]
    fn the_base_unit_in_use_opens_with_deactivate_disabled_and_its_reason() {
        let (ports, shared) = stub();
        ports.borrow_mut().usage.uses.insert(
            UnitId::SEED_AUD,
            UnitUse {
                accounts: 3,
                ..UnitUse::default()
            },
        );
        let mut dialog = DeleteUnitPresenter::new(shared, UnitId::SEED_AUD).unwrap();

        let view = dialog.view_model();
        assert_eq!(view.frame.submit, "Deactivate unit");
        assert!(
            !view.frame.can_confirm,
            "the base Unit can't be deactivated"
        );
        assert_eq!(
            view.new_base, None,
            "a Unit in use isn't deleted, so needs no new base"
        );
        assert_eq!(
            view.in_use.unwrap().base.as_deref(),
            Some(
                "It is also the base unit, so it stays active until another unit takes over as \
                 base."
            )
        );

        assert_eq!(send(&mut dialog, vec![DeleteUnitEvent::Confirm]), []);
        assert!(
            ports
                .borrow()
                .units
                .unit(UnitId::SEED_AUD)
                .unwrap()
                .is_active
        );
    }

    #[test]
    fn deleting_the_only_unit_or_a_base_with_no_active_successor_is_refused_as_text() {
        let (ports, shared) = stub();
        ports.borrow_mut().units.units.truncate(1);
        let mut only = DeleteUnitPresenter::new(shared.clone(), UnitId::SEED_AUD).unwrap();
        send(
            &mut only,
            vec![
                DeleteUnitEvent::ConfirmText("aud".into()),
                DeleteUnitEvent::Confirm,
            ],
        );
        assert_eq!(
            only.view_model().refusal.as_deref(),
            Some("aud is the only unit, so it can't be deleted.")
        );

        let (ports, shared) = stub();
        for unit in &mut ports.borrow_mut().units.units[1..] {
            unit.is_active = false;
        }
        let mut base = DeleteUnitPresenter::new(shared, UnitId::SEED_AUD).unwrap();
        assert_eq!(base.view_model().new_base, None, "no active Unit to offer");
        send(
            &mut base,
            vec![
                DeleteUnitEvent::ConfirmText("aud".into()),
                DeleteUnitEvent::Confirm,
            ],
        );
        assert_eq!(
            base.view_model().refusal.as_deref(),
            Some("aud is the base unit, and no active unit can take over as base.")
        );
    }

    #[test]
    fn a_unit_or_new_base_removed_under_the_dialog_is_refused_as_text() {
        let (ports, shared) = stub();
        let mut dialog = DeleteUnitPresenter::new(shared.clone(), UnitId::SEED_VAS).unwrap();
        ports.borrow_mut().units.units.remove(2);
        send(
            &mut dialog,
            vec![
                DeleteUnitEvent::ConfirmText("vas".into()),
                DeleteUnitEvent::Confirm,
            ],
        );
        assert_eq!(
            dialog.view_model().refusal.as_deref(),
            Some("This unit was removed while the dialog was open.")
        );

        let (ports, shared) = stub();
        let mut base = DeleteUnitPresenter::new(shared, UnitId::SEED_AUD).unwrap();
        send(&mut base, vec![DeleteUnitEvent::NewBase(UnitId::SEED_VAS)]);
        ports.borrow_mut().units.units.remove(2);
        send(
            &mut base,
            vec![
                DeleteUnitEvent::ConfirmText("aud".into()),
                DeleteUnitEvent::Confirm,
            ],
        );
        assert_eq!(
            base.view_model().refusal.as_deref(),
            Some("The unit chosen as the new base unit is gone.")
        );

        let (ports, shared) = stub();
        let mut inactive = DeleteUnitPresenter::new(shared, UnitId::SEED_AUD).unwrap();
        ports.borrow_mut().units.units[1].is_active = false;
        send(
            &mut inactive,
            vec![
                DeleteUnitEvent::ConfirmText("aud".into()),
                DeleteUnitEvent::Confirm,
            ],
        );
        assert_eq!(
            inactive.view_model().refusal.as_deref(),
            Some("A deactivated unit can't be the base unit.")
        );
    }

    #[test]
    fn deleting_or_deactivating_takes_no_field_keys_but_the_new_base_select_takes_choose() {
        let (ports, shared) = stub();
        let view = DeleteUnitPresenter::new(shared.clone(), UnitId::SEED_BTC)
            .unwrap()
            .view_model();
        assert_eq!(hints(&view.frame), pairs(&[("back", "cancel")]));

        let view = DeleteUnitPresenter::new(shared.clone(), UnitId::SEED_AUD)
            .unwrap()
            .view_model();
        assert!(view.new_base.is_some(), "aud is the base");
        assert_eq!(
            hints(&view.frame),
            pairs(&[
                ("select_previous/select_next", "choose"),
                ("back", "cancel")
            ])
        );

        ports.borrow_mut().usage.uses.insert(
            UnitId::SEED_BTC,
            UnitUse {
                accounts: 1,
                ..UnitUse::default()
            },
        );
        let view = DeleteUnitPresenter::new(shared, UnitId::SEED_BTC)
            .unwrap()
            .view_model();
        assert!(view.in_use.is_some(), "btc deactivates");
        assert_eq!(hints(&view.frame), pairs(&[("back", "cancel")]));
    }
}
