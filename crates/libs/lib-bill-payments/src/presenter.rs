//! The Pay Dialog (8d), the Desktop Bills Surface map's settlement decision (#368):
//!
//! - Two panels behind one segmented switch: **Pay it directly** (Amount prefilled from the Bill
//!   Plan, Date defaulting to today) and **Match existing transaction** (the entry's Match
//!   candidates, then a "None of these" row that switches to the other panel). Switching keeps
//!   both panels' state.
//! - The Dialog opens on Match when there is any candidate, and on Pay it directly otherwise. The
//!   Match list pre-selects only a Payee match (`lib_bills::preselected_candidate`); with nothing
//!   chosen, confirm is refused.
//! - Pay it directly runs this module's [`PayBill`]; Match runs `lib-bills`' `MatchBill`. A refusal
//!   shows under the panel until the next event.
//!
//! The candidates and every name the Dialog shows are read when it opens: it is modal, so nothing
//! changes them under it.

use bigdecimal::{BigDecimal, Signed};
use chrono::{Duration, NaiveDate};
use lib_bills::interactor::MatchBill;
use lib_bills::presenter::money;
use lib_bills::{BillError, EntryId, MATCH_WINDOW_DAYS, PayRequest, SharedPorts, SplitRef};
use lib_core::{DateStyle, Money};
use lib_locale::format::{
    AmountStyle, DateInputError, DateInputOptions, date_error_message, format_amount,
    format_date_input, format_month_day, parse_date_with,
};
use lib_viper::view::{Choice, DialogFrame, TextFieldView};

use crate::interactor::PayBill;
use crate::msg;
use crate::protocols::{BillPaymentsRouter, PayOutcome, SharedPayments};

/// The two settlement paths, in the switch's order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayMode {
    /// Pay it directly: create a Transaction from the Bill Plan and Match it.
    Direct,
    /// Match existing transaction.
    Match,
}

/// Pay it directly's text fields, in `Tab` order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayField {
    Amount,
    Date,
}

/// A row of the Match list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchChoice {
    Candidate(usize),
    /// "None of these — pay it directly instead".
    NoneOfThese,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PayEvent {
    /// A click on a switch segment.
    Mode(PayMode),
    /// `←`/`→`: the other panel.
    ToggleMode,
    /// `j`/`k` over the Match list, "None of these" last. Stops at either end; with nothing chosen
    /// yet, either lands on the first row.
    NextChoice,
    PrevChoice,
    /// A click on a Match row; "None of these" switches to Pay it directly at once.
    Choose(MatchChoice),
    /// A text field's whole new text, as its View's input holds it. The amount keeps only a
    /// decimal's characters.
    Amount(String),
    Date(String),
    /// `Tab`: the other text field.
    NextField,
    /// A click on a text field.
    Focus(PayField),
    /// Pays or Matches from the active panel. On "None of these" it switches to Pay it directly
    /// instead, as a click on that row does.
    Confirm,
    Cancel,
}

/// One Match candidate, worded: "Telstra · 21 Sep · −79.99", and its Account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateLine {
    pub summary: String,
    pub account: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayViewModel {
    pub frame: DialogFrame,
    /// The switch's segments, in order.
    pub modes: Vec<Choice<PayMode>>,
    pub mode: PayMode,
    /// The Match list's heading.
    pub candidates_heading: String,
    pub candidates: Vec<CandidateLine>,
    /// Shown in place of the candidates when there are none.
    pub no_candidates: Option<String>,
    pub none_of_these: String,
    /// The Match list's chosen row.
    pub choice: Option<MatchChoice>,
    pub match_notice: String,
    pub amount: TextFieldView,
    pub amount_error: Option<String>,
    pub date: TextFieldView,
    pub date_error: Option<String>,
    /// The Plan's Category, Payee and Account, read-only on Pay it directly.
    pub chips: Vec<String>,
    pub direct_notice: String,
    /// Why the last confirm was refused.
    pub refusal: Option<String>,
}

/// What the Dialog shows that it reads once, when it opens.
struct Opened {
    title: String,
    candidates: Vec<CandidateLine>,
    match_notice: String,
    chips: Vec<String>,
}

