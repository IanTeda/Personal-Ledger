//! Unit delete, the module's one Interactor: it removes a Unit, moves the base when it was the
//! base, and gives every Institution that named it as its Usual unit the Unit chosen instead, or
//! none, two domains, so it lives here rather than in `lib-units`. It checks everything before
//! writing, so a refusal writes nothing.
//!
//! `lib-units`' delete rules hold unchanged ([`lib_units::interactor::DeleteUnit::check`]), but
//! for the Institutions naming the Unit: they are reassigned here, so they don't hold the delete
//! back as they do `lib-units`' own. An Institution keeps no Unit unless one is chosen for it,
//! never silently the base Unit.

use lib_core::{InstitutionId, UnitId};
use lib_units::{UnitUsage, UnitUse};

use crate::error::{Error, Result};
use crate::protocols::{UnitDeleted, UnitDeletion, UnitRemoval};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteUnit {
    pub id: UnitId,
    /// The Unit that takes over as base, needed when Unit `id` is the base and ignored otherwise.
    pub new_base: Option<UnitId>,
    /// The Usual unit each Institution naming Unit `id` takes instead. An Institution not listed
    /// is left with none; one listed that no longer names the Unit is left alone.
    pub replacements: Vec<Replacement>,
}

/// The Usual unit an Institution naming the deleted Unit takes instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Replacement {
    pub institution: InstitutionId,
    pub unit: UnitId,
}

impl DeleteUnit {
    /// Deletes the Unit, moving the base and reassigning the Institutions that named it in the same
    /// write. `usage` says what names it in the domains this doesn't write.
    ///
    /// # Errors
    /// `Unit` for any of `lib-units`' delete refusals, and `ReplacementGone` for a chosen Usual unit
    /// that is the Unit itself, gone or deactivated.
    pub fn run(
        self,
        deletion: &mut dyn UnitDeletion,
        usage: &dyn UnitUsage,
    ) -> Result<UnitDeleted> {
        let id = self.id;
        let new_base = self.new_base.map(tracing::field::display);
        let outcome = self.apply(deletion, usage);
        match &outcome {
            Ok(deleted) => tracing::info!(
                unit_id = %id,
                new_base,
                reassigned = deleted.reassigned,
                cleared = deleted.cleared,
                "Deleted a Unit"
            ),
            Err(refusal) => {
                tracing::info!(unit_id = %id, new_base, %refusal, "Refused to delete a Unit");
            }
        }
        outcome
    }

    fn apply(self, deletion: &mut dyn UnitDeletion, usage: &dyn UnitUsage) -> Result<UnitDeleted> {
        let units = deletion.units();
        let uses = UnitUse {
            institutions: 0,
            ..usage.unit_usage(self.id)
        };
        let checked = lib_units::interactor::DeleteUnit {
            id: self.id,
            new_base: self.new_base,
        }
        .check(&units, deletion.base_unit(), uses)?;
        let mut reassigned = deletion.institutions_naming(self.id);
        for institution in &mut reassigned {
            institution.usual_unit = self
                .replacements
                .iter()
                .find(|replacement| replacement.institution == institution.id)
                .map(|replacement| replacement.unit);
            if let Some(replacement) = institution.usual_unit {
                let offered = units
                    .iter()
                    .any(|unit| unit.id == replacement && unit.id != self.id && unit.is_active);
                if !offered {
                    return Err(Error::ReplacementGone);
                }
            }
        }
        let cleared = reassigned
            .iter()
            .filter(|institution| institution.usual_unit.is_none())
            .count();
        let deleted = UnitDeleted {
            unit: checked.unit,
            reassigned: reassigned.len() - cleared,
            cleared,
        };
        deletion.remove_unit(UnitRemoval {
            unit: self.id,
            new_base: checked.new_base,
            reassigned,
        });
        Ok(deleted)
    }
}

