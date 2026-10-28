//! The contract every stored Institutions port keeps (ADR-0046), run against both the in-memory
//! port here and the `lib-database` adapter (#849), so the fake the tests use can be trusted.
//!
//! Each check is a function generic over a [`PortsFactory`]: a consumer runs it from its own test,
//! since an adapter's tests need their own set-up (a fresh database per test). Each writes through
//! the Institutions Interactors, as a Client does, reloads as a restart would, and expects the same
//! rows back, in the same order.

use lib_core::InstitutionId;

use crate::fake::InMemoryInstitutions;
use crate::interactor::{AddInstitution, EditInstitution};
use crate::{
    Institution, InstitutionDraft, InstitutionError, InstitutionRepository, default_institutions,
};

/// Builds the port a contract check runs on.
pub trait PortsFactory {
    type Ports: InstitutionRepository;

    /// A port holding `seed`'s Institutions, in order, as if stored and loaded.
    fn create(&mut self, seed: Vec<Institution>) -> Self::Ports;

    /// Ends the writes made on `ports`, then loads a fresh port from wherever they were stored, as
    /// a restart would.
    fn reload(&mut self, ports: Self::Ports) -> Self::Ports;

    /// Ends one write made on `ports`, sending its changes to storage as an adapter's owner does
    /// after each input.
    fn commit(&mut self, ports: &mut Self::Ports);

    /// Makes every write from now on fail, as a store that refuses it would.
    fn break_writes(&mut self, ports: &mut Self::Ports);

    /// Settles the writes sent so far, then returns `ports` reloaded to the last committed rows,
    /// with the number of failures reported naming the Institutions domain.
    fn recover(&mut self, ports: Self::Ports) -> (Self::Ports, usize);
}

/// The in-memory port is its own storage, so a reload hands it back as it is. It keeps a copy as of
/// the last commit, which a failed write falls back to.
#[derive(Debug, Default)]
pub struct InMemoryFactory {
    committed: Option<InMemoryInstitutions>,
    broken: bool,
    failures: usize,
}

impl PortsFactory for InMemoryFactory {
    type Ports = InMemoryInstitutions;

    fn create(&mut self, seed: Vec<Institution>) -> InMemoryInstitutions {
        let ports = InMemoryInstitutions { institutions: seed };
        self.committed = Some(ports.clone());
        ports
    }

    fn reload(&mut self, mut ports: InMemoryInstitutions) -> InMemoryInstitutions {
        self.commit(&mut ports);
        ports
    }

    fn commit(&mut self, ports: &mut InMemoryInstitutions) {
        if self.broken {
            self.failures += 1;
        } else {
            self.committed = Some(ports.clone());
        }
    }

    fn break_writes(&mut self, _ports: &mut InMemoryInstitutions) {
        self.broken = true;
    }

    fn recover(&mut self, _ports: InMemoryInstitutions) -> (InMemoryInstitutions, usize) {
        let committed = self.committed.clone().expect("create ran first");
        (committed, std::mem::take(&mut self.failures))
    }
}

fn draft(name: &str, products: &str) -> InstitutionDraft {
    InstitutionDraft {
        name: name.into(),
        products: products.into(),
    }
}

fn add(ports: &mut dyn InstitutionRepository, draft: InstitutionDraft) -> Institution {
    AddInstitution(draft).run(ports).expect("the name is free")
}

/// An added Institution reloads with its id, trimmed name and joined Products label.
pub fn an_added_institution_reloads_the_same<F: PortsFactory>(factory: &mut F) {
    let mut ports = factory.create(default_institutions());

    let added = add(&mut ports, draft("  Bank X ", "savings \u{b7} offset"));
    factory.commit(&mut ports);

    let before = ports.institutions();
    let ports = factory.reload(ports);
    assert_eq!(ports.institutions(), before);
    assert_eq!(
        ports.institution(added.id),
        Some(Institution {
            id: added.id,
            name: "Bank X".into(),
            products: "savings \u{b7} offset".into(),
        })
    );
}

/// Institutions list in the order they were added, the seed's first, before and after a reload.
pub fn institutions_list_in_the_order_added<F: PortsFactory>(factory: &mut F) {
    let mut ports = factory.create(default_institutions());

    let zeta = add(&mut ports, draft("Zeta Credit Union", "savings"));
    factory.commit(&mut ports);
    let alpha = add(&mut ports, draft("Alpha Bank", "loan"));
    factory.commit(&mut ports);

    let mut expected: Vec<_> = default_institutions()
        .into_iter()
        .map(|institution| institution.id)
        .collect();
    expected.extend([zeta.id, alpha.id]);
    let ids = |ports: &F::Ports| -> Vec<_> {
        ports
            .institutions()
            .into_iter()
            .map(|institution| institution.id)
            .collect()
    };
    assert_eq!(ids(&ports), expected);
    let ports = factory.reload(ports);
    assert_eq!(ids(&ports), expected, "a reload keeps the order");
}

