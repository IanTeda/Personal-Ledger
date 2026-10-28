//! The in-memory deletion port, for tests and UI-first work, and [`world`], the small Ledger the
//! tests and the contract delete from. It holds its rows in plain fields so a test can set up or
//! read state directly. The Units Dialog's Router is `lib-units`' own recording one.

use lib_core::UnitId;
use lib_institutions::fake::InMemoryInstitutions;
use lib_institutions::{Institution, InstitutionRepository};
use lib_units::fake::{InMemoryBaseUnit, InMemoryUnits};
use lib_units::{BaseUnitRepository, UnitRepository, UnitRow};

use crate::protocols::{UnitDeletion, UnitRemoval, naming};

/// A delete's two stores in memory, the Units with the base and the Institutions, written
/// together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InMemoryUnitDeletion {
    pub units: InMemoryUnits,
    pub base_unit: InMemoryBaseUnit,
    pub institutions: InMemoryInstitutions,
}

impl UnitDeletion for InMemoryUnitDeletion {
    fn units(&self) -> Vec<UnitRow> {
        self.units.units()
    }

    fn base_unit(&self) -> UnitId {
        self.base_unit.base_unit()
    }

    fn institutions_naming(&self, unit: UnitId) -> Vec<Institution> {
        naming(self.institutions.institutions(), unit)
    }

    fn remove_unit(&mut self, removal: UnitRemoval) {
        for institution in removal.reassigned {
            self.institutions.upsert_institution(institution);
        }
        if let Some(new_base) = removal.new_base {
            self.base_unit.set_base_unit(new_base);
        }
        self.units.delete_unit(removal.unit);
    }
}

/// The small Ledger the tests and the contract delete from: the stub's aud, btc and vas, aud the
/// base, with nothing quoted in aud so it can go; Westpac names aud as its Usual unit, two crypto
/// exchanges name btc, ANZ names none, and nothing names vas.
pub mod world {
    use chrono::{DateTime, TimeZone, Utc};
    use lib_core::{InstitutionId, UnitId};
    use lib_institutions::Institution;
    use lib_institutions::fake::InMemoryInstitutions;
    use lib_units::fake::{InMemoryBaseUnit, InMemoryUnits};

    use super::InMemoryUnitDeletion;

    /// The base Unit, and Westpac's Usual unit.
    pub const AUD: UnitId = UnitId::SEED_AUD;
    /// Both exchanges' Usual unit.
    pub const BTC: UnitId = UnitId::SEED_BTC;
    /// Nothing names it.
    pub const VAS: UnitId = UnitId::SEED_VAS;
    pub const WESTPAC: InstitutionId = InstitutionId::SEED_WESTPAC;
    pub const ANZ: InstitutionId = InstitutionId::SEED_ANZ;
    pub const COINSPOT: InstitutionId = InstitutionId::seed(101);
    pub const KRAKEN: InstitutionId = InstitutionId::seed(102);

    /// The instant the world's Units were priced at.
    pub fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 21, 9, 30, 0)
            .single()
            .unwrap_or_default()
    }

    fn institution(id: InstitutionId, name: &str, usual_unit: Option<UnitId>) -> Institution {
        Institution {
            id,
            name: name.to_string(),
            products: "savings".to_string(),
            usual_unit,
        }
    }

    /// The Units and Institutions described above.
    pub fn world() -> InMemoryUnitDeletion {
        let mut units = lib_units::default_units(now());
        // Each Unit quoted in aud is priced in itself instead, so the base can be deleted: the
        // stub quotes btc and vas in aud, which a delete refuses.
        for unit in &mut units {
            if unit.link.quote == AUD {
                unit.link.quote = unit.id;
            }
        }
        InMemoryUnitDeletion {
            units: InMemoryUnits { units },
            base_unit: InMemoryBaseUnit(AUD),
            institutions: InMemoryInstitutions {
                institutions: vec![
                    institution(ANZ, "ANZ Banking Group", None),
                    institution(WESTPAC, "Westpac", Some(AUD)),
                    institution(COINSPOT, "Coinspot", Some(BTC)),
                    institution(KRAKEN, "Kraken", Some(BTC)),
                ],
            },
        }
    }
}