#[cfg(test)]
mod tests {
    use lib_institutions::InstitutionRepository as _;
    use lib_units::UnitRepository as _;
    use lib_units::fake::InMemoryUnitUsage;

    use super::*;
    use crate::fake::InMemoryUnitDeletion;
    use crate::fake::world::{ANZ, AUD, BTC, COINSPOT, KRAKEN, VAS, WESTPAC, world};

    fn delete(
        deletion: &mut InMemoryUnitDeletion,
        id: UnitId,
        new_base: Option<UnitId>,
        replacements: Vec<Replacement>,
    ) -> Result<UnitDeleted> {
        DeleteUnit {
            id,
            new_base,
            replacements,
        }
        .run(deletion, &InMemoryUnitUsage::default())
    }

    /// Runs a delete that should be refused and checks it wrote nothing.
    fn refused(
        deletion: &mut InMemoryUnitDeletion,
        id: UnitId,
        new_base: Option<UnitId>,
        replacements: Vec<Replacement>,
    ) -> Error {
        let before = deletion.clone();
        let refusal =
            delete(deletion, id, new_base, replacements).expect_err("the delete should be refused");
        assert_eq!(*deletion, before, "a refused delete writes nothing");
        refusal
    }

    fn usual_unit(deletion: &InMemoryUnitDeletion, id: InstitutionId) -> Option<UnitId> {
        deletion
            .institutions
            .institution(id)
            .and_then(|institution| institution.usual_unit)
    }

    #[test]
    fn a_unit_neither_the_base_nor_named_goes_alone() {
        let mut deletion = world();
        let before = deletion.clone();

        let deleted = delete(&mut deletion, VAS, None, Vec::new()).expect("nothing names vas");

        assert_eq!(deleted.unit.code, "vas");
        assert_eq!((deleted.reassigned, deleted.cleared), (0, 0));
        assert_eq!(deletion.units.unit(VAS), None);
        assert_eq!(deletion.base_unit.0, AUD);
        assert_eq!(deletion.institutions, before.institutions);
    }

    #[test]
    fn a_named_unit_goes_with_its_institutions_reassigned_or_cleared() {
        let mut deletion = world();

        let deleted = delete(
            &mut deletion,
            BTC,
            None,
            vec![Replacement {
                institution: COINSPOT,
                unit: VAS,
            }],
        )
        .expect("only Institutions name btc");

        assert_eq!((deleted.reassigned, deleted.cleared), (1, 1));
        assert_eq!(deletion.units.unit(BTC), None);
        assert_eq!(usual_unit(&deletion, COINSPOT), Some(VAS));
        assert_eq!(
            usual_unit(&deletion, KRAKEN),
            None,
            "an Institution with no Unit chosen is left with none, not the base"
        );
        assert_eq!(usual_unit(&deletion, WESTPAC), Some(AUD));
        assert_eq!(usual_unit(&deletion, ANZ), None);
        assert_eq!(deletion.base_unit.0, AUD);
    }

    #[test]
    fn with_nothing_chosen_every_institution_is_left_with_none() {
        let mut deletion = world();

        let deleted = delete(&mut deletion, BTC, None, Vec::new()).expect("confirmed as-is");

        assert_eq!((deleted.reassigned, deleted.cleared), (0, 2));
        assert_eq!(usual_unit(&deletion, COINSPOT), None);
        assert_eq!(usual_unit(&deletion, KRAKEN), None);
    }

    #[test]
    fn the_base_unit_alone_goes_with_the_base_moved() {
        let mut deletion = world();
        deletion
            .institutions
            .institutions
            .iter_mut()
            .for_each(|institution| {
                if institution.id == WESTPAC {
                    institution.usual_unit = None;
                }
            });
        let before = deletion.clone();

        let deleted = delete(&mut deletion, AUD, Some(VAS), Vec::new()).expect("nothing names aud");

        assert_eq!((deleted.reassigned, deleted.cleared), (0, 0));
        assert_eq!(deletion.units.unit(AUD), None);
        assert_eq!(deletion.base_unit.0, VAS);
        assert_eq!(deletion.institutions, before.institutions);
    }

