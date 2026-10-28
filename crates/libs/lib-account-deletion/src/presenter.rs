//! The Delete account Dialog (17-accounts' 3d):
//!
//! - It names what goes: the Account's balance in its own Unit, how many Transactions are deleted
//!   with it, and how many Budgets referenced it, which stop counting it (`AccountUsage`).
//! - The Account's name must be typed back exactly, case and all, before **Delete account**
//!   enables, on the destructive frame.
//! - While a Bill Plan pays from the Account it says how many, and confirm stays off: the Plans
//!   move to another Account first. A Plan that came to pay from it under the open Dialog turns
//!   it into that, as the Interactor's refusal says.
//!
//! Every figure is read when it opens: it is modal, so nothing changes them under it. The Dialog
//! raises its own Toast on landing, so the Client only learns what it removed.

use lib_accounts::AccountUse;
use lib_core::AccountId;
use lib_locale::Segment;
use lib_locale::format::{AmountStyle, format_amount};
use lib_toast::{Notice, ToastKind};
use lib_viper::view::{DialogFrame, TextFieldView};

use crate::error::Error;
use crate::interactor::DeleteAccount;
use crate::msg;
use crate::protocols::{AccountDeletionRouter, DeletionReads, SharedDeletion};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeleteAccountEvent {
    /// The confirmation field's whole new text.
    ConfirmText(String),
    /// Deletes once the name is typed back and nothing pays from the Account.
    Confirm,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteAccountViewModel {
    pub frame: DialogFrame,
    /// The balance and the Transaction count, each tagged `strong`.
    pub warning: Vec<Segment>,
    /// How many Transactions go, and how many Budgets referenced the Account.
    pub reference: String,
    /// Why it can't be deleted yet: the Bill Plans that pay from it.
    pub blocked: Option<String>,
    /// `Type Wallet to confirm`.
    pub confirm: TextFieldView,
    /// The counts the Dialog quotes.
    pub usage: AccountUse,
    /// Why the last confirm was refused.
    pub refusal: Option<String>,
}

pub struct DeleteAccountPresenter {
    deletion: SharedDeletion,
    id: AccountId,
    name: String,
    /// The balance in the Account's own Unit, formatted when it opens.
    balance: String,
    usage: AccountUse,
    /// The Bill Plans paying from it.
    plans: usize,
    confirm_text: String,
    refusal: Option<String>,
}

impl DeleteAccountPresenter {
    /// The Dialog for Account `id`, reading its counts from `reads`, or `None` when it is gone.
    pub fn new(deletion: SharedDeletion, reads: &dyn DeletionReads, id: AccountId) -> Option<Self> {
        let (account, plans) = {
            let port = deletion.borrow();
            (port.account(id)?, port.bill_plans_paying_from(id))
        };
        let amount = format_amount(&account.balance, AmountStyle::own()).text;
        let balance = match reads.unit_code(account.unit) {
            Some(code) => format!("{amount} {code}"),
            None => amount,
        };
        Some(Self {
            deletion,
            id,
            name: account.name,
            balance,
            usage: reads.account_usage(id),
            plans,
            confirm_text: String::new(),
            refusal: None,
        })
    }

    pub fn handle(&mut self, event: DeleteAccountEvent, router: &mut dyn AccountDeletionRouter) {
        match event {
            DeleteAccountEvent::ConfirmText(text) => self.confirm_text = text,
            DeleteAccountEvent::Confirm => self.confirm(router),
            DeleteAccountEvent::Cancel => router.close_dialog(None),
        }
    }

    pub fn view_model(&self) -> DeleteAccountViewModel {
        let name = self.name.as_str();
        DeleteAccountViewModel {
            frame: DialogFrame {
                title: msg::account_deletion_title(name),
                submit: msg::account_deletion_submit(),
                can_confirm: self.can_confirm(),
                destructive: true,
            },
            warning: msg::account_deletion_warning(&self.balance, count(self.usage.transactions)),
            reference: msg::account_deletion_reference(
                count(self.usage.transactions),
                count(self.usage.budgets),
            ),
            blocked: (self.plans > 0).then(|| msg::account_deletion_bill_plans(count(self.plans))),
            confirm: TextFieldView {
                label: msg::account_deletion_confirm_label(name),
                text: self.confirm_text.clone(),
                placeholder: self.name.clone(),
                focused: true,
            },
            usage: self.usage,
            refusal: self.refusal.clone(),
        }
    }

    /// The typed name must match exactly, case and all, and no Bill Plan may pay from it.
    fn can_confirm(&self) -> bool {
        self.plans == 0 && self.confirm_text == self.name
    }

    fn confirm(&mut self, router: &mut dyn AccountDeletionRouter) {
        if !self.can_confirm() {
            return;
        }
        let landed = DeleteAccount { id: self.id }.run(&mut *self.deletion.borrow_mut());
        match landed {
            Ok(deleted) => {
                router.notify(Notice::Toast(
                    ToastKind::Success,
                    lib_locale::msg::toast_account_deleted(
                        &deleted.name,
                        count(deleted.transactions),
                    ),
                ));
                router.close_dialog(Some(deleted));
            }
            Err(Error::PaidFromByBillPlans { plans }) => {
                self.plans = plans;
                self.refusal = None;
            }
            Err(Error::NotFound) => {
                self.refusal = Some(msg::account_deletion_error_gone());
            }
        }
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

    use lib_accounts::AccountRepository as _;
    use lib_bills::BillPlanRepository as _;
    use lib_core::UnitId;

    use super::*;
    use crate::fake::world::{EVERYDAY, WALLET, world};
    use crate::fake::{InMemoryDeletion, InMemoryReads, RecordingRouter};

    /// The Delete Dialog on the world, its Wallet named by 2 Transactions and 1 Budget.
    struct Dialog {
        deletion: Rc<RefCell<InMemoryDeletion>>,
        presenter: DeleteAccountPresenter,
        router: RecordingRouter,
    }

    impl Dialog {
        fn open(id: AccountId) -> Self {
            Self::open_on(Rc::new(RefCell::new(world())), id)
        }

        fn open_on(deletion: Rc<RefCell<InMemoryDeletion>>, id: AccountId) -> Self {
            lib_locale::init_with_layers("en-US", &[msg::LAYER]);
            let mut reads = InMemoryReads::default();
            reads.usage.uses.insert(
                WALLET,
                AccountUse {
                    transactions: 2,
                    budgets: 1,
                },
            );
            reads.unit_codes.insert(UnitId::SEED_AUD, "aud".into());
            let shared: SharedDeletion = deletion.clone();
            let presenter =
                DeleteAccountPresenter::new(shared, &reads, id).expect("a world Account");
            Self {
                deletion,
                presenter,
                router: RecordingRouter::default(),
            }
        }

        fn send(&mut self, event: DeleteAccountEvent) {
            self.presenter.handle(event, &mut self.router);
        }

        fn typed(&mut self, text: &str) {
            self.send(DeleteAccountEvent::ConfirmText(text.into()));
        }

        fn view(&self) -> DeleteAccountViewModel {
            self.presenter.view_model()
        }
    }

    fn strong(segments: &[Segment]) -> Vec<&str> {
        segments
            .iter()
            .filter(|segment| segment.tag.as_deref() == Some("strong"))
            .map(|segment| segment.text.as_str())
            .collect()
    }

    #[test]
    fn it_quotes_the_balance_the_transactions_and_the_budgets_it_reads_when_it_opens() {
        let dialog = Dialog::open(WALLET);
        let view = dialog.view();

        assert_eq!(view.frame.title, "Delete account \u{2014} Wallet");
        assert_eq!(view.frame.submit, "Delete account");
        assert!(view.frame.destructive);
        assert_eq!(
            strong(&view.warning),
            ["\u{2212}2,318.44 aud", "2 transactions."]
        );
        assert_eq!(
            view.reference,
            "2 transactions will be permanently deleted \u{b7} referenced in 1 budget"
        );
        assert_eq!(
            view.usage,
            AccountUse {
                transactions: 2,
                budgets: 1
            }
        );
        assert_eq!(view.confirm.label, "Type Wallet to confirm");
        assert_eq!(view.confirm.placeholder, "Wallet");
        assert_eq!(view.blocked, None);
    }

    #[test]
    fn an_account_nothing_names_says_so() {
        let view = Dialog::open(crate::fake::world::OFFSET).view();

        assert_eq!(strong(&view.warning)[1], "0 transactions.");
        assert!(
            view.reference.ends_with("not referenced by any budget"),
            "{}",
            view.reference
        );
    }

    #[test]
    fn confirm_waits_for_the_exact_name_then_deletes_raises_its_toast_and_closes() {
        let mut dialog = Dialog::open(WALLET);

        dialog.typed("wallet");
        assert!(!dialog.view().frame.can_confirm);
        dialog.send(DeleteAccountEvent::Confirm);
        assert!(dialog.router.closed.is_empty());
        assert!(dialog.deletion.borrow().accounts.account(WALLET).is_some());

        dialog.typed("Wallet");
        assert!(dialog.view().frame.can_confirm);
        dialog.send(DeleteAccountEvent::Confirm);

        assert_eq!(
            dialog.router.notices,
            [Notice::Toast(
                ToastKind::Success,
                "Deleted account Wallet \u{b7} 2 transactions deleted".into()
            )]
        );
        assert_eq!(
            dialog.router.closed,
            [Some(crate::AccountDeleted {
                id: WALLET,
                name: "Wallet".into(),
                transactions: 2,
                unmatched: 1,
            })]
        );
        assert!(dialog.deletion.borrow().accounts.account(WALLET).is_none());
    }

    #[test]
    fn an_account_a_bill_plan_pays_from_says_how_many_and_never_confirms() {
        let mut dialog = Dialog::open(EVERYDAY);
        dialog.typed("Everyday");
        let view = dialog.view();

        assert_eq!(
            view.blocked.as_deref(),
            Some(
                "1 bill is paid from this account. Move it to another account before deleting \
                 this one."
            )
        );
        assert!(!view.frame.can_confirm);
        dialog.send(DeleteAccountEvent::Confirm);
        assert!(dialog.router.closed.is_empty());
        assert!(
            dialog
                .deletion
                .borrow()
                .accounts
                .account(EVERYDAY)
                .is_some()
        );
    }

    #[test]
    fn a_plan_that_came_to_pay_from_it_under_the_dialog_blocks_it() {
        let deletion = Rc::new(RefCell::new(world()));
        let mut dialog = Dialog::open_on(deletion.clone(), WALLET);
        dialog.typed("Wallet");
        {
            let mut deletion = deletion.borrow_mut();
            let mut plan = deletion.bills.bill_plans()[0].clone();
            plan.account_id = WALLET;
            deletion.bills.upsert_bill_plan(plan);
        }

        dialog.send(DeleteAccountEvent::Confirm);

        assert!(dialog.router.closed.is_empty());
        let view = dialog.view();
        assert!(view.blocked.is_some());
        assert!(!view.frame.can_confirm);
        assert!(deletion.borrow().accounts.account(WALLET).is_some());
    }

    #[test]
    fn an_account_gone_under_the_dialog_is_refused_and_cancel_closes_having_written_nothing() {
        let deletion = Rc::new(RefCell::new(world()));
        let mut dialog = Dialog::open_on(deletion.clone(), WALLET);
        dialog.typed("Wallet");
        deletion.borrow_mut().accounts.delete_account(WALLET);
        let before = deletion.borrow().clone();

        dialog.send(DeleteAccountEvent::Confirm);
        assert_eq!(
            dialog.view().refusal.as_deref(),
            Some("This account has since been deleted.")
        );
        assert!(dialog.router.closed.is_empty());

        dialog.send(DeleteAccountEvent::Cancel);
        assert_eq!(dialog.router.closed, [None]);
        assert!(dialog.router.notices.is_empty());
        assert_eq!(*deletion.borrow(), before);
    }

    #[test]
    fn an_account_gone_opens_no_dialog() {
        let shared: SharedDeletion = Rc::new(RefCell::new(world()));
        assert!(
            DeleteAccountPresenter::new(shared, &InMemoryReads::default(), AccountId::seed(9_999))
                .is_none()
        );
    }
}
