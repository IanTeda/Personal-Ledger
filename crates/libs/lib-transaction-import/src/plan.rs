//! The resolved plan an Import's commit sends through [`TransactionImport`]: the new Payees with
//! their normalised aliases and one Transaction per row. Resolving runs the rules the Ledger's
//! `CommitImport` ran, so the port only checks the plan against what is stored and writes it.
//!
//! [`TransactionImport`]: crate::TransactionImport

use lib_core::{AccountId, PayeeId, TransactionId, TransactionStatus};
use lib_payees::{self, Payee};
use lib_transactions::{Split, Transaction};

use crate::entity::{ImportRow, RowPayee, cleaned_token};

/// Everything one Import writes, as one write.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportPlan {
    /// The Payees to create, each once however many rows name it, with the aliases to remember.
    pub payees: Vec<Payee>,
    /// One single-Split Transaction per row, in the statement's order.
    pub transactions: Vec<Transaction>,
}

/// Why rows can't become a plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PlanError {
    #[error("{0} rows still need review")]
    NeedsReview(usize),
}

/// Why the port refused a plan, which it then wrote none of. A variant carries the name or alias
/// the Presenter's Message needs, but the `#[error]` text never repeats it: it is for logs, which
/// never hold a Payee's name.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ImportRefused {
    /// A new Payee's name is another Payee's name or alias, case-insensitively.
    #[error("a new payee's name is taken")]
    PayeeNameTaken(String),
    /// A new Payee's alias is another Payee's, or isn't stored-normalised.
    #[error("a new payee's alias is taken or not normalised")]
    AliasTaken(String),
}

/// What a commit wrote, for its Toast.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImportCommitted {
    pub transactions: usize,
    pub new_payees: usize,
}

impl ImportPlan {
    /// Resolves `rows` into a plan booking to `account_id`: each new name becomes one Payee, given
    /// its row's cleaned token as an alias when `remember` and no Payee owns it yet (#282).
    /// `payees` is every stored Payee. Refused while any row still needs review.
    pub fn resolve(
        account_id: AccountId,
        rows: &[ImportRow],
        remember: bool,
        payees: &[Payee],
    ) -> Result<Self, PlanError> {
        let pending = rows.iter().filter(|row| row.needs_review()).count();
        if pending > 0 {
            return Err(PlanError::NeedsReview(pending));
        }
        let mut created: Vec<Payee> = Vec::new();
        let mut transactions = Vec::with_capacity(rows.len());
        for row in rows {
            let (Some(payee), Some(category_id)) = (&row.payee, row.category) else {
                return Err(PlanError::NeedsReview(1));
            };
            let payee_id = match payee {
                RowPayee::Rule(id) | RowPayee::Existing(id) => *id,
                RowPayee::Create(name) => {
                    let folded = name.to_lowercase();
                    let index = match created
                        .iter()
                        .position(|payee| payee.name.to_lowercase() == folded)
                    {
                        Some(index) => index,
                        None => {
                            created.push(Payee {
                                id: PayeeId::new(),
                                name: name.clone(),
                                aliases: Vec::new(),
                                default_category: None,
                                is_active: true,
                            });
                            created.len() - 1
                        }
                    };
                    if let Some(token) = remember.then(|| cleaned_token(&row.raw)).flatten()
                        && lib_payees::alias_owner(payees, &token).is_none()
                        && lib_payees::alias_owner(&created, &token).is_none()
                    {
                        created[index].aliases.push(token);
                    }
                    created[index].id
                }
            };
            transactions.push(Transaction {
                id: TransactionId::new(),
                date: row.date,
                account_id,
                status: TransactionStatus::Cleared,
                is_flagged: false,
                description: Some(row.raw.clone()),
                splits: vec![Split {
                    amount: row.amount.clone(),
                    category_id,
                    payee_id: Some(payee_id),
                    tag_ids: Vec::new(),
                }],
            });
        }
        Ok(Self {
            payees: created,
            transactions,
        })
    }

