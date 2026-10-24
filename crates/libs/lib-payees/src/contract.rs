//! The contract every stored Payees port set keeps (ADR-0046), run against both the in-memory
//! ports here and the `lib-database` adapter (`adapter/payees.rs`), so the fakes the Presenter
//! tests use can be trusted.
//!
//! Each check is a function generic over a [`PortsFactory`]: a consumer runs it from its own test,
//! since an adapter's tests need their own set-up (a fresh database per test). Each writes through
//! the Payee rules, reloads as a restart would, and expects the same rows back. Until the Payees
//! Interactors land (#832) the writes here apply the rules to the rows read from the repository
//! and write back what changed, as `lib-ledger`'s commands do to the Ledger.

use lib_core::{CategoryId, PayeeId};

use crate::fake::InMemoryPorts;
use crate::protocols::PortsMut;
use crate::{
    DeletePayeeRefusal, Payee, PayeeDraft, PayeeError, PayeeUse, default_payees, delete_payee,
    edit_payee, find_by_name, get, insert_payee, set_active,
};

/// A stored set of the Payees ports, lent out together for one write.
pub trait StoredPorts {
    fn split_mut(&mut self) -> PortsMut<'_>;
}

/// Builds the ports a contract check runs on.
pub trait PortsFactory {
    type Ports: StoredPorts;

    /// Ports holding `seed`'s Payees with their aliases, as if stored and loaded, and nothing
    /// naming any of them.
    fn create(&mut self, seed: Vec<Payee>) -> Self::Ports;

    /// Ends the writes made on `ports`, then loads a fresh set from wherever they were stored, as
    /// a restart would.
    fn reload(&mut self, ports: Self::Ports) -> Self::Ports;

    /// Ends one write made on `ports`, sending its changes to storage as an adapter's owner does
    /// after each input.
    fn commit(&mut self, ports: &mut Self::Ports);

    /// Makes `uses.splits` more Splits and `uses.bill_plans` more Bill Plans name Payee `id`, as
    /// those domains' stored rows would, so [`crate::PayeeUsage`] has something to count.
    fn name_payee(&mut self, ports: &mut Self::Ports, id: PayeeId, uses: PayeeUse);

    /// Makes every write from now on fail, as a store that refuses it would.
    fn break_writes(&mut self, ports: &mut Self::Ports);

    /// Settles the writes sent so far, then returns `ports` reloaded to the last committed rows,
    /// with the number of failures reported naming the Payees domain.
    fn recover(&mut self, ports: Self::Ports) -> (Self::Ports, usize);
}

impl StoredPorts for InMemoryPorts {
    fn split_mut(&mut self) -> PortsMut<'_> {
        InMemoryPorts::split_mut(self)
    }
}

/// The in-memory ports are their own storage, so a reload hands them back as they are. It keeps
/// a copy as of the last commit, which a failed write falls back to.
#[derive(Debug, Default)]
pub struct InMemoryFactory {
    committed: Option<InMemoryPorts>,
    broken: bool,
    failures: usize,
}

impl PortsFactory for InMemoryFactory {
    type Ports = InMemoryPorts;

    fn create(&mut self, seed: Vec<Payee>) -> InMemoryPorts {
        let mut ports = InMemoryPorts::default();
        ports.payees.payees = seed;
        self.committed = Some(ports.clone());
        ports
    }

    fn reload(&mut self, mut ports: InMemoryPorts) -> InMemoryPorts {
        self.commit(&mut ports);
        ports
    }

    fn commit(&mut self, ports: &mut InMemoryPorts) {
        if self.broken {
            self.failures += 1;
        } else {
            self.committed = Some(ports.clone());
        }
    }

    /// Other domains' rows aren't the Payees writes, so the count lands in the committed copy too.
    fn name_payee(&mut self, ports: &mut InMemoryPorts, id: PayeeId, uses: PayeeUse) {
        let add = |ports: &mut InMemoryPorts| {
            let entry = ports.usage.uses.entry(id).or_default();
            entry.splits += uses.splits;
            entry.bill_plans += uses.bill_plans;
        };
        add(ports);
        if let Some(committed) = self.committed.as_mut() {
            add(committed);
        }
    }

    fn break_writes(&mut self, _ports: &mut InMemoryPorts) {
        self.broken = true;
    }

    fn recover(&mut self, _ports: InMemoryPorts) -> (InMemoryPorts, usize) {
        let committed = self.committed.clone().expect("create ran first");
        (committed, std::mem::take(&mut self.failures))
    }
}

/// The Payees a port set holds, by id, each with its aliases sorted: storage keeps which aliases a
/// Payee has, not the order they were typed in.
pub fn snapshot(ports: &mut impl StoredPorts) -> Vec<Payee> {
    let mut payees = ports.split_mut().payees.payees();
    payees.sort_by_key(|payee| payee.id);
    for payee in &mut payees {
        payee.aliases.sort();
    }
    payees
}