/// A blank name, or one another Institution holds in any case, is refused before any write
/// reaches storage, whose unique name would refuse it.
pub fn a_blank_or_taken_name_is_refused_and_writes_nothing<F: PortsFactory>(factory: &mut F) {
    let mut ports = factory.create(default_institutions());
    let before = ports.institutions();

    assert_eq!(
        AddInstitution(draft(" anz banking group ", "savings")).run(&mut ports),
        Err(InstitutionError::DuplicateName("ANZ Banking Group".into()))
    );
    assert_eq!(
        AddInstitution(draft("  ", "savings")).run(&mut ports),
        Err(InstitutionError::NameRequired)
    );
    factory.commit(&mut ports);

    assert_eq!(
        ports.institutions(),
        before,
        "the refused adds wrote nothing"
    );
    let ports = factory.reload(ports);
    assert_eq!(ports.institutions(), before);
}

/// An edited Institution reloads with its new name and Products, in the place it held.
pub fn an_edited_institution_reloads_the_same<F: PortsFactory>(factory: &mut F) {
    let mut ports = factory.create(default_institutions());

    let edited = EditInstitution {
        id: InstitutionId::SEED_AMEX,
        draft: draft(" Amex ", "credit card \u{b7} cash"),
    }
    .run(&mut ports)
    .expect("a free name");
    factory.commit(&mut ports);

    let before = ports.institutions();
    assert_eq!(before[2], edited);
    let ports = factory.reload(ports);
    assert_eq!(ports.institutions(), before);
    assert_eq!(ports.institutions().len(), default_institutions().len());
    assert_eq!(ports.institution(edited.id), Some(edited));
}

/// A rename to a name another Institution holds, in any case, is refused and writes nothing; a
/// blank name is too.
pub fn a_clashing_rename_is_refused_and_writes_nothing<F: PortsFactory>(factory: &mut F) {
    let mut ports = factory.create(default_institutions());
    let before = ports.institutions();

    assert_eq!(
        EditInstitution {
            id: InstitutionId::SEED_AMEX,
            draft: draft(" anz banking group ", "loan"),
        }
        .run(&mut ports),
        Err(InstitutionError::DuplicateName("ANZ Banking Group".into()))
    );
    assert_eq!(
        EditInstitution {
            id: InstitutionId::SEED_AMEX,
            draft: draft("  ", "loan"),
        }
        .run(&mut ports),
        Err(InstitutionError::NameRequired)
    );
    factory.commit(&mut ports);

    assert_eq!(
        ports.institutions(),
        before,
        "the refused edits wrote nothing"
    );
    let ports = factory.reload(ports);
    assert_eq!(ports.institutions(), before);
}

/// A rename that changes only the capitals of the Institution's own name is accepted.
pub fn a_case_only_rename_is_accepted<F: PortsFactory>(factory: &mut F) {
    let mut ports = factory.create(default_institutions());

    EditInstitution {
        id: InstitutionId::SEED_WESTPAC,
        draft: draft("WESTPAC BANKING", "savings"),
    }
    .run(&mut ports)
    .expect("its own name in other capitals");
    factory.commit(&mut ports);

    let ports = factory.reload(ports);
    assert_eq!(
        ports
            .institution(InstitutionId::SEED_WESTPAC)
            .map(|institution| institution.name),
        Some("WESTPAC BANKING".to_string())
    );
}

/// A write that fails is reported once, naming the Institutions domain, and the port goes back to
/// the last committed rows rather than keeping the change the store refused.
pub fn a_failed_write_reloads_and_reports<F: PortsFactory>(factory: &mut F) {
    let mut ports = factory.create(default_institutions());
    let stored = ports.institutions();

    factory.break_writes(&mut ports);
    add(&mut ports, draft("Bank X", "savings"));
    factory.commit(&mut ports);

    let (ports, failures) = factory.recover(ports);
    assert_eq!(failures, 1, "one failure naming the Institutions domain");
    assert_eq!(ports.institutions(), stored, "the refused write is gone");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_memory_an_added_institution_reloads_the_same() {
        an_added_institution_reloads_the_same(&mut InMemoryFactory::default());
    }

    #[test]
    fn in_memory_institutions_list_in_the_order_added() {
        institutions_list_in_the_order_added(&mut InMemoryFactory::default());
    }

    #[test]
    fn in_memory_a_blank_or_taken_name_is_refused_and_writes_nothing() {
        a_blank_or_taken_name_is_refused_and_writes_nothing(&mut InMemoryFactory::default());
    }

    #[test]
    fn in_memory_an_edited_institution_reloads_the_same() {
        an_edited_institution_reloads_the_same(&mut InMemoryFactory::default());
    }

    #[test]
    fn in_memory_a_clashing_rename_is_refused_and_writes_nothing() {
        a_clashing_rename_is_refused_and_writes_nothing(&mut InMemoryFactory::default());
    }

    #[test]
    fn in_memory_a_case_only_rename_is_accepted() {
        a_case_only_rename_is_accepted(&mut InMemoryFactory::default());
    }

    #[test]
    fn in_memory_a_failed_write_reloads_and_reports() {
        a_failed_write_reloads_and_reports(&mut InMemoryFactory::default());
    }
}
