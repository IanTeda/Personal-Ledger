//! The Payees writes (ADR-0038): each a command value with its own refusal, checked before
//! anything is written so a refusal leaves the Ledger untouched, and recording an upsert or delete
//! per Payee it wrote (ADR-0039).
//!
//! Single-Payee rules (names, aliases) stay in `lib_payees`. What lives here is the rule that
//! crosses domains: a Payee that a Split or a Bill Plan names can only be deactivated.

use lib_payees::{
    Payee, PayeeDraft, PayeeError, clean_name, delete_payee, edit_payee, get, insert_payee,
};

use crate::{Ledger, LedgerChange, LedgerCommand};

/// How many rows name a Payee, so a Dialog can offer Delete or only Deactivate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PayeeUse {
    pub splits: usize,
    pub bill_plans: usize,
}

impl PayeeUse {
    pub fn is_used(self) -> bool {
        self.splits + self.bill_plans > 0
    }
}

/// Why a Payee delete was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DeletePayeeRefusal {
    #[error("payee not found")]
    NotFound,
    /// A Split or a Bill Plan names the Payee, so it can only be deactivated.
    #[error("a payee in use can only be deactivated")]
    InUse { splits: usize, bill_plans: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SetPayeeActiveRefusal {
    #[error("payee not found")]
    NotFound,
}

/// Why an Import's Payees could not be merged in.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ImportPayeesRefusal {
    /// The Ledger changed under the working copy: a new row's id is already taken.
    #[error("payee id {0} is already taken")]
    IdTaken(u32),
    /// A row named an existing Payee but changed more than its Aliases.
    #[error("payee id {0} changed beyond its aliases")]
    NotAliasOnly(u32),
    #[error(transparent)]
    Payee(#[from] PayeeError),
}

impl Ledger {
    /// How many Splits and Bill Plans name Payee `id`.
    pub fn payee_use(&self, id: u32) -> PayeeUse {
        PayeeUse {
            splits: self
                .transactions
                .iter()
                .flat_map(|t| &t.splits)
                .filter(|split| split.payee_id == Some(id))
                .count(),
            bill_plans: self
                .bill_plans
                .iter()
                .filter(|plan| plan.payee_id == Some(id))
                .count(),
        }
    }

    /// Adds a new, active Payee and returns its id.
    pub fn add_payee(&mut self, draft: &PayeeDraft) -> Result<u32, PayeeError> {
        let id = insert_payee(&mut self.payees, draft)?;
        self.record_payee(id);
        Ok(id)
    }

    pub fn edit_payee(&mut self, id: u32, draft: &PayeeDraft) -> Result<(), PayeeError> {
        edit_payee(&mut self.payees, id, draft)?;
        self.record_payee(id);
        Ok(())
    }

    /// Hard-deletes Payee `id`, refused while a Split or a Bill Plan names it.
    pub fn delete_payee(&mut self, id: u32) -> Result<(), DeletePayeeRefusal> {
        get(&self.payees, id).ok_or(DeletePayeeRefusal::NotFound)?;
        let PayeeUse { splits, bill_plans } = self.payee_use(id);
        if splits + bill_plans > 0 {
            return Err(DeletePayeeRefusal::InUse { splits, bill_plans });
        }
        delete_payee(&mut self.payees, id).map_err(|_| DeletePayeeRefusal::NotFound)?;
        self.pending.push(LedgerChange::DeletePayee(id));
        Ok(())
    }

    pub fn set_payee_active(
        &mut self,
        id: u32,
        is_active: bool,
    ) -> Result<(), SetPayeeActiveRefusal> {
        lib_payees::set_active(&mut self.payees, id, is_active)
            .map_err(|_| SetPayeeActiveRefusal::NotFound)?;
        self.record_payee(id);
        Ok(())
    }

    /// Merges the Payees an Import's working copy created or gave Aliases to. `rows` is the whole
    /// Payees list after the Import's rules ran; only rows that are new or differ are written.
    pub fn import_payees(&mut self, rows: &[Payee]) -> Result<Vec<u32>, ImportPayeesRefusal> {
        let mut next = self.payees.clone();
        let mut written = Vec::new();
        for row in rows {
            match get(&next, row.id) {
                Some(existing) if existing == row => {}
                Some(existing) => {
                    let only_aliases = Payee {
                        aliases: row.aliases.clone(),
                        ..existing.clone()
                    } == *row
                        && existing.aliases.iter().all(|a| row.aliases.contains(a));
                    if !only_aliases {
                        return Err(ImportPayeesRefusal::NotAliasOnly(row.id));
                    }
                    next.iter_mut()
                        .filter(|p| p.id == row.id)
                        .for_each(|p| p.aliases = row.aliases.clone());
                    written.push(row.id);
                }
                None => {
                    if next.iter().any(|p| p.id == row.id) {
                        return Err(ImportPayeesRefusal::IdTaken(row.id));
                    }
                    clean_name(&next, None, &row.name)?;
                    next.push(row.clone());
                    written.push(row.id);
                }
            }
        }
        self.payees = next;
        for id in &written {
            self.record_payee(*id);
        }
        Ok(written)
    }

    fn record_payee(&mut self, id: u32) {
        if let Some(payee) = get(&self.payees, id) {
            self.pending.push(LedgerChange::UpsertPayee(payee.clone()));
        }
    }
}

/// [`Ledger::add_payee`] as a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddPayee(pub PayeeDraft);

impl LedgerCommand for AddPayee {
    type Outcome = u32;
    type Refusal = PayeeError;
    fn run(self, ledger: &mut Ledger) -> Result<u32, PayeeError> {
        ledger.add_payee(&self.0)
    }
}

/// [`Ledger::edit_payee`] as a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditPayee {
    pub id: u32,
    pub draft: PayeeDraft,
}