fn draft(name: &str, aliases: &[&str]) -> PayeeDraft {
    PayeeDraft {
        name: name.into(),
        default_category: None,
        aliases: aliases.iter().map(|alias| (*alias).into()).collect(),
    }
}

/// Adds a Payee by the rules, writing the new row.
fn add(ports: PortsMut<'_>, draft: &PayeeDraft) -> Result<PayeeId, PayeeError> {
    let mut rows = ports.payees.payees();
    let id = insert_payee(&mut rows, draft)?;
    let row = get(&rows, id).expect("the Payee was just added").clone();
    ports.payees.upsert_payee(row);
    Ok(id)
}

/// Edits a Payee by the rules, writing the edited row.
fn edit(ports: PortsMut<'_>, id: PayeeId, draft: &PayeeDraft) -> Result<(), PayeeError> {
    let mut rows = ports.payees.payees();
    edit_payee(&mut rows, id, draft)?;
    let row = get(&rows, id).expect("the Payee was just edited").clone();
    ports.payees.upsert_payee(row);
    Ok(())
}

/// Deletes a Payee by the rules, refused while anything names it.
fn delete(ports: PortsMut<'_>, id: PayeeId) -> Result<(), DeletePayeeRefusal> {
    let mut rows = ports.payees.payees();
    delete_payee(&mut rows, ports.usage, id)?;
    ports.payees.delete_payee(id);
    Ok(())
}

fn deactivate(ports: PortsMut<'_>, id: PayeeId) {
    let mut rows = ports.payees.payees();
    set_active(&mut rows, id, false).expect("the Payee is on the Ledger");
    let row = get(&rows, id).expect("the Payee is on the Ledger").clone();
    ports.payees.upsert_payee(row);
}

fn id_of(ports: &mut impl StoredPorts, name: &str) -> PayeeId {
    find_by_name(&ports.split_mut().payees.payees(), name).expect("a stub Payee")
}

/// Writes every kind of change through the rules, reloads, and gets the same rows: an added Payee
/// with aliases and a default Category, a rename that keeps the old name as an alias, and a
/// deactivation.
pub fn written_rows_reload_the_same<F: PortsFactory>(factory: &mut F) {
    let mut ports = factory.create(default_payees());

    let mut candles = draft("Aussie Candle Co", &["aussie candle", "SP AUSSIE CANDLE"]);
    candles.default_category = Some(CategoryId::seed(10));
    let candles = add(ports.split_mut(), &candles).expect("the name and aliases are free");
    factory.commit(&mut ports);
    let kmart = id_of(&mut ports, "Kmart");
    edit(
        ports.split_mut(),
        kmart,
        &draft("Kmart AU", &["KMART ONLINE"]),
    )
    .expect("the name and alias are free");
    factory.commit(&mut ports);
    let hudson = id_of(&mut ports, "Hudson News");
    deactivate(ports.split_mut(), hudson);
    factory.commit(&mut ports);

    let before = snapshot(&mut ports);
    let mut ports = factory.reload(ports);
    let after = snapshot(&mut ports);
    assert_eq!(after, before);

    let candles = get(&after, candles).expect("the added Payee reloads");
    assert_eq!(candles.aliases, ["AUSSIE CANDLE", "SP AUSSIE CANDLE"]);
    assert_eq!(candles.default_category, Some(CategoryId::seed(10)));
    let kmart = get(&after, kmart).expect("the renamed Payee reloads");
    assert_eq!(kmart.name, "Kmart AU");
    assert_eq!(kmart.aliases, ["KMART", "KMART ONLINE"]);
    assert!(
        !get(&after, hudson)
            .expect("deactivated, not gone")
            .is_active
    );
}

/// An alias another Payee owns, typed or taken by a rename, is refused before any write reaches
/// storage, whose unique alias would refuse it.
pub fn an_alias_another_payee_owns_is_refused_and_writes_nothing<F: PortsFactory>(factory: &mut F) {
    let mut ports = factory.create(default_payees());
    let before = snapshot(&mut ports);
    let woolworths_owns = PayeeError::AliasTaken {
        alias: "WOOLIES".into(),
        owner: "Woolworths".into(),
    };

    let refusal = add(ports.split_mut(), &draft("Woolies Metro", &["woolies"]))
        .expect_err("Woolworths owns WOOLIES");
    assert_eq!(refusal, woolworths_owns);
    factory.commit(&mut ports);
    let coles = id_of(&mut ports, "Coles");
    let refusal = edit(ports.split_mut(), coles, &draft("Woolies", &[]))
        .expect_err("a name is matched as an alias too");
    assert_eq!(refusal, woolworths_owns);
    factory.commit(&mut ports);

    assert_eq!(
        snapshot(&mut ports),
        before,
        "the refused saves wrote nothing"
    );
    let mut ports = factory.reload(ports);
    assert_eq!(snapshot(&mut ports), before);
}

