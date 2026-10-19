//! The contract every [`LedgerRepository`] meets (ADR-0044): load, run a command, apply the changes
//! it recorded, load again, and get back what the in-memory Ledger holds. Behind the
//! `contract-tests` feature so each adapter's tests run the same suite; a failure panics, as any
//! test assertion does.
//!
//! Only the Units are checked, since they are the only domain persisted so far. Each scenario adds
//! its own Units rather than touching the seeded ones, so it holds whatever a fresh repository was
//! seeded with.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "a contract test reports a failure by panicking, like any test"
)]

use std::fmt::Debug;

use lib_core::{UnitId, UnitKind};
use lib_units::PriceLink;

use crate::{
    AddUnit, DeactivateUnit, DeleteUnit, EditUnit, Ledger, LedgerCommand, LedgerRepository,
    ReactivateUnit, SetBaseUnit,
};

/// Runs every Units scenario, each on a fresh repository from `new_repository`.
pub fn units<R: LedgerRepository>(new_repository: impl Fn() -> R) {
    add_unit(&new_repository());
    edit_unit(&new_repository());
    deactivate_and_reactivate_unit(&new_repository());
    delete_unit(&new_repository());
    set_base_unit(&new_repository());
    delete_base_unit_with_new_base(&new_repository());
}

pub fn add_unit(repository: &impl LedgerRepository) {
    let mut ledger = load(repository);
    add(repository, &mut ledger, "eth");
}

pub fn edit_unit(repository: &impl LedgerRepository) {
    let mut ledger = load(repository);
    let id = add(repository, &mut ledger, "eth");
    let edit = EditUnit {
        id,
        code: "ether".into(),
        name: "Ether".into(),
        kind: UnitKind::Currency,
        link: PriceLink::manual(ledger.base_unit()),
    };
    round_trip(repository, &mut ledger, edit, "edit a Unit");
}

pub fn deactivate_and_reactivate_unit(repository: &impl LedgerRepository) {
    let mut ledger = load(repository);
    let id = add(repository, &mut ledger, "eth");
    round_trip(
        repository,
        &mut ledger,
        DeactivateUnit(id),
        "deactivate a Unit",
    );
    round_trip(
        repository,
        &mut ledger,
        ReactivateUnit(id),
        "reactivate a Unit",
    );
}

pub fn delete_unit(repository: &impl LedgerRepository) {
    let mut ledger = load(repository);
    let id = add(repository, &mut ledger, "eth");
    let delete = DeleteUnit { id, new_base: None };
    round_trip(repository, &mut ledger, delete, "delete a Unit");
}

pub fn set_base_unit(repository: &impl LedgerRepository) {
    let mut ledger = load(repository);
    let id = add(repository, &mut ledger, "eth");
    round_trip(
        repository,
        &mut ledger,
        SetBaseUnit(id),
        "set the base Unit",
    );
}

/// One batch that moves the base and then deletes the old one, so a repository that applied it out
/// of order would be left without a base.
pub fn delete_base_unit_with_new_base(repository: &impl LedgerRepository) {
    let mut ledger = load(repository);
    let old_base = add(repository, &mut ledger, "eth");
    round_trip(
        repository,
        &mut ledger,
        SetBaseUnit(old_base),
        "set the base Unit",
    );
    let new_base = add(repository, &mut ledger, "sol");
    let delete = DeleteUnit {
        id: old_base,
        new_base: Some(new_base),
    };
    round_trip(repository, &mut ledger, delete, "delete the base Unit");
    assert_eq!(ledger.base_unit(), new_base);
}

fn load(repository: &impl LedgerRepository) -> Ledger {
    repository.load().expect("a fresh repository loads")
}

/// Adds an unused crypto Unit coded `code` through the round trip and returns its id.
fn add(repository: &impl LedgerRepository, ledger: &mut Ledger, code: &str) -> UnitId {
    let add = AddUnit {
        code: code.into(),
        name: code.to_uppercase(),
        kind: UnitKind::Crypto,
        link: PriceLink::manual(ledger.base_unit()),
    };
    round_trip(repository, ledger, add, "add a Unit");
    ledger
        .units()
        .iter()
        .find(|unit| unit.code == code)
        .expect("the Unit just added is on the Ledger")
        .id
}

/// Runs `command`, applies what it recorded, reloads, and checks the reload holds the same Units
/// and base Unit as `ledger`.
fn round_trip<C>(repository: &impl LedgerRepository, ledger: &mut Ledger, command: C, step: &str)
where
    C: LedgerCommand,
    C::Refusal: Debug,
{
    if let Err(refusal) = command.run(ledger) {
        panic!("{step}: the command was refused: {refusal:?}");
    }
    let changes = ledger.drain_changes();
    if let Err(error) = repository.apply(&changes) {
        panic!("{step}: apply failed: {error}");
    }
    let reloaded = match repository.load() {
        Ok(reloaded) => reloaded,
        Err(error) => panic!("{step}: the reload failed: {error}"),
    };
    assert_eq!(
        reloaded.base_unit(),
        ledger.base_unit(),
        "{step}: the base Unit"
    );
    // Row order is the repository's own, so the Units are matched by id.
    assert_eq!(
        reloaded.units().len(),
        ledger.units().len(),
        "{step}: the number of Units"
    );
    for unit in ledger.units() {
        assert_eq!(
            reloaded.unit(unit.id),
            Some(unit),
            "{step}: Unit {}",
            unit.code
        );
    }
}