pub struct PayPresenter {
    bills: SharedPorts,
    payments: SharedPayments,
    entry: EntryId,
    candidates: Vec<SplitRef>,
    opened: Opened,
    mode: PayMode,
    choice: Option<MatchChoice>,
    focused: PayField,
    amount: String,
    date: String,
    refusal: Option<String>,
    today: NaiveDate,
    date_style: Option<DateStyle>,
}

impl PayPresenter {
    /// The Dialog on `request`'s entry, or `None` when its Plan is gone.
    pub fn new(
        bills: SharedPorts,
        payments: SharedPayments,
        request: PayRequest,
        today: NaiveDate,
        date_style: Option<DateStyle>,
    ) -> Option<Self> {
        let PayRequest {
            entry,
            candidates,
            preselected,
        } = request;
        let (opened, planned_amount) = {
            let ports = bills.borrow();
            let plan = ports.bills().bill_plan(entry.plan_id)?;
            let accounts = ports.accounts().bill_accounts();
            let payees = ports.payees().bill_payees();
            let account_name = |id| {
                accounts
                    .iter()
                    .find(|account| account.id == id)
                    .map(|account| account.name.clone())
                    .unwrap_or_default()
            };
            let payee_name = |id| {
                payees
                    .iter()
                    .find(|payee| Some(payee.id) == id)
                    .map(|payee| payee.name.clone())
            };
            let reach = Duration::days(MATCH_WINDOW_DAYS);
            let nearby = ports
                .transactions()
                .splits_between(entry.due - reach, entry.due + reach);
            let lines = candidates
                .iter()
                .map(|split_ref| {
                    let Some(split) = lib_bills::split_of(&nearby, *split_ref) else {
                        return CandidateLine {
                            summary: String::new(),
                            account: String::new(),
                        };
                    };
                    let payee = payee_name(split.payee_id)
                        .or_else(|| split.description.clone())
                        .unwrap_or_default();
                    CandidateLine {
                        summary: msg::bill_payments_pay_candidate(
                            &payee,
                            &format_month_day(split.date),
                            &format_amount(&split.amount, AmountStyle::own().with_plus()).text,
                        ),
                        account: account_name(split.account_id),
                    }
                })
                .collect();
            let category =
                lib_bills::category_path(&ports.categories().bill_categories(), plan.category_id)
                    .unwrap_or_default();
            let opened = Opened {
                title: msg::bill_payments_pay_title(&plan.name, &format_month_day(entry.due)),
                candidates: lines,
                match_notice: msg::bill_payments_pay_match_notice(&money(
                    &*ports,
                    &plan.planned_amount,
                    plan.unit,
                )),
                chips: vec![
                    category,
                    payee_name(plan.payee_id).unwrap_or_else(msg::bill_payments_pay_no_payee),
                    account_name(plan.account_id),
                ],
            };
            (opened, plan.planned_amount)
        };
        let choice = preselected
            .and_then(|split| candidates.iter().position(|c| *c == split))
            .map(MatchChoice::Candidate);
        Some(Self {
            bills,
            payments,
            entry,
            mode: if candidates.is_empty() {
                PayMode::Direct
            } else {
                PayMode::Match
            },
            candidates,
            opened,
            choice,
            focused: PayField::Amount,
            amount: planned_amount.0.to_string(),
            date: format_date_input(today, date_style),
            refusal: None,
            today,
            date_style,
        })
    }

    /// Any event clears a refused confirm; Confirm sets it again if it is refused once more.
    pub fn handle(&mut self, event: PayEvent, router: &mut dyn BillPaymentsRouter) {
        self.refusal = None;
        match event {
            PayEvent::Mode(mode) => self.mode = mode,
            PayEvent::ToggleMode => {
                self.mode = match self.mode {
                    PayMode::Direct => PayMode::Match,
                    PayMode::Match => PayMode::Direct,
                };
            }
            PayEvent::NextChoice => self.step_choice(true),
            PayEvent::PrevChoice => self.step_choice(false),
            PayEvent::Choose(choice) => self.choose(choice),
            PayEvent::Amount(text) => self.amount = decimal_characters(&text),
            PayEvent::Date(text) => self.date = text,
            PayEvent::NextField => {
                self.focused = match self.focused {
                    PayField::Amount => PayField::Date,
                    PayField::Date => PayField::Amount,
                };
            }
            PayEvent::Focus(field) => self.focused = field,
            PayEvent::Confirm => self.confirm(router),
            PayEvent::Cancel => router.close_dialog(None),
        }
    }