    #[test]
    fn the_named_base_unit_goes_with_the_base_moved_and_its_institution_reassigned() {
        let mut deletion = world();

        delete(
            &mut deletion,
            AUD,
            Some(VAS),
            vec![Replacement {
                institution: WESTPAC,
                unit: BTC,
            }],
        )
        .expect("Westpac names aud");

        assert_eq!(deletion.units.unit(AUD), None);
        assert_eq!(deletion.base_unit.0, VAS);
        assert_eq!(usual_unit(&deletion, WESTPAC), Some(BTC));
        assert_eq!(usual_unit(&deletion, COINSPOT), Some(BTC));
    }

    #[test]
    fn lib_units_rules_still_refuse_and_write_nothing() {
        let mut deletion = world();
        assert_eq!(
            refused(
                &mut deletion,
                AUD,
                None,
                vec![Replacement {
                    institution: WESTPAC,
                    unit: BTC
                }]
            ),
            Error::Unit(lib_units::Error::BaseUnitNeeded)
        );
        assert_eq!(
            refused(&mut deletion, UnitId::new(), None, Vec::new()),
            Error::Unit(lib_units::Error::UnitGone)
        );

        let mut usage = InMemoryUnitUsage::default();
        let uses = UnitUse {
            accounts: 1,
            institutions: 2,
            ..UnitUse::default()
        };
        usage.uses.insert(BTC, uses);
        let before = deletion.clone();
        assert_eq!(
            DeleteUnit {
                id: BTC,
                new_base: None,
                replacements: Vec::new(),
            }
            .run(&mut deletion, &usage),
            Err(Error::Unit(lib_units::Error::UnitInUse(UnitUse {
                institutions: 0,
                ..uses
            }))),
            "an Account holding btc still refuses it, counting only what this can't reassign"
        );
        assert_eq!(deletion, before);
    }

    #[test]
    fn the_institutions_naming_a_unit_never_refuse_it() {
        let mut deletion = world();
        let mut usage = InMemoryUnitUsage::default();
        usage.uses.insert(
            BTC,
            UnitUse {
                institutions: 2,
                ..UnitUse::default()
            },
        );

        DeleteUnit {
            id: BTC,
            new_base: None,
            replacements: Vec::new(),
        }
        .run(&mut deletion, &usage)
        .expect("the Institutions are reassigned, not counted against it");
    }

    #[test]
    fn a_replacement_that_is_the_unit_gone_or_deactivated_is_refused() {
        let mut deletion = world();
        for replacement in [BTC, UnitId::new()] {
            assert_eq!(
                refused(
                    &mut deletion,
                    BTC,
                    None,
                    vec![Replacement {
                        institution: KRAKEN,
                        unit: replacement
                    }]
                ),
                Error::ReplacementGone
            );
        }

        deletion.units.units[2].is_active = false;
        assert_eq!(deletion.units.units[2].id, VAS);
        assert_eq!(
            refused(
                &mut deletion,
                BTC,
                None,
                vec![Replacement {
                    institution: KRAKEN,
                    unit: VAS
                }]
            ),
            Error::ReplacementGone
        );
    }

    #[test]
    fn a_replacement_for_an_institution_that_no_longer_names_the_unit_is_ignored() {
        let mut deletion = world();

        delete(
            &mut deletion,
            BTC,
            None,
            vec![
                Replacement {
                    institution: ANZ,
                    unit: VAS,
                },
                Replacement {
                    institution: WESTPAC,
                    unit: VAS,
                },
            ],
        )
        .expect("neither names btc");

        assert_eq!(usual_unit(&deletion, ANZ), None);
        assert_eq!(usual_unit(&deletion, WESTPAC), Some(AUD));
    }
}
