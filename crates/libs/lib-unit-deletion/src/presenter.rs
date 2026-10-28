//! The Delete unit Dialog (16-settings' Units), as `lib-units` draws it, with a Usual unit section
//! when Institutions name the Unit:
//!
//! - One Usual unit select per Institution naming the Unit, labelled with its name, each starting
//!   on None, never silently the base Unit, and offering every other active Unit. The person may
//!   confirm as-is.
//! - When the Unit is also the base, the same Dialog asks for the Unit that takes over as base,
//!   rather than chaining a second Dialog.
//! - A Unit something other than an Institution names (an Account, a Bill Plan, a Budget, a
//!   Location) can't be deleted: the Dialog is `lib-units`' own, which turns into Deactivate. So is
//!   one that came into use under the open Dialog, as the Interactor's refusal says.
//!
//! Every figure is read when it opens: it is modal, so nothing changes them under it, and the
//! Interactor reads the Institutions again on confirm. The Dialog closes through `lib-units`'
//! [`UnitsRouter`] with the outcome every Units Dialog closes with.

use lib_core::{InstitutionId, UnitId};
use lib_units::presenter::{
    DeleteUnitEvent as UnitsDeleteEvent, DeleteUnitPresenter as UnitsDeletePresenter,
    DeleteUnitViewModel as UnitsDeleteViewModel, NewBaseView, UnitChange, UnitsOutcome,
    active_unit_choices, delete_refusal,
};
use lib_units::{SharedPorts, UnitUsage, UnitUse, UnitsRouter};
use lib_viper::view::{Choice, DialogFrame, SelectView, TextFieldView};