impl LedgerCommand for EditPayee {
    type Outcome = ();
    type Refusal = PayeeError;
    fn run(self, ledger: &mut Ledger) -> Result<(), PayeeError> {
        ledger.edit_payee(self.id, &self.draft)
    }
}

/// [`Ledger::delete_payee`] as a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeletePayee(pub u32);

impl LedgerCommand for DeletePayee {
    type Outcome = ();
    type Refusal = DeletePayeeRefusal;
    fn run(self, ledger: &mut Ledger) -> Result<(), DeletePayeeRefusal> {
        ledger.delete_payee(self.0)
    }
}

/// [`Ledger::set_payee_active`] as a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetPayeeActive {
    pub id: u32,
    pub is_active: bool,
}

impl LedgerCommand for SetPayeeActive {
    type Outcome = ();
    type Refusal = SetPayeeActiveRefusal;
    fn run(self, ledger: &mut Ledger) -> Result<(), SetPayeeActiveRefusal> {
        ledger.set_payee_active(self.id, self.is_active)
    }
}

/// [`Ledger::import_payees`] as a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportPayees(pub Vec<Payee>);

impl LedgerCommand for ImportPayees {
    type Outcome = Vec<u32>;
    type Refusal = ImportPayeesRefusal;
    fn run(self, ledger: &mut Ledger) -> Result<Vec<u32>, ImportPayeesRefusal> {
        ledger.import_payees(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft(name: &str) -> PayeeDraft {
        PayeeDraft {
            name: name.to_owned(),
            aliases: Vec::new(),
            default_category: None,
        }
    }

    fn id_of(ledger: &Ledger, name: &str) -> u32 {
        lib_payees::find_by_name(ledger.payees(), name).expect("seeded Payee")
    }

    #[test]
    fn add_records_one_upsert_of_the_row_now_in_the_ledger() {
        let mut ledger = Ledger::stub();
        let id = AddPayee(draft("Corner Shop")).run(&mut ledger).unwrap();

        let row = get(ledger.payees(), id).unwrap().clone();
        assert_eq!(ledger.drain_changes(), vec![LedgerChange::UpsertPayee(row)]);
    }

    #[test]
    fn a_refused_add_or_edit_changes_nothing_and_records_nothing() {
        let mut ledger = Ledger::stub();
        let before = ledger.payees().to_vec();
        let taken = before[0].name.clone();

        assert!(AddPayee(draft(&taken)).run(&mut ledger).is_err());
        assert!(
            EditPayee {
                id: u32::MAX,
                draft: draft("x")
            }
            .run(&mut ledger)
            .is_err()
        );
        assert_eq!(ledger.payees(), before);
        assert!(ledger.drain_changes().is_empty());
    }

    #[test]
    fn edit_records_the_edited_row() {
        let mut ledger = Ledger::stub();
        let id = id_of(&ledger, "Netflix");
        EditPayee {
            id,
            draft: draft("Netflix AU"),
        }
        .run(&mut ledger)
        .unwrap();

        let row = get(ledger.payees(), id).unwrap().clone();
        assert_eq!(row.name, "Netflix AU");
        assert_eq!(ledger.drain_changes(), vec![LedgerChange::UpsertPayee(row)]);
    }

    #[test]
    fn an_unused_payee_deletes_and_records_the_delete() {
        let mut ledger = Ledger::stub();
        let id = ledger.add_payee(&draft("Corner Shop")).unwrap();
        ledger.drain_changes();
        assert!(!ledger.payee_use(id).is_used());

        DeletePayee(id).run(&mut ledger).unwrap();

        assert!(get(ledger.payees(), id).is_none());
        assert_eq!(ledger.drain_changes(), vec![LedgerChange::DeletePayee(id)]);
    }

    #[test]
    fn a_payee_a_split_names_is_refused_and_the_ledger_is_untouched() {
        let mut ledger = Ledger::stub();
        let id = id_of(&ledger, "J Smith");
        let before = ledger.payees().to_vec();
        let used = ledger.payee_use(id);
        assert!(used.splits > 0);

        assert_eq!(
            DeletePayee(id).run(&mut ledger),
            Err(DeletePayeeRefusal::InUse {
                splits: used.splits,
                bill_plans: used.bill_plans
            })
        );
        assert_eq!(ledger.payees(), before);
        assert!(ledger.drain_changes().is_empty());
    }

    #[test]
    fn a_payee_only_a_bill_plan_names_is_in_use() {
        let mut ledger = Ledger::stub();
        let plan = ledger
            .bill_plans()
            .iter()
            .find(|plan| plan.payee_id.is_some())
            .expect("the stub has a Bill Plan with a Payee");
        let id = plan.payee_id.unwrap();
        ledger.transactions.clear();

        let used = ledger.payee_use(id);
        assert_eq!(used.splits, 0);
        assert!(used.bill_plans > 0);
        assert!(matches!(
            DeletePayee(id).run(&mut ledger),
            Err(DeletePayeeRefusal::InUse { splits: 0, .. })
        ));
    }

    #[test]
    fn deleting_an_unknown_payee_is_not_found() {
        let mut ledger = Ledger::stub();
        assert_eq!(
            DeletePayee(u32::MAX).run(&mut ledger),
            Err(DeletePayeeRefusal::NotFound)
        );
    }

    #[test]
    fn set_active_records_the_row_and_refuses_an_unknown_id() {
        let mut ledger = Ledger::stub();
        let id = id_of(&ledger, "J Smith");
        SetPayeeActive {
            id,
            is_active: false,
        }
        .run(&mut ledger)
        .unwrap();

        let row = get(ledger.payees(), id).unwrap().clone();
        assert!(!row.is_active);
        assert_eq!(ledger.drain_changes(), vec![LedgerChange::UpsertPayee(row)]);
        assert_eq!(
            SetPayeeActive {
                id: u32::MAX,
                is_active: true
            }
            .run(&mut ledger),
            Err(SetPayeeActiveRefusal::NotFound)
        );
        assert!(ledger.drain_changes().is_empty());
    }

    #[test]
    fn import_writes_only_new_and_alias_extended_rows() {
        let mut ledger = Ledger::stub();
        let mut rows = ledger.payees().to_vec();
        let id = rows[0].id;
        rows[0].aliases.push("NEW TOKEN".into());
        let new_id = rows.iter().map(|p| p.id).max().unwrap() + 1;
        rows.push(Payee {
            id: new_id,
            name: "Aussie Candle Co".into(),
            aliases: vec![],
            default_category: None,
            is_active: true,
        });

        let written = ImportPayees(rows).run(&mut ledger).unwrap();

        assert_eq!(written, vec![id, new_id]);
        assert_eq!(ledger.drain_changes().len(), 2);
        assert!(
            get(ledger.payees(), id)
                .unwrap()
                .aliases
                .contains(&"NEW TOKEN".to_owned())
        );
    }

    #[test]
    fn a_refused_import_leaves_the_ledger_untouched() {
        let mut ledger = Ledger::stub();
        let before = ledger.payees().to_vec();
        let mut rows = before.clone();
        rows[0].name = "Renamed".into();
        assert_eq!(
            ImportPayees(rows).run(&mut ledger),
            Err(ImportPayeesRefusal::NotAliasOnly(before[0].id))
        );

        let mut dup = before.clone();
        dup.push(Payee {
            id: 9_999,
            name: before[1].name.clone(),
            aliases: vec![],
            default_category: None,
            is_active: true,
        });
        assert!(matches!(
            ImportPayees(dup).run(&mut ledger),
            Err(ImportPayeesRefusal::Payee(PayeeError::DuplicateName(_)))
        ));
        assert_eq!(ledger.payees(), before);
        assert!(ledger.drain_changes().is_empty());
    }

    #[test]
    fn an_in_use_payee_can_be_deactivated_and_the_change_is_recorded() {
        let mut ledger = Ledger::stub();
        let id = id_of(&ledger, "J Smith");
        assert!(ledger.payee_use(id).is_used());

        SetPayeeActive {
            id,
            is_active: false,
        }
        .run(&mut ledger)
        .unwrap();

        assert!(!get(ledger.payees(), id).unwrap().is_active);
        assert_eq!(ledger.drain_changes().len(), 1);
    }

    #[test]
    fn an_import_row_reusing_an_id_with_other_fields_is_refused() {
        let mut ledger = Ledger::stub();
        let before = ledger.payees().to_vec();
        let mut rows = before.clone();
        rows[0].is_active = !rows[0].is_active;

        assert_eq!(
            ImportPayees(rows).run(&mut ledger),
            Err(ImportPayeesRefusal::NotAliasOnly(before[0].id))
        );
        assert_eq!(ledger.payees(), before);
    }
}
