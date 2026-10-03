//! The Pay dialog's live form (`docs/ux/desktop/12-bills/README.md`'s 8d): `gpui`-free and
//! unit-tested, the same split `bill_form.rs` uses.
//!
//! The behaviour is the Desktop Bills Surface map's settlement decision (#368):
//!
//! - Two panels behind one segmented switch: **Pay it directly** (Amount prefilled from the Bill
//!   Plan, Date defaulting to today) and **Match existing transaction** (the entry's Match
//!   candidates, then a "None of these" row that switches to the other panel). Switching keeps
//!   both panels' state.
//! - The dialog opens on Match when there is any candidate, and on Pay it directly otherwise. The
//!   Match list pre-selects only a Payee match (`bills::preselected_candidate`); with nothing
//!   chosen, confirm is refused.
//! - The candidates are worked out once when the dialog opens: nothing else can change the
//!   Transactions while it is modal.

use bigdecimal::{BigDecimal, Signed};
use chrono::NaiveDate;
use lib_core::{DateStyle, Money};
use lib_locale::format::format_date_input;

use crate::{
    bills::{BillError, BillPlan, EntryId, SplitRef},
    transaction_filter_form::parse_date,
};

/// The two settlement paths, in the switch's order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayMode {
    /// Pay it directly: create a Transaction from the Bill Plan and Match it.
    Direct,
    /// Match existing transaction.
    Match,
}

pub const PAY_MODES: [PayMode; 2] = [PayMode::Direct, PayMode::Match];

/// Pay it directly's text fields, in `Tab` order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayField {
    Amount,
    Date,
}

/// A choice in the Match list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchChoice {
    Candidate(usize),
    /// "None of these — pay it directly instead".
    NoneOfThese,
}

/// What confirming the dialog does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PayAction {
    Pay { amount: Money, date: NaiveDate },
    Match(SplitRef),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayForm {
    pub entry: EntryId,
    pub mode: PayMode,
    pub candidates: Vec<SplitRef>,
    pub choice: Option<MatchChoice>,
    pub focused: PayField,
    pub amount: String,
    pub date: String,
    /// A refused confirm, shown until the next change.
    pub error: Option<BillError>,
}

impl PayForm {
    pub fn new(
        entry: EntryId,
        plan: &BillPlan,
        candidates: Vec<SplitRef>,
        preselected: Option<SplitRef>,
        today: NaiveDate,
        date_style: Option<DateStyle>,
    ) -> Self {
        let choice = preselected
            .and_then(|split| candidates.iter().position(|c| *c == split))
            .map(MatchChoice::Candidate);
        Self {
            entry,
            mode: if candidates.is_empty() {
                PayMode::Direct
            } else {
                PayMode::Match
            },
            candidates,
            choice,
            focused: PayField::Amount,
            amount: plan.planned_amount.0.to_string(),
            date: format_date_input(today, date_style),
            error: None,
        }
    }

    pub fn set_mode(&mut self, mode: PayMode) {
        self.mode = mode;
        self.error = None;
    }

    /// `left`/`right`: the other panel.
    pub fn toggle_mode(&mut self) {
        self.set_mode(match self.mode {
            PayMode::Direct => PayMode::Match,
            PayMode::Match => PayMode::Direct,
        });
    }

    /// `j`/`k` over the Match list, the "None of these" row last. Stops at either end; with
    /// nothing chosen yet, either key lands on the first row.
    pub fn step_choice(&mut self, forward: bool) {
        let last = self.candidates.len();
        let at = match self.choice {
            None => {
                self.choice = Some(Self::choice_at(0, last));
                return;
            }
            Some(MatchChoice::Candidate(index)) => index,
            Some(MatchChoice::NoneOfThese) => last,
        };
        let next = if forward {
            (at + 1).min(last)
        } else {
            at.saturating_sub(1)
        };
        self.choice = Some(Self::choice_at(next, last));
        self.error = None;
    }

    fn choice_at(index: usize, last: usize) -> MatchChoice {
        if index >= last {
            MatchChoice::NoneOfThese
        } else {
            MatchChoice::Candidate(index)
        }
    }

    /// A click on a Match row; "None of these" switches to Pay it directly at once.
    pub fn choose(&mut self, choice: MatchChoice) {
        self.error = None;
        if choice == MatchChoice::NoneOfThese {
            self.choice = Some(choice);
            self.mode = PayMode::Direct;
        } else {
            self.choice = Some(choice);
        }
    }

    pub fn cycle_focus(&mut self) {
        self.focused = match self.focused {
            PayField::Amount => PayField::Date,
            PayField::Date => PayField::Amount,
        };
    }

    pub fn focus(&mut self, field: PayField) {
        self.focused = field;
    }

    /// Types into Pay it directly's focused field: the amount takes a decimal only.
    pub fn push_char(&mut self, ch: char) {
        if ch.is_control() || self.mode != PayMode::Direct {
            return;
        }
        match self.focused {
            PayField::Amount
                if ch.is_ascii_digit() || (ch == '.' && !self.amount.contains('.')) =>
            {
                self.amount.push(ch);
            }
            PayField::Amount => return,
            PayField::Date => self.date.push(ch),
        }
        self.error = None;
    }