    fn step_choice(&mut self, forward: bool) {
        let last = self.candidates.len();
        let next = match self.choice {
            None => 0,
            Some(MatchChoice::Candidate(index)) if forward => (index + 1).min(last),
            Some(MatchChoice::Candidate(index)) => index.saturating_sub(1),
            Some(MatchChoice::NoneOfThese) if forward => last,
            Some(MatchChoice::NoneOfThese) => last.saturating_sub(1),
        };
        self.choice = Some(if next >= last {
            MatchChoice::NoneOfThese
        } else {
            MatchChoice::Candidate(next)
        });
    }

    fn choose(&mut self, choice: MatchChoice) {
        self.choice = Some(choice);
        if choice == MatchChoice::NoneOfThese {
            self.mode = PayMode::Direct;
        }
    }

    fn confirm(&mut self, router: &mut dyn BillPaymentsRouter) {
        let landed = match self.mode {
            PayMode::Direct => {
                let (Some(amount), Ok(Some(date))) = (self.amount_money(), self.parse_date())
                else {
                    return;
                };
                PayBill {
                    entry: self.entry,
                    amount,
                    date,
                }
                .run(&mut *self.payments.borrow_mut())
                .map(PayOutcome::Paid)
            }
            PayMode::Match => match self.choice {
                Some(MatchChoice::NoneOfThese) => {
                    self.mode = PayMode::Direct;
                    return;
                }
                Some(MatchChoice::Candidate(index)) => {
                    let Some(split) = self.candidates.get(index).copied() else {
                        return;
                    };
                    let mut ports = self.bills.borrow_mut();
                    let ports = ports.split_mut();
                    MatchBill {
                        entry: self.entry,
                        split,
                    }
                    .run(
                        ports.bills,
                        ports.transactions,
                        ports.accounts,
                        ports.categories,
                    )
                    .map(|()| PayOutcome::Matched)
                }
                None => return,
            },
        };
        match landed {
            Ok(outcome) => router.close_dialog(Some(outcome)),
            Err(refusal) => self.refusal = Some(refusal_text(&refusal)),
        }
    }

    fn amount_money(&self) -> Option<Money> {
        self.amount
            .trim()
            .parse::<BigDecimal>()
            .ok()
            .filter(Signed::is_positive)
            .map(Money)
    }

    fn parse_date(&self) -> Result<Option<NaiveDate>, String> {
        let options = DateInputOptions {
            style: self.date_style,
            allow_yearless: true,
        };
        match parse_date_with(&self.date, self.today, &options) {
            Ok(date) => Ok(Some(date)),
            Err(DateInputError::Empty) => Ok(None),
            Err(error) => Err(date_error_message(&error, self.today, &options)),
        }
    }

    /// Whether confirm would pay or match from the active panel: Pay it directly needs a positive
    /// amount and a date, Match a chosen candidate.
    fn can_confirm(&self) -> bool {
        match self.mode {
            PayMode::Direct => {
                self.amount_money().is_some() && matches!(self.parse_date(), Ok(Some(_)))
            }
            PayMode::Match => matches!(self.choice, Some(MatchChoice::Candidate(_))),
        }
    }

    pub fn view_model(&self) -> PayViewModel {
        let direct = self.mode == PayMode::Direct;
        let text = |label: String, text: &str, placeholder: String, field| TextFieldView {
            label,
            text: text.to_string(),
            placeholder,
            focused: direct && self.focused == field,
        };
        PayViewModel {
            frame: DialogFrame {
                title: self.opened.title.clone(),
                submit: if direct {
                    msg::bill_payments_pay_submit_direct()
                } else {
                    msg::bill_payments_pay_submit_match()
                },
                can_confirm: self.can_confirm(),
                destructive: false,
            },
            modes: vec![
                Choice {
                    value: PayMode::Direct,
                    label: msg::bill_payments_pay_mode_direct(),
                },
                Choice {
                    value: PayMode::Match,
                    label: msg::bill_payments_pay_mode_match(),
                },
            ],
            mode: self.mode,
            candidates_heading: msg::bill_payments_pay_candidates(),
            candidates: self.opened.candidates.clone(),
            no_candidates: self
                .candidates
                .is_empty()
                .then(msg::bill_payments_pay_no_candidates),
            none_of_these: msg::bill_payments_pay_none_of_these(),
            choice: self.choice,
            match_notice: self.opened.match_notice.clone(),
            amount: text(
                msg::bill_payments_pay_field_amount(),
                &self.amount,
                msg::bill_payments_pay_amount_placeholder(),
                PayField::Amount,
            ),
            amount_error: (!self.amount.trim().is_empty() && self.amount_money().is_none())
                .then(msg::bill_payments_pay_error_amount),
            date: text(
                msg::bill_payments_pay_field_date(),
                &self.date,
                lib_locale::msg::date_word_today(),
                PayField::Date,
            ),
            date_error: self.parse_date().err(),
            chips: self.opened.chips.clone(),
            direct_notice: msg::bill_payments_pay_direct_notice(),
            refusal: self.refusal.clone(),
        }
    }
}