    /// Checks the plan against `stored`, every Payee already stored: each new Payee's name must be
    /// free and each of its aliases normalised and unowned, among the stored Payees and the plan's
    /// earlier ones. A port runs this before it writes anything, so a refused plan writes nothing.
    pub fn check(&self, stored: &[Payee]) -> Result<ImportCommitted, ImportRefused> {
        let mut known = stored.to_vec();
        for payee in &self.payees {
            if !lib_payees::name_available(&known, &payee.name) {
                return Err(ImportRefused::PayeeNameTaken(payee.name.clone()));
            }
            if let Some(alias) = payee.aliases.iter().find(|alias| {
                lib_payees::normalise_alias(alias).as_deref() != Some(alias.as_str())
                    || lib_payees::alias_owner(&known, alias).is_some()
            }) {
                return Err(ImportRefused::AliasTaken(alias.clone()));
            }
            known.push(payee.clone());
        }
        Ok(ImportCommitted {
            transactions: self.transactions.len(),
            new_payees: self.payees.len(),
        })
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use lib_core::CategoryId;
    use lib_payees::default_payees;

    use super::*;
    use crate::entity::match_line;
    use crate::statement::SeededStatement;
    use crate::{StatementSource, cents};

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 28).expect("a September day")
    }

    /// The seeded statement's rows, every one still needing a Category given Household.
    fn resolved_rows(payees: &[Payee]) -> Vec<ImportRow> {
        SeededStatement::stub(today())
            .statement()
            .lines
            .iter()
            .map(|line| {
                let row = match_line(payees, line);
                ImportRow {
                    category: row.category.or(Some(CategoryId::seed(10))),
                    ..row
                }
            })
            .collect()
    }

    fn resolve(rows: &[ImportRow], remember: bool) -> Result<ImportPlan, PlanError> {
        ImportPlan::resolve(
            SeededStatement::STUB_ACCOUNT,
            rows,
            remember,
            &default_payees(),
        )
    }

    #[test]
    fn a_plan_is_refused_while_rows_need_review() {
        let payees = default_payees();
        let rows: Vec<ImportRow> = SeededStatement::stub(today())
            .statement()
            .lines
            .iter()
            .map(|line| match_line(&payees, line))
            .collect();
        assert_eq!(resolve(&rows, true), Err(PlanError::NeedsReview(4)));
    }

    #[test]
    fn a_plan_creates_new_payees_with_remembered_aliases_and_books_every_row() {
        let payees = default_payees();
        let plan = resolve(&resolved_rows(&payees), true).expect("every row is resolved");

        assert_eq!(plan.transactions.len(), 18);
        let names: Vec<&str> = plan.payees.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(
            names,
            ["Aussie Candle Co", "Blue Bottle Cafe", "Handmade Soaps"]
        );
        let candle = &plan.payees[0];
        assert_eq!(candle.aliases, ["AUSSIE CANDLE CO"]);
        assert!(candle.is_active);
        // The next import matches it by rule.
        assert_eq!(
            lib_payees::match_alias(&plan.payees, "SP AUSSIE CANDLE CO"),
            Some(candle.id)
        );

        let imported = plan
            .transactions
            .iter()
            .find(|t| t.description.as_deref() == Some("SP AUSSIE CANDLE CO"))
            .expect("the row is booked");
        assert_eq!(imported.account_id, SeededStatement::STUB_ACCOUNT);
        assert_eq!(imported.status, TransactionStatus::Cleared);
        assert_eq!(
            imported.splits,
            [Split {
                amount: cents(-3_400),
                category_id: CategoryId::seed(10),
                payee_id: Some(candle.id),
                tag_ids: Vec::new(),
            }]
        );
        assert_eq!(
            plan.check(&payees),
            Ok(ImportCommitted {
                transactions: 18,
                new_payees: 3
            })
        );
    }

    #[test]
    fn a_plan_without_remember_adds_no_aliases() {
        let plan = resolve(&resolved_rows(&default_payees()), false).expect("resolved");
        assert!(plan.payees.iter().all(|payee| payee.aliases.is_empty()));
    }

    #[test]
    fn rows_sharing_a_new_name_create_one_payee() {
        let payees = default_payees();
        let rows: Vec<ImportRow> = ["SP AUSSIE CANDLE CO", "SP AUSSIE CANDLE CO 2"]
            .iter()
            .map(|raw| ImportRow {
                raw: (*raw).to_string(),
                amount: cents(-100),
                date: today(),
                payee: Some(RowPayee::Create("Aussie Candle Co".to_string())),
                category: Some(CategoryId::seed(10)),
            })
            .collect();
        let plan = resolve(&rows, true).expect("resolved");
        assert_eq!(plan.payees.len(), 1);
        assert_eq!(plan.payees[0].aliases, ["AUSSIE CANDLE CO"]);
        assert!(
            plan.transactions
                .iter()
                .all(|t| t.splits[0].payee_id == Some(plan.payees[0].id))
        );
        assert_eq!(plan.check(&payees).map(|c| c.new_payees), Ok(1));
    }

    #[test]
    fn an_alias_another_payee_owns_is_not_remembered() {
        let payees = default_payees();
        let owned = payees
            .iter()
            .find_map(|payee| payee.aliases.first())
            .expect("a stub Payee has an alias")
            .clone();
        assert_eq!(cleaned_token(&owned).as_ref(), Some(&owned));
        let rows = [ImportRow {
            raw: owned.clone(),
            amount: cents(-100),
            date: today(),
            payee: Some(RowPayee::Create("Fresh Name".to_string())),
            category: Some(CategoryId::seed(10)),
        }];
        let plan = resolve(&rows, true).expect("resolved");
        assert!(plan.payees[0].aliases.is_empty());
    }

    #[test]
    fn check_refuses_a_taken_name_or_alias() {
        let payees = default_payees();
        let new_payee = |name: &str, aliases: &[&str]| Payee {
            id: PayeeId::new(),
            name: name.to_string(),
            aliases: aliases.iter().map(ToString::to_string).collect(),
            default_category: None,
            is_active: true,
        };
        let plan = |payees| ImportPlan {
            payees,
            transactions: Vec::new(),
        };

        assert_eq!(
            plan(vec![new_payee("woolworths", &[])]).check(&payees),
            Err(ImportRefused::PayeeNameTaken("woolworths".to_string()))
        );
        assert_eq!(
            plan(vec![new_payee("Twice", &[]), new_payee("TWICE", &[])]).check(&payees),
            Err(ImportRefused::PayeeNameTaken("TWICE".to_string()))
        );
        let owned = payees[0].aliases[0].clone();
        assert_eq!(
            plan(vec![new_payee("Fresh", &[&owned])]).check(&payees),
            Err(ImportRefused::AliasTaken(owned))
        );
        assert_eq!(
            plan(vec![new_payee("Fresh", &["lower case"])]).check(&payees),
            Err(ImportRefused::AliasTaken("lower case".to_string()))
        );
    }
}