    pub fn backspace(&mut self) {
        if self.mode != PayMode::Direct {
            return;
        }
        match self.focused {
            PayField::Amount => self.amount.pop(),
            PayField::Date => self.date.pop(),
        };
        self.error = None;
    }

    fn amount_money(&self) -> Option<Money> {
        self.amount
            .trim()
            .parse::<BigDecimal>()
            .ok()
            .filter(|amount| amount.is_positive())
            .map(Money)
    }

    /// The amount's problem, once something is typed.
    pub fn amount_invalid(&self) -> bool {
        !self.amount.trim().is_empty() && self.amount_money().is_none()
    }

    /// The date's problem, as the Transactions filter words it.
    pub fn date_error(&self, today: NaiveDate, date_style: Option<DateStyle>) -> Option<String> {
        parse_date(&self.date, today, date_style).err()
    }

    /// What confirm does from the active panel, or `None` while it can't: an empty or invalid
    /// field, or no candidate chosen.
    pub fn action(&self, today: NaiveDate, date_style: Option<DateStyle>) -> Option<PayAction> {
        match self.mode {
            PayMode::Direct => Some(PayAction::Pay {
                amount: self.amount_money()?,
                date: parse_date(&self.date, today, date_style).ok()??,
            }),
            PayMode::Match => match self.choice? {
                MatchChoice::Candidate(index) => {
                    self.candidates.get(index).copied().map(PayAction::Match)
                }
                MatchChoice::NoneOfThese => None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        accounts::default_accounts, bills::default_bills, categories::default_categories,
        payees::default_payees, tags::default_tags, transactions::default_transactions,
    };

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 19).unwrap()
    }

    /// The seeded Telstra Internet Plan.
    fn plan() -> BillPlan {
        let (accounts, categories, payees) =
            (default_accounts(), default_categories(), default_payees());
        let mut transactions =
            default_transactions(&accounts, &categories, &payees, &default_tags(), today());
        default_bills(&accounts, &categories, &payees, &mut transactions, today())
            .plans
            .into_iter()
            .find(|plan| plan.name == "Telstra Internet")
            .unwrap()
    }

    fn split(transaction_id: u32) -> SplitRef {
        SplitRef {
            transaction_id,
            split_index: 0,
        }
    }

    fn form(candidates: Vec<SplitRef>, preselected: Option<SplitRef>) -> PayForm {
        let entry = EntryId {
            plan_id: 1,
            due: today(),
        };
        PayForm::new(entry, &plan(), candidates, preselected, today(), None)
    }

    #[test]
    fn opens_on_match_with_the_preselected_candidate() {
        let f = form(vec![split(4), split(9)], Some(split(9)));
        assert_eq!(f.mode, PayMode::Match);
        assert_eq!(f.choice, Some(MatchChoice::Candidate(1)));
        assert_eq!(f.action(today(), None), Some(PayAction::Match(split(9))));
    }

    #[test]
    fn opens_on_direct_with_no_candidates_and_prefills_the_plan() {
        let f = form(Vec::new(), None);
        assert_eq!(f.mode, PayMode::Direct);
        assert_eq!(
            f.action(today(), None),
            Some(PayAction::Pay {
                amount: plan().planned_amount,
                date: today(),
            })
        );
    }

    #[test]
    fn nothing_chosen_refuses_confirm_until_a_step() {
        let mut f = form(vec![split(4)], None);
        assert_eq!(f.action(today(), None), None);
        f.step_choice(true);
        assert_eq!(f.choice, Some(MatchChoice::Candidate(0)));
        f.step_choice(true);
        f.step_choice(true);
        assert_eq!(f.choice, Some(MatchChoice::NoneOfThese));
        assert_eq!(f.action(today(), None), None);
        f.step_choice(false);
        assert_eq!(f.choice, Some(MatchChoice::Candidate(0)));
    }

    #[test]
    fn choosing_none_of_these_switches_to_direct() {
        let mut f = form(vec![split(4)], Some(split(4)));
        f.choose(MatchChoice::NoneOfThese);
        assert_eq!(f.mode, PayMode::Direct);
        f.toggle_mode();
        assert_eq!(f.mode, PayMode::Match);
    }

    #[test]
    fn direct_refuses_a_zero_amount_or_a_bad_date() {
        let mut f = form(Vec::new(), None);
        f.amount = "0".to_string();
        assert!(f.amount_invalid());
        assert_eq!(f.action(today(), None), None);
        f.amount.clear();
        f.push_char('1');
        f.push_char('.');
        f.push_char('.');
        f.push_char('x');
        assert_eq!(f.amount, "1.");
        f.focus(PayField::Date);
        f.date = "not a date".to_string();
        assert!(f.date_error(today(), None).is_some());
        assert_eq!(f.action(today(), None), None);
    }

    #[test]
    fn typing_is_ignored_on_the_match_panel() {
        let mut f = form(vec![split(4)], None);
        let amount = f.amount.clone();
        f.push_char('5');
        f.backspace();
        assert_eq!(f.amount, amount);
    }
}