/// Deleting a Payee takes its aliases with it, so another Payee can take one after a reload.
pub fn a_delete_takes_its_aliases<F: PortsFactory>(factory: &mut F) {
    let mut ports = factory.create(default_payees());
    let corner = add(
        ports.split_mut(),
        &draft("Corner Shop", &["CORNER", "CNR SHOP"]),
    )
    .expect("the name and aliases are free");
    let mut ports = factory.reload(ports);

    delete(ports.split_mut(), corner).expect("nothing names the Payee");
    let mut ports = factory.reload(ports);
    let after = snapshot(&mut ports);
    assert!(get(&after, corner).is_none());
    assert!(
        after
            .iter()
            .all(|payee| !payee.aliases.iter().any(|alias| alias == "CORNER")),
        "no Payee keeps the deleted one's alias"
    );

    let store = add(ports.split_mut(), &draft("Corner Store", &["corner"]))
        .expect("the deleted Payee's alias is free");
    let mut ports = factory.reload(ports);
    let after = snapshot(&mut ports);
    assert_eq!(
        get(&after, store).expect("the new Payee reloads").aliases,
        ["CORNER"]
    );
}

/// [`crate::PayeeUsage`] counts the Splits and Bill Plans naming a Payee, before and after a
/// reload, and a delete it counts against is refused with those counts and writes nothing, while
/// deactivating the Payee lands.
pub fn payee_usage_counts_and_refuses_the_delete<F: PortsFactory>(factory: &mut F) {
    let mut ports = factory.create(default_payees());
    let woolworths = id_of(&mut ports, "Woolworths");
    let coles = id_of(&mut ports, "Coles");
    let telstra = id_of(&mut ports, "Telstra");
    let named = PayeeUse {
        splits: 2,
        bill_plans: 1,
    };
    factory.name_payee(&mut ports, woolworths, named);
    factory.name_payee(
        &mut ports,
        telstra,
        PayeeUse {
            splits: 0,
            bill_plans: 1,
        },
    );
    let mut ports = factory.reload(ports);

    let usage = |ports: &mut F::Ports, id| ports.split_mut().usage.payee_usage(id);
    assert_eq!(usage(&mut ports, woolworths), named);
    assert!(usage(&mut ports, telstra).is_used());
    assert!(!usage(&mut ports, coles).is_used());

    let before = snapshot(&mut ports);
    assert_eq!(
        delete(ports.split_mut(), woolworths),
        Err(DeletePayeeRefusal::InUse {
            splits: 2,
            bill_plans: 1
        })
    );
    assert_eq!(
        delete(ports.split_mut(), telstra),
        Err(DeletePayeeRefusal::InUse {
            splits: 0,
            bill_plans: 1
        }),
        "a Bill Plan alone keeps a Payee"
    );
    factory.commit(&mut ports);
    assert_eq!(
        snapshot(&mut ports),
        before,
        "the refused deletes wrote nothing"
    );

    deactivate(ports.split_mut(), woolworths);
    let mut ports = factory.reload(ports);
    let after = snapshot(&mut ports);
    assert!(
        !get(&after, woolworths)
            .expect("deactivated, not gone")
            .is_active
    );
    assert_eq!(usage(&mut ports, woolworths), named);
}

/// A write that fails is reported once, naming the Payees domain, and the ports go back to the
/// last committed rows rather than keeping the change the store refused.
pub fn a_failed_write_reloads_and_reports<F: PortsFactory>(factory: &mut F) {
    let mut ports = factory.create(default_payees());
    let stored = snapshot(&mut ports);

    factory.break_writes(&mut ports);
    add(ports.split_mut(), &draft("Corner Shop", &["CORNER"])).expect("the name is free");
    factory.commit(&mut ports);

    let (mut ports, failures) = factory.recover(ports);
    assert_eq!(failures, 1, "one failure naming the Payees domain");
    assert_eq!(snapshot(&mut ports), stored, "the refused write is gone");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_memory_written_rows_reload_the_same() {
        written_rows_reload_the_same(&mut InMemoryFactory::default());
    }

    #[test]
    fn in_memory_an_alias_another_payee_owns_is_refused_and_writes_nothing() {
        an_alias_another_payee_owns_is_refused_and_writes_nothing(&mut InMemoryFactory::default());
    }

    #[test]
    fn in_memory_a_delete_takes_its_aliases() {
        a_delete_takes_its_aliases(&mut InMemoryFactory::default());
    }

    #[test]
    fn in_memory_payee_usage_counts_and_refuses_the_delete() {
        payee_usage_counts_and_refuses_the_delete(&mut InMemoryFactory::default());
    }

    #[test]
    fn in_memory_a_failed_write_reloads_and_reports() {
        a_failed_write_reloads_and_reports(&mut InMemoryFactory::default());
    }
}