/// `text` with only a decimal's characters: digits and the first point.
fn decimal_characters(text: &str) -> String {
    let mut kept = String::new();
    for ch in text.chars() {
        if ch.is_ascii_digit() || (ch == '.' && !kept.contains('.')) {
            kept.push(ch);
        }
    }
    kept
}

/// A refused confirm. The panel's own checks catch a bad amount first, so what's left is an entry
/// settled, superseded or gone under the Dialog, or a candidate no longer one.
fn refusal_text(refusal: &BillError) -> String {
    match refusal {
        BillError::AmountNotPositive => msg::bill_payments_pay_error_amount(),
        _ => msg::bill_payments_pay_error_gone(),
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use lib_bills::fake::InMemoryPorts;
    use lib_bills::interactor::{MatchCandidates, SkipBill};
    use lib_bills::{BillsPorts as _, Resolution, entry};

    use super::*;
    use crate::fake::{PortsPayments, RecordingRouter};
    use crate::interactor::test_support::{date, entry_of, stub, today};

    /// The Dialog on the stub's Telstra Internet entry due on the 22nd, which has Match
    /// candidates and a Payee match to pre-select.
    struct Dialog {
        ports: Rc<RefCell<InMemoryPorts>>,
        entry: EntryId,
        presenter: PayPresenter,
        router: RecordingRouter,
    }

    impl Dialog {
        fn open() -> Self {
            Self::open_with(|request| request)
        }

        /// The Dialog on a request `change` has altered, e.g. to offer no candidates.
        fn open_with(change: impl FnOnce(PayRequest) -> PayRequest) -> Self {
            lib_locale::init_with_layers("en-US", &[msg::LAYER]);
            let ports = stub();
            let entry = entry_of(&ports, "Telstra Internet", date(2026, 9, 22));
            let (candidates, preselected) = {
                let ports = ports.borrow();
                MatchCandidates { entry }.run(
                    ports.bills(),
                    ports.transactions(),
                    ports.accounts(),
                    ports.categories(),
                )
            };
            let shared: SharedPorts = ports.clone();
            let request = change(PayRequest {
                entry,
                candidates,
                preselected,
            });
            let presenter = PayPresenter::new(
                shared,
                PortsPayments::shared(&ports),
                request,
                today(),
                None,
            )
            .expect("the Plan is there");
            Self {
                ports,
                entry,
                presenter,
                router: RecordingRouter::default(),
            }
        }

        fn send(&mut self, event: PayEvent) {
            self.presenter.handle(event, &mut self.router);
        }

        fn view(&self) -> PayViewModel {
            self.presenter.view_model()
        }

        fn resolution(&self) -> Option<Resolution> {
            entry(&self.ports.borrow().bills.entries, self.entry).map(|e| e.resolution.clone())
        }
    }

    fn no_candidates(request: PayRequest) -> PayRequest {
        PayRequest {
            candidates: Vec::new(),
            preselected: None,
            ..request
        }
    }

    fn nothing_preselected(request: PayRequest) -> PayRequest {
        PayRequest {
            preselected: None,
            ..request
        }
    }

    #[test]
    fn opens_on_match_with_the_payee_match_preselected_and_worded() {
        let dialog = Dialog::open();
        let view = dialog.view();

        assert_eq!(view.mode, PayMode::Match);
        assert_eq!(view.choice, Some(MatchChoice::Candidate(0)));
        assert!(view.frame.can_confirm);
        assert_eq!(view.frame.title, "Pay \u{2014} Telstra Internet, Sep 22");
        assert_eq!(view.frame.submit, "Match & mark paid");
        assert_eq!(
            view.modes
                .iter()
                .map(|mode| mode.label.as_str())
                .collect::<Vec<_>>(),
            ["Pay it directly", "Match existing transaction"]
        );
        assert_eq!(view.candidates_heading, "Unmatched transactions");
        assert!(
            view.candidates[0].summary.starts_with("Telstra \u{b7} "),
            "{}",
            view.candidates[0].summary
        );
        assert!(!view.candidates[0].account.is_empty());
        assert_eq!(view.no_candidates, None);
        assert_eq!(
            view.none_of_these,
            "None of these \u{2014} pay it directly instead"
        );
        assert!(
            view.match_notice.contains("the plan's 89.00 estimate"),
            "{}",
            view.match_notice
        );
    }

    #[test]
    fn opens_on_pay_it_directly_with_no_candidates_and_prefills_the_plan() {
        let dialog = Dialog::open_with(no_candidates);
        let view = dialog.view();

        assert_eq!(view.mode, PayMode::Direct);
        assert_eq!(view.frame.submit, "Create transaction & mark paid");
        assert_eq!(
            view.no_candidates.as_deref(),
            Some("No unmatched expense within 14 days of the due date.")
        );
        assert_eq!(view.amount.text, "89.00");
        assert!(view.amount.focused);
        assert_eq!(view.amount.label, "Amount");
        assert_eq!(view.date.text, format_date_input(today(), None));
        assert_eq!(view.date.label, "Date");
        assert_eq!(view.chips.len(), 3);
        assert!(view.chips[1].contains("Telstra"), "{:?}", view.chips);
        assert!(view.frame.can_confirm);
    }

    #[test]
    fn nothing_chosen_refuses_confirm_until_a_step_and_the_steps_stop_at_either_end() {
        let mut dialog = Dialog::open_with(nothing_preselected);
        let rows = dialog.view().candidates.len();
        assert_eq!(dialog.view().choice, None);
        assert!(!dialog.view().frame.can_confirm);

        dialog.send(PayEvent::Confirm);
        assert!(dialog.router.closed.is_empty());

        dialog.send(PayEvent::PrevChoice);
        assert_eq!(dialog.view().choice, Some(MatchChoice::Candidate(0)));
        dialog.send(PayEvent::PrevChoice);
        assert_eq!(dialog.view().choice, Some(MatchChoice::Candidate(0)));
        for _ in 0..=rows {
            dialog.send(PayEvent::NextChoice);
        }
        assert_eq!(dialog.view().choice, Some(MatchChoice::NoneOfThese));
        assert!(!dialog.view().frame.can_confirm);
        dialog.send(PayEvent::PrevChoice);
        assert_eq!(dialog.view().choice, Some(MatchChoice::Candidate(rows - 1)));
    }

    #[test]
    fn none_of_these_switches_to_pay_it_directly_by_click_or_confirm() {
        let mut dialog = Dialog::open();
        dialog.send(PayEvent::Choose(MatchChoice::NoneOfThese));
        assert_eq!(dialog.view().mode, PayMode::Direct);

        dialog.send(PayEvent::ToggleMode);
        assert_eq!(dialog.view().mode, PayMode::Match);
        assert_eq!(dialog.view().choice, Some(MatchChoice::NoneOfThese));
        dialog.send(PayEvent::Confirm);
        assert_eq!(dialog.view().mode, PayMode::Direct);
        assert!(dialog.router.closed.is_empty(), "switching closes nothing");
    }

    #[test]
    fn switching_keeps_both_panels_state() {
        let mut dialog = Dialog::open();
        dialog.send(PayEvent::NextChoice);
        let chosen = dialog.view().choice;
        dialog.send(PayEvent::Mode(PayMode::Direct));
        dialog.send(PayEvent::Amount("12.50".into()));
        dialog.send(PayEvent::NextField);

        dialog.send(PayEvent::Mode(PayMode::Match));
        let view = dialog.view();
        assert_eq!(view.choice, chosen);
        assert!(
            !view.amount.focused && !view.date.focused,
            "Match has no text field"
        );
        dialog.send(PayEvent::ToggleMode);
        let view = dialog.view();
        assert_eq!(view.amount.text, "12.50");
        assert!(view.date.focused);
    }

    #[test]
    fn the_amount_keeps_a_decimal_and_refuses_zero_and_the_date_must_parse() {
        let mut dialog = Dialog::open_with(no_candidates);

        dialog.send(PayEvent::Amount("1x2.5.".into()));
        assert_eq!(dialog.view().amount.text, "12.5");
        dialog.send(PayEvent::Amount("0".into()));
        let view = dialog.view();
        assert_eq!(
            view.amount_error.as_deref(),
            Some("Enter an amount greater than zero.")
        );
        assert!(!view.frame.can_confirm);
        dialog.send(PayEvent::Amount(String::new()));
        assert_eq!(
            dialog.view().amount_error,
            None,
            "a blank amount shows no error"
        );
        assert!(!dialog.view().frame.can_confirm);

        dialog.send(PayEvent::Amount("10".into()));
        dialog.send(PayEvent::Focus(PayField::Date));
        dialog.send(PayEvent::Date("not a date".into()));
        let view = dialog.view();
        assert!(view.date_error.is_some());
        assert!(!view.frame.can_confirm);
        dialog.send(PayEvent::Confirm);
        assert!(dialog.router.closed.is_empty());
    }

    #[test]
    fn confirming_pay_it_directly_pays_the_entry_and_closes() {
        let mut dialog = Dialog::open_with(no_candidates);
        dialog.send(PayEvent::Amount("42.50".into()));

        dialog.send(PayEvent::Confirm);

        let Some(Some(PayOutcome::Paid(id))) = dialog.router.closed.first().copied() else {
            panic!("closed Paid: {:?}", dialog.router.closed);
        };
        assert_eq!(
            dialog.resolution(),
            Some(Resolution::Paid(SplitRef {
                transaction_id: id,
                split_index: 0,
            }))
        );
        let ports = dialog.ports.borrow();
        let paid: Vec<_> = ports
            .reads
            .splits
            .iter()
            .filter(|s| s.split.transaction_id == id)
            .collect();
        let [paid] = paid.as_slice() else {
            panic!("the Transaction is booked with one Split: {paid:?}");
        };
        assert_eq!(paid.amount, Money("-42.50".parse().expect("a decimal")));
    }

    #[test]
    fn confirming_match_matches_the_chosen_candidate_and_closes() {
        let mut dialog = Dialog::open();
        let splits = dialog.ports.borrow().reads.splits.len();

        dialog.send(PayEvent::Confirm);

        assert_eq!(dialog.router.closed, [Some(PayOutcome::Matched)]);
        assert!(matches!(dialog.resolution(), Some(Resolution::Paid(_))));
        assert_eq!(
            dialog.ports.borrow().reads.splits.len(),
            splits,
            "a Match books no Transaction"
        );
    }

    #[test]
    fn an_entry_settled_underneath_is_refused_on_either_panel_until_the_next_event() {
        let mut dialog = Dialog::open();
        SkipBill {
            entry: dialog.entry,
        }
        .run(&mut dialog.ports.borrow_mut().bills)
        .expect("an open entry");

        dialog.send(PayEvent::Confirm);
        assert_eq!(
            dialog.view().refusal.as_deref(),
            Some("This bill can no longer be paid.")
        );
        assert!(dialog.router.closed.is_empty());

        dialog.send(PayEvent::ToggleMode);
        assert_eq!(dialog.view().refusal, None);
        dialog.send(PayEvent::Confirm);
        assert_eq!(
            dialog.view().refusal.as_deref(),
            Some("This bill can no longer be paid.")
        );
        assert!(dialog.router.closed.is_empty());
    }

    #[test]
    fn cancel_closes_without_paying() {
        let mut dialog = Dialog::open();
        let before = dialog.resolution();

        dialog.send(PayEvent::Cancel);

        assert_eq!(dialog.router.closed, [None]);
        assert_eq!(dialog.resolution(), before);
    }

    #[test]
    fn a_plan_gone_opens_no_dialog() {
        lib_locale::init_with_layers("en-US", &[msg::LAYER]);
        let ports = stub();
        let shared: SharedPorts = ports.clone();
        let request = PayRequest {
            entry: EntryId {
                plan_id: lib_core::BillPlanId::seed(9_999),
                due: today(),
            },
            candidates: Vec::new(),
            preselected: None,
        };

        assert!(
            PayPresenter::new(
                shared,
                PortsPayments::shared(&ports),
                request,
                today(),
                None
            )
            .is_none()
        );
    }
}