use crate::error::UnitDeletionError;
use crate::interactor::DeleteUnit;
use crate::msg;
use crate::protocols::SharedUnitDeletion;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeleteUnitEvent {
    /// The confirmation field's whole new text.
    ConfirmText(String),
    NewBase(UnitId),
    /// The Usual unit `institution` takes instead, `None` to leave it with none.
    UsualUnit {
        institution: InstitutionId,
        unit: Option<UnitId>,
    },
    Confirm,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteUnitViewModel {
    /// The Dialog as `lib-units` draws it: the frame, the confirmation field, the New base unit
    /// select, or the in-use counts once it deactivates.
    pub unit: UnitsDeleteViewModel,
    /// The Institutions naming the Unit; `None` when none does, or once it deactivates.
    pub usual_units: Option<UsualUnitsView>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsualUnitsView {
    /// `2 institutions use btc as their usual unit. Choose the unit each uses instead, ...`
    pub help: String,
    /// One per Institution, in the Ledger's order.
    pub selects: Vec<UsualUnitSelect>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsualUnitSelect {
    pub institution: InstitutionId,
    /// Labelled with the Institution's name; the `None` choice leaves it with no Usual unit.
    pub select: SelectView<Option<UnitId>>,
}

pub struct DeleteUnitPresenter {
    mode: Mode,
}

enum Mode {
    /// Something other than an Institution names the Unit: `lib-units`' Dialog, which deactivates.
    Deactivate(UnitsDeletePresenter),
    Delete(Deleting),
}

/// The Dialog while the Unit can still be deleted.
struct Deleting {
    ports: SharedPorts,
    deletion: SharedUnitDeletion,
    id: UnitId,
    code: String,
    confirm_text: String,
    /// The active Units that could take over as base, labelled; empty unless the Unit was the base
    /// when the Dialog opened.
    candidates: Vec<Choice<UnitId>>,
    new_base: Option<UnitId>,
    named: Vec<Named>,
    /// What each Institution's select offers: None, then every other active Unit.
    options: Vec<Choice<Option<UnitId>>>,
    refusal: Option<String>,
}

/// An Institution naming the Unit, with the Usual unit chosen for it.
struct Named {
    id: InstitutionId,
    name: String,
    usual_unit: Option<UnitId>,
}

impl DeleteUnitPresenter {
    /// The Dialog for Unit `id`, reading what names it through `ports`' usage and the rest through
    /// `deletion`, or `None` when it is gone. `ports` are `lib-units`' own, which Deactivate writes
    /// through.
    pub fn new(ports: SharedPorts, deletion: SharedUnitDeletion, id: UnitId) -> Option<Self> {
        let uses = ports.borrow().usage().unit_usage(id);
        let others = UnitUse {
            institutions: 0,
            ..uses
        };
        if others.is_used() {
            return Some(Self {
                mode: Mode::Deactivate(UnitsDeletePresenter::new(ports, id)?),
            });
        }
        let (units, base_unit, naming) = {
            let deletion = deletion.borrow();
            (
                deletion.units(),
                deletion.base_unit(),
                deletion.institutions_naming(id),
            )
        };
        let code = units.iter().find(|unit| unit.id == id)?.code.clone();
        // The selects offer only active Units, as deactivating one hides it from them.
        let others = active_unit_choices(&units, id);
        let candidates = if base_unit == id {
            others.clone()
        } else {
            Vec::new()
        };
        let options = std::iter::once(Choice {
            value: None,
            label: msg::unit_deletion_usual_unit_none(),
        })
        .chain(others.into_iter().map(|choice| Choice {
            value: Some(choice.value),
            label: choice.label,
        }))
        .collect();
        let named = naming
            .into_iter()
            .map(|institution| Named {
                id: institution.id,
                name: institution.name,
                usual_unit: None,
            })
            .collect();
        Some(Self {
            mode: Mode::Delete(Deleting {
                ports,
                deletion,
                id,
                code,
                confirm_text: String::new(),
                new_base: candidates.first().map(|choice| choice.value),
                candidates,
                named,
                options,
                refusal: None,
            }),
        })
    }

    pub fn handle(&mut self, event: DeleteUnitEvent, router: &mut dyn UnitsRouter) {
        match &mut self.mode {
            Mode::Deactivate(dialog) => {
                let event = match event {
                    DeleteUnitEvent::ConfirmText(text) => UnitsDeleteEvent::ConfirmText(text),
                    DeleteUnitEvent::NewBase(id) => UnitsDeleteEvent::NewBase(id),
                    DeleteUnitEvent::UsualUnit { .. } => return,
                    DeleteUnitEvent::Confirm => UnitsDeleteEvent::Confirm,
                    DeleteUnitEvent::Cancel => UnitsDeleteEvent::Cancel,
                };
                dialog.handle(event, router);
            }
            Mode::Delete(dialog) => {
                if let Some(deactivate) = dialog.handle(event, router) {
                    self.mode = Mode::Deactivate(deactivate);
                }
            }
        }
    }

    pub fn view_model(&self) -> DeleteUnitViewModel {
        match &self.mode {
            Mode::Deactivate(dialog) => DeleteUnitViewModel {
                unit: dialog.view_model(),
                usual_units: None,
            },
            Mode::Delete(dialog) => dialog.view_model(),
        }
    }
}

impl Deleting {
    /// Returns `lib-units`' Dialog to carry on in when the Unit came into use under this one.
    fn handle(
        &mut self,
        event: DeleteUnitEvent,
        router: &mut dyn UnitsRouter,
    ) -> Option<UnitsDeletePresenter> {
        match event {
            DeleteUnitEvent::ConfirmText(text) => self.confirm_text = text,
            DeleteUnitEvent::NewBase(id) => {
                if self.candidates.iter().any(|choice| choice.value == id) {
                    self.new_base = Some(id);
                }
            }
            DeleteUnitEvent::UsualUnit { institution, unit } => {
                let offered = self.options.iter().any(|choice| choice.value == unit);
                let named = self.named.iter_mut().find(|named| named.id == institution);
                if let (true, Some(named)) = (offered, named) {
                    named.usual_unit = unit;
                }
            }
            DeleteUnitEvent::Confirm => return self.delete(router),
            DeleteUnitEvent::Cancel => router.close_dialog(None),
        }
        None
    }

    fn view_model(&self) -> DeleteUnitViewModel {
        let code = self.code.as_str();
        let unit = UnitsDeleteViewModel {
            frame: DialogFrame {
                title: lib_units::msg::units_delete_title(code),
                submit: lib_units::msg::units_delete_submit(),
                can_confirm: self.confirm_text == self.code,
                destructive: true,
            },
            confirm: Some(TextFieldView {
                label: lib_units::msg::units_delete_confirm_label(code),
                text: self.confirm_text.clone(),
                placeholder: self.code.clone(),
                focused: true,
            }),
            new_base: (!self.candidates.is_empty()).then(|| NewBaseView {
                select: SelectView {
                    label: lib_units::msg::units_delete_new_base_label(),
                    options: self.candidates.clone(),
                    chosen: self.new_base,
                    focused: false,
                },
                help: lib_units::msg::units_delete_new_base_help(code),
            }),
            in_use: None,
            refusal: self.refusal.clone(),
        };
        let usual_units = (!self.named.is_empty()).then(|| UsualUnitsView {
            help: msg::unit_deletion_institutions_help(count(self.named.len()), code),
            selects: self
                .named
                .iter()
                .map(|named| UsualUnitSelect {
                    institution: named.id,
                    select: SelectView {
                        label: named.name.clone(),
                        options: self.options.clone(),
                        chosen: Some(named.usual_unit),
                        focused: false,
                    },
                })
                .collect(),
        });
        DeleteUnitViewModel { unit, usual_units }
    }

    /// The typed code must match exactly, case and all.
    fn delete(&mut self, router: &mut dyn UnitsRouter) -> Option<UnitsDeletePresenter> {
        if self.confirm_text != self.code {
            return None;
        }
        // The usage is copied out before the write: a Client may share one store between `ports`
        // and `deletion`, which a borrow of `ports` held across the write would borrow twice.
        let usage = Usage(self.ports.borrow().usage().unit_usage(self.id));
        let deleted = DeleteUnit {
            id: self.id,
            new_base: self.new_base,
            replacements: self
                .named
                .iter()
                .filter_map(|named| Some((named.id, named.usual_unit?)))
                .collect(),
        }
        .run(&mut *self.deletion.borrow_mut(), &usage);
        self.refusal = match deleted {
            Ok(deleted) => {
                router.close_dialog(Some(UnitsOutcome::Unit {
                    change: UnitChange::Deleted,
                    code: deleted.unit.code,
                }));
                None
            }
            Err(UnitDeletionError::Unit(lib_units::Error::UnitInUse(_))) => {
                match UnitsDeletePresenter::new(self.ports.clone(), self.id) {
                    Some(deactivate) => return Some(deactivate),
                    None => Some(lib_units::msg::units_refusal_unit_gone()),
                }
            }
            Err(UnitDeletionError::Unit(refusal)) => Some(delete_refusal(refusal, &self.code)),
            Err(UnitDeletionError::ReplacementGone) => {
                Some(msg::unit_deletion_refusal_replacement_gone())
            }
        };
        None
    }
}

/// The deleted Unit's usage as read just before the write; the Interactor asks of no other Unit.
struct Usage(UnitUse);

impl UnitUsage for Usage {
    fn unit_usage(&self, _id: UnitId) -> UnitUse {
        self.0
    }
}

/// A count as a Message's plural argument.
fn count(count: usize) -> i64 {
    i64::try_from(count).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use lib_institutions::InstitutionRepository as _;
    use lib_units::UnitRepository as _;
    use lib_units::fake::{InMemoryPorts, RecordingRouter, RouterCall};

    use super::*;
    use crate::fake::InMemoryUnitDeletion;
    use crate::fake::world::{AUD, BTC, COINSPOT, KRAKEN, VAS, WESTPAC, now, world};

    /// The Delete unit Dialog on the world, with `lib-units`' ports holding the same Units and
    /// nothing but what `uses` says naming any.
    struct Dialog {
        deletion: Rc<RefCell<InMemoryUnitDeletion>>,
        ports: Rc<RefCell<InMemoryPorts>>,
        presenter: DeleteUnitPresenter,
        router: RecordingRouter,
    }

    impl Dialog {
        fn open(id: UnitId) -> Self {
            Self::open_on(world(), id, UnitUse::default())
        }

        fn open_on(rows: InMemoryUnitDeletion, id: UnitId, uses: UnitUse) -> Self {
            lib_locale::init_with_layers("en-US", &[msg::LAYER, lib_units::msg::LAYER]);
            let mut ports = InMemoryPorts::stub(now());
            ports.units = rows.units.clone();
            ports.base_unit = rows.base_unit;
            ports.usage.uses.insert(id, uses);
            let (ports, shared) = ports.shared();
            let deletion = Rc::new(RefCell::new(rows));
            let port: SharedUnitDeletion = deletion.clone();
            let presenter = DeleteUnitPresenter::new(shared, port, id).expect("a world Unit");
            Self {
                deletion,
                ports,
                presenter,
                router: RecordingRouter::default(),
            }
        }

        fn send(&mut self, event: DeleteUnitEvent) {
            self.presenter.handle(event, &mut self.router);
        }

        fn confirm(&mut self, code: &str) {
            self.send(DeleteUnitEvent::ConfirmText(code.into()));
            self.send(DeleteUnitEvent::Confirm);
        }

        fn view(&self) -> DeleteUnitViewModel {
            self.presenter.view_model()
        }

        fn usual_unit(&self, id: InstitutionId) -> Option<UnitId> {
            self.deletion
                .borrow()
                .institutions
                .institution(id)
                .and_then(|institution| institution.usual_unit)
        }

        fn closed_deleting(&self, code: &str) {
            assert_eq!(
                self.router.calls,
                [RouterCall::CloseDialog(Some(UnitsOutcome::Unit {
                    change: UnitChange::Deleted,
                    code: code.into(),
                }))]
            );
        }
    }

    fn labels<T>(select: &SelectView<T>) -> Vec<&str> {
        select.options.iter().map(|o| o.label.as_str()).collect()
    }

    #[test]
    fn a_unit_nothing_names_asks_for_neither_section_and_deletes() {
        let mut dialog = Dialog::open(VAS);
        let view = dialog.view();

        assert_eq!(view.unit.frame.title, "Delete unit \u{2014} vas");
        assert!(view.unit.frame.destructive);
        assert!(view.unit.new_base.is_none());
        assert!(view.usual_units.is_none());

        dialog.confirm("vas");

        dialog.closed_deleting("vas");
        assert!(dialog.deletion.borrow().units.unit(VAS).is_none());
    }

    #[test]
    fn a_named_unit_lists_its_institutions_each_on_none_with_every_other_active_unit() {
        let mut rows = world();
        rows.units.units[2].is_active = false;
        let dialog = Dialog::open_on(rows, BTC, UnitUse::default());
        let usual_units = dialog.view().usual_units.expect("two exchanges name btc");

        assert_eq!(
            usual_units.help,
            "2 institutions use btc as their usual unit. Choose the unit each uses instead, or \
             leave them on None."
        );
        let names: Vec<_> = usual_units
            .selects
            .iter()
            .map(|named| named.select.label.as_str())
            .collect();
        assert_eq!(names, ["Coinspot", "Kraken"]);
        for named in &usual_units.selects {
            assert_eq!(named.select.chosen, Some(None), "never silently the base");
            assert_eq!(
                labels(&named.select),
                ["None", "aud \u{2014} Australian Dollar"],
                "deactivated vas and btc itself aren't offered"
            );
        }
        assert!(dialog.view().unit.new_base.is_none());
    }

    #[test]
    fn confirming_a_named_unit_reassigns_the_chosen_and_leaves_the_rest_on_none() {
        let mut dialog = Dialog::open(BTC);

        dialog.send(DeleteUnitEvent::UsualUnit {
            institution: COINSPOT,
            unit: Some(VAS),
        });
        dialog.confirm("btc");

        dialog.closed_deleting("btc");
        assert_eq!(dialog.usual_unit(COINSPOT), Some(VAS));
        assert_eq!(dialog.usual_unit(KRAKEN), None);
        assert!(dialog.deletion.borrow().units.unit(BTC).is_none());
    }

    #[test]
    fn a_unit_not_offered_or_an_institution_not_listed_is_ignored() {
        let mut dialog = Dialog::open(BTC);

        dialog.send(DeleteUnitEvent::UsualUnit {
            institution: COINSPOT,
            unit: Some(BTC),
        });
        dialog.send(DeleteUnitEvent::UsualUnit {
            institution: WESTPAC,
            unit: Some(VAS),
        });

        let usual_units = dialog.view().usual_units.expect("btc is named");
        assert!(
            usual_units
                .selects
                .iter()
                .all(|named| named.select.chosen == Some(None))
        );
        assert_eq!(usual_units.selects.len(), 2);
    }

    #[test]
    fn the_base_unit_alone_asks_only_for_the_new_base() {
        let mut rows = world();
        rows.institutions.institutions[1].usual_unit = None;
        let mut dialog = Dialog::open_on(rows, AUD, UnitUse::default());
        let view = dialog.view();

        let new_base = view.unit.new_base.expect("aud is the base");
        assert_eq!(
            labels(&new_base.select),
            ["btc \u{2014} Bitcoin", "vas \u{2014} Vanguard Aus Shares"]
        );
        assert!(view.usual_units.is_none());

        dialog.send(DeleteUnitEvent::NewBase(VAS));
        dialog.confirm("aud");

        dialog.closed_deleting("aud");
        assert_eq!(dialog.deletion.borrow().base_unit.0, VAS);
    }

    #[test]
    fn the_named_base_unit_asks_for_both_in_one_dialog() {
        let mut dialog = Dialog::open(AUD);
        let view = dialog.view();

        assert!(view.unit.new_base.is_some());
        let usual_units = view.usual_units.expect("Westpac names aud");
        assert_eq!(
            usual_units.help,
            "1 institution uses aud as its usual unit. Choose the unit it uses instead, or leave \
             it on None."
        );
        assert_eq!(usual_units.selects[0].institution, WESTPAC);

        dialog.send(DeleteUnitEvent::NewBase(VAS));
        dialog.send(DeleteUnitEvent::UsualUnit {
            institution: WESTPAC,
            unit: Some(BTC),
        });
        dialog.confirm("aud");

        dialog.closed_deleting("aud");
        assert_eq!(dialog.deletion.borrow().base_unit.0, VAS);
        assert_eq!(dialog.usual_unit(WESTPAC), Some(BTC));
    }

    #[test]
    fn a_unit_an_account_holds_opens_as_lib_units_deactivate() {
        let uses = UnitUse {
            accounts: 1,
            institutions: 2,
            ..UnitUse::default()
        };
        let mut dialog = Dialog::open_on(world(), BTC, uses);
        let view = dialog.view();

        assert_eq!(view.unit.frame.title, "Deactivate unit \u{2014} btc");
        assert!(view.unit.in_use.is_some());
        assert!(view.usual_units.is_none());

        dialog.send(DeleteUnitEvent::Confirm);

        assert_eq!(
            dialog.router.calls,
            [RouterCall::CloseDialog(Some(UnitsOutcome::Unit {
                change: UnitChange::Deactivated,
                code: "btc".into(),
            }))]
        );
        assert!(
            dialog
                .ports
                .borrow()
                .units
                .unit(BTC)
                .is_some_and(|unit| !unit.is_active)
        );
        assert_eq!(dialog.usual_unit(COINSPOT), Some(BTC), "nothing reassigned");
    }

    #[test]
    fn a_unit_that_came_into_use_under_the_dialog_turns_it_into_deactivate() {
        let mut dialog = Dialog::open(BTC);
        dialog.ports.borrow_mut().usage.uses.insert(
            BTC,
            UnitUse {
                accounts: 1,
                ..UnitUse::default()
            },
        );
        let before = dialog.deletion.borrow().clone();

        dialog.confirm("btc");

        assert!(dialog.router.calls.is_empty());
        let view = dialog.view();
        assert_eq!(view.unit.frame.title, "Deactivate unit \u{2014} btc");
        assert!(view.usual_units.is_none());
        assert_eq!(*dialog.deletion.borrow(), before);
    }

    #[test]
    fn a_refusal_is_worded_and_cancel_closes_having_written_nothing() {
        let mut dialog = Dialog::open(BTC);
        dialog.send(DeleteUnitEvent::UsualUnit {
            institution: KRAKEN,
            unit: Some(VAS),
        });
        dialog.deletion.borrow_mut().units.units[2].is_active = false;
        let before = dialog.deletion.borrow().clone();

        dialog.confirm("btc");
        assert_eq!(
            dialog.view().unit.refusal.as_deref(),
            Some("A usual unit you chose has since been deleted or deactivated. Choose another.")
        );

        dialog.confirm("BTC");
        dialog.send(DeleteUnitEvent::Cancel);
        assert_eq!(dialog.router.calls, [RouterCall::CloseDialog(None)]);
        assert_eq!(*dialog.deletion.borrow(), before);
    }

    #[test]
    fn a_unit_gone_opens_no_dialog() {
        lib_locale::init_with_layers("en-US", &[msg::LAYER, lib_units::msg::LAYER]);
        let (_, shared) = InMemoryPorts::stub(now()).shared();
        let port: SharedUnitDeletion = Rc::new(RefCell::new(world()));
        assert!(DeleteUnitPresenter::new(shared, port, UnitId::new()).is_none());
    }
}
