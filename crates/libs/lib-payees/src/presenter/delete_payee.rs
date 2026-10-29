//! The Delete payee Dialog (6d), and the Deactivate and Reactivate Dialogs it turns into (#283).
//!
//! It reads the Payee's use when it opens: a Payee nothing names is deleted, one in use can only be
//! deactivated, and an inactive one in use is offered reactivation. A use that can't be counted is
//! taken as in use, and the warning says it couldn't be checked (#840). Delete and Deactivate sit
//! behind the typed-back name (exact, case-sensitive) and the destructive frame; reactivating
//! loses nothing, so it drops both. A Payee that came into use under the open Dialog turns it into
//! Deactivate, as the Interactor's refusal says.
//!
//! The Dialog raises its own Toast on landing, so the page only learns what it did.

use lib_core::PayeeId;
use lib_locale::Segment;
use lib_toast::{Notice, ToastKind};
use lib_viper::view::{DialogFrame, TextFieldView};

use super::{count, refusal_text};
use crate::interactor::{DeactivatePayee, DeletePayee, ReactivatePayee};
use crate::protocols::{PayeeChange, PayeesOutcome, PayeesRouter, SharedPorts};
use crate::{DeletePayeeRefusal, PayeeError, PayeeUse, msg};

/// What the Dialog does to the Payee, chosen from its use when it opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeleteAction {
    Delete,
    Deactivate,
    Reactivate,
}

impl DeleteAction {
    /// `uses` is `None` when it couldn't be counted, which offers what a Payee in use is offered.
    pub fn for_payee(is_active: bool, uses: Option<PayeeUse>) -> Self {
        if uses.is_some_and(|uses| !uses.is_used()) {
            Self::Delete
        } else if is_active {
            Self::Deactivate
        } else {
            Self::Reactivate
        }
    }

    /// Reactivating loses nothing, so it skips the typed-back name and the destructive frame.
    fn is_destructive(self) -> bool {
        self != Self::Reactivate
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeletePayeeEvent {
    /// The confirmation field's whole new text.
    ConfirmText(String),
    Confirm,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeletePayeeViewModel {
    pub frame: DialogFrame,
    pub action: DeleteAction,
    /// What the action does, its emphasis tagged `strong`.
    pub warning: Vec<Segment>,
    /// What goes with it: the match rules a delete removes, or what deactivating keeps.
    pub callout: String,
    /// `Type J Smith to confirm`; `None` to reactivate.
    pub confirm: Option<TextFieldView>,
    /// Why the last confirm was refused.
    pub refusal: Option<String>,
}

pub struct DeletePayeePresenter {
    ports: SharedPorts,
    id: PayeeId,
    name: String,
    rules: usize,
    /// `None` when it couldn't be counted.
    uses: Option<PayeeUse>,
    action: DeleteAction,
    confirm_text: String,
    refusal: Option<String>,
}

impl DeletePayeePresenter {
    /// The Dialog for Payee `id`, or `None` when it is gone.
    pub fn new(ports: SharedPorts, id: PayeeId) -> Option<Self> {
        let (payee, uses) = {
            let ports = ports.borrow();
            (ports.payees().payee(id)?, ports.usage().payee_usage(id))
        };
        Some(Self {
            ports,
            id,
            action: DeleteAction::for_payee(payee.is_active, uses),
            name: payee.name,
            rules: payee.aliases.len(),
            uses,
            confirm_text: String::new(),
            refusal: None,
        })
    }

    pub fn handle(&mut self, event: DeletePayeeEvent, router: &mut dyn PayeesRouter) {
        match event {
            DeletePayeeEvent::ConfirmText(text) => self.confirm_text = text,
            DeletePayeeEvent::Confirm => self.confirm(router),
            DeletePayeeEvent::Cancel => router.close_dialog(None),
        }
    }

    pub fn view_model(&self) -> DeletePayeeViewModel {
        let name = self.name.as_str();
        let (title, submit, warning, callout) = match self.action {
            DeleteAction::Delete => (
                msg::payees_delete_title(name),
                msg::payees_delete_submit(),
                msg::payees_delete_warning(),
                msg::payees_delete_callout(count(self.rules)),
            ),
            DeleteAction::Deactivate => (
                msg::payees_deactivate_title(name),
                msg::payees_deactivate_submit(),
                self.uses
                    .map_or_else(msg::payees_deactivate_warning_unchecked, |uses| {
                        msg::payees_deactivate_warning(count(uses.splits), count(uses.bill_plans))
                    }),
                msg::payees_deactivate_callout(),
            ),
            DeleteAction::Reactivate => (
                msg::payees_reactivate_title(name),
                msg::payees_reactivate_submit(),
                self.uses
                    .map_or_else(msg::payees_reactivate_warning_unchecked, |uses| {
                        msg::payees_reactivate_warning(count(uses.splits), count(uses.bill_plans))
                    }),
                msg::payees_reactivate_callout(),
            ),
        };
        let destructive = self.action.is_destructive();
        DeletePayeeViewModel {
            frame: DialogFrame {
                title,
                submit,
                can_confirm: self.can_confirm(),
                destructive,
                hints: Vec::new(),
            },
            action: self.action,
            warning,
            callout,
            confirm: destructive.then(|| TextFieldView {
                label: msg::payees_delete_confirm_label(name),
                text: self.confirm_text.clone(),
                placeholder: self.name.clone(),
                focused: true,
            }),
            refusal: self.refusal.clone(),
        }
    }

    /// The typed name must match exactly, case and all, for a destructive action.
    fn can_confirm(&self) -> bool {
        !self.action.is_destructive() || self.confirm_text == self.name
    }

    fn confirm(&mut self, router: &mut dyn PayeesRouter) {
        if !self.can_confirm() {
            return;
        }
        let landed = {
            let mut ports = self.ports.borrow_mut();
            let ports = ports.split_mut();
            match self.action {
                DeleteAction::Delete => DeletePayee(self.id)
                    .run(ports)
                    .map_err(|refusal| match refusal {
                        DeletePayeeRefusal::InUse(uses) => Refused::InUse(Some(uses)),
                        DeletePayeeRefusal::Unchecked => Refused::InUse(None),
                        DeletePayeeRefusal::NotFound => Refused::Gone,
                    })
                    .map(|payee| (PayeeChange::Deleted, payee.name)),
                DeleteAction::Deactivate => DeactivatePayee(self.id)
                    .run(ports.payees)
                    .map(|payee| (PayeeChange::Deactivated, payee.name))
                    .map_err(|_: PayeeError| Refused::Gone),
                DeleteAction::Reactivate => ReactivatePayee(self.id)
                    .run(ports.payees)
                    .map(|payee| (PayeeChange::Reactivated, payee.name))
                    .map_err(|_: PayeeError| Refused::Gone),
            }
        };
        match landed {
            Ok((change, name)) => {
                let toast = match change {
                    PayeeChange::Deactivated => lib_locale::msg::toast_payee_deactivated(&name),
                    PayeeChange::Reactivated => lib_locale::msg::toast_payee_reactivated(&name),
                    PayeeChange::Deleted => lib_locale::msg::toast_payee_deleted(&name),
                    // The Delete Dialog never saves a Payee, so no Toast is due for one.
                    PayeeChange::Saved => {
                        debug_assert!(false, "the Delete payee Dialog doesn't save a payee");
                        lib_locale::msg::toast_payee_deleted(&name)
                    }
                };
                router.notify(Notice::Toast(ToastKind::Success, toast));
                router.close_dialog(Some(PayeesOutcome {
                    change,
                    id: self.id,
                    name,
                }));
            }
            // It came into use under the Dialog, or its use can no longer be counted: what's left
            // to offer is Deactivate.
            Err(Refused::InUse(uses)) => {
                self.uses = uses;
                self.action = DeleteAction::Deactivate;
                self.refusal = None;
            }
            Err(Refused::Gone) => self.refusal = Some(refusal_text(&PayeeError::NotFound)),
        }
    }
}

/// Why a confirm didn't land.
enum Refused {
    Gone,
    /// What names the Payee, `None` when it couldn't be counted.
    InUse(Option<PayeeUse>),
}

#[cfg(test)]
mod tests {
    use super::super::test_support::stub;
    use super::*;
    use crate::PayeeRepository as _;
    use crate::fake::{RecordingRouter, RouterCall};

    const WOOLWORTHS: PayeeId = PayeeId::seed(1);
    const NETFLIX: PayeeId = PayeeId::seed(3);
    const DON_QUIJOTE: PayeeId = PayeeId::seed(18);

    fn used(splits: usize) -> PayeeUse {
        PayeeUse {
            splits,
            bill_plans: 0,
        }
    }

    fn used_by(splits: usize, bill_plans: usize) -> PayeeUse {
        PayeeUse { splits, bill_plans }
    }

    fn send(dialog: &mut DeletePayeePresenter, events: Vec<DeletePayeeEvent>) -> Vec<RouterCall> {
        let mut router = RecordingRouter::default();
        for event in events {
            dialog.handle(event, &mut router);
        }
        router.calls
    }

    fn typed(text: &str) -> DeletePayeeEvent {
        DeletePayeeEvent::ConfirmText(text.to_string())
    }

    fn toast(text: &str) -> RouterCall {
        RouterCall::Notice(Notice::Toast(ToastKind::Success, text.to_string()))
    }

    #[test]
    fn the_three_outcomes_are_chosen_from_the_payees_use_when_it_opens() {
        let (ports, shared) = stub();
        ports.borrow_mut().usage.uses.insert(WOOLWORTHS, used(4));
        ports.borrow_mut().usage.uses.insert(DON_QUIJOTE, used(1));
        let open = |id| {
            DeletePayeePresenter::new(shared.clone(), id)
                .expect("a stub Payee")
                .view_model()
        };

        let delete = open(NETFLIX);
        assert_eq!(delete.action, DeleteAction::Delete);
        assert_eq!(delete.frame.title, "Delete payee — Netflix");
        assert!(delete.frame.destructive);
        assert_eq!(delete.callout, "Deleting also removes its 1 match rule.");
        let deactivate = open(WOOLWORTHS);
        assert_eq!(deactivate.action, DeleteAction::Deactivate);
        let strong: Vec<&str> = deactivate
            .warning
            .iter()
            .filter(|segment| segment.tag.as_deref() == Some("strong"))
            .map(|segment| segment.text.as_str())
            .collect();
        assert_eq!(strong.concat(), "4 transactions");
        assert!(deactivate.confirm.is_some());
        let reactivate = open(DON_QUIJOTE);
        assert_eq!(reactivate.action, DeleteAction::Reactivate);
        assert!(!reactivate.frame.destructive);
        assert_eq!(reactivate.confirm, None);
        assert!(
            reactivate.frame.can_confirm,
            "reactivating needs no typed name"
        );
        assert!(DeletePayeePresenter::new(shared, PayeeId::new()).is_none());
    }

    #[test]
    fn delete_waits_for_the_exact_name_then_deletes_and_raises_its_toast() {
        let (ports, shared) = stub();
        let mut dialog = DeletePayeePresenter::new(shared, NETFLIX).expect("a stub Payee");

        assert_eq!(
            send(
                &mut dialog,
                vec![typed("netflix"), DeletePayeeEvent::Confirm]
            ),
            []
        );
        assert!(!dialog.view_model().frame.can_confirm);

        let calls = send(
            &mut dialog,
            vec![typed("Netflix"), DeletePayeeEvent::Confirm],
        );

        assert_eq!(
            calls,
            [
                toast("Deleted payee Netflix"),
                RouterCall::CloseDialog(Some(PayeesOutcome {
                    change: PayeeChange::Deleted,
                    id: NETFLIX,
                    name: "Netflix".into(),
                })),
            ]
        );
        assert_eq!(ports.borrow().payees.payee(NETFLIX), None);
    }

    #[test]
    fn deactivate_and_reactivate_keep_the_payee_and_raise_their_toasts() {
        let (ports, shared) = stub();
        ports.borrow_mut().usage.uses.insert(WOOLWORTHS, used(4));
        let mut deactivate = DeletePayeePresenter::new(shared.clone(), WOOLWORTHS).expect("stub");

        let calls = send(
            &mut deactivate,
            vec![typed("Woolworths"), DeletePayeeEvent::Confirm],
        );
        assert_eq!(calls[0], toast("Deactivated payee Woolworths"));
        assert!(
            !ports
                .borrow()
                .payees
                .payee(WOOLWORTHS)
                .is_some_and(|payee| payee.is_active)
        );

        let mut reactivate = DeletePayeePresenter::new(shared, WOOLWORTHS).expect("stub");
        let calls = send(&mut reactivate, vec![DeletePayeeEvent::Confirm]);
        assert_eq!(calls[0], toast("Reactivated payee Woolworths"));
        assert!(
            ports
                .borrow()
                .payees
                .payee(WOOLWORTHS)
                .is_some_and(|payee| payee.is_active)
        );
    }

    #[test]
    fn a_payee_that_came_into_use_under_the_dialog_turns_it_into_deactivate() {
        let (ports, shared) = stub();
        let mut dialog = DeletePayeePresenter::new(shared, NETFLIX).expect("a stub Payee");
        ports.borrow_mut().usage.uses.insert(NETFLIX, used(2));

        assert_eq!(
            send(
                &mut dialog,
                vec![typed("Netflix"), DeletePayeeEvent::Confirm]
            ),
            []
        );

        let view = dialog.view_model();
        assert_eq!(view.action, DeleteAction::Deactivate);
        assert_eq!(view.frame.title, "Deactivate payee — Netflix");
        assert!(ports.borrow().payees.payee(NETFLIX).is_some());
    }

    fn plain(segments: &[Segment]) -> String {
        segments
            .iter()
            .map(|segment| segment.text.as_str())
            .collect()
    }

    #[test]
    fn the_warnings_count_what_names_the_payee() {
        let (ports, shared) = stub();
        let warnings = |id, uses| {
            ports.borrow_mut().usage.uses.insert(id, uses);
            let view = DeletePayeePresenter::new(shared.clone(), id)
                .expect("a stub Payee")
                .view_model();
            plain(&view.warning)
        };

        assert_eq!(
            warnings(NETFLIX, used_by(0, 1)),
            "This payee is used by 1 bill plan, so it can't be deleted — only deactivated."
        );
        assert_eq!(
            warnings(NETFLIX, used_by(0, 3)),
            "This payee is used by 3 bill plans, so it can't be deleted — only deactivated."
        );
        assert_eq!(
            warnings(NETFLIX, used_by(2, 1)),
            "This payee is used by 2 transactions and 1 bill plan, so it can't be deleted — only deactivated."
        );
        assert_eq!(
            warnings(NETFLIX, used_by(1, 0)),
            "This payee is used by 1 transaction, so it can't be deleted — only deactivated."
        );
        assert_eq!(
            warnings(DON_QUIJOTE, used_by(0, 2)),
            "This payee is inactive and used by 2 bill plans."
        );
        assert_eq!(
            warnings(DON_QUIJOTE, used_by(4, 2)),
            "This payee is inactive and used by 4 transactions and 2 bill plans."
        );
    }

    #[test]
    fn a_use_that_cant_be_counted_offers_no_delete_and_says_why() {
        let (ports, shared) = stub();
        ports.borrow_mut().usage.unreadable = true;
        let open = |id| {
            DeletePayeePresenter::new(shared.clone(), id)
                .expect("a stub Payee")
                .view_model()
        };

        let active = open(NETFLIX);
        assert_eq!(active.action, DeleteAction::Deactivate);
        assert_eq!(
            plain(&active.warning),
            "What uses this payee couldn't be checked, so it can't be deleted — only deactivated."
        );
        let inactive = open(DON_QUIJOTE);
        assert_eq!(inactive.action, DeleteAction::Reactivate);
        assert_eq!(
            plain(&inactive.warning),
            "This payee is inactive, and what uses it couldn't be checked."
        );
    }

    #[test]
    fn a_count_that_fails_under_the_dialog_turns_it_into_deactivate() {
        let (ports, shared) = stub();
        let mut dialog = DeletePayeePresenter::new(shared, NETFLIX).expect("a stub Payee");
        ports.borrow_mut().usage.unreadable = true;

        assert_eq!(
            send(
                &mut dialog,
                vec![typed("Netflix"), DeletePayeeEvent::Confirm]
            ),
            []
        );

        let view = dialog.view_model();
        assert_eq!(view.action, DeleteAction::Deactivate);
        assert!(plain(&view.warning).contains("couldn't be checked"));
        assert!(ports.borrow().payees.payee(NETFLIX).is_some());
    }

    #[test]
    fn a_payee_gone_under_the_dialog_is_refused_and_cancel_closes() {
        let (ports, shared) = stub();
        let mut dialog = DeletePayeePresenter::new(shared, NETFLIX).expect("a stub Payee");
        ports.borrow_mut().payees.delete_payee(NETFLIX);

        assert_eq!(
            send(
                &mut dialog,
                vec![typed("Netflix"), DeletePayeeEvent::Confirm]
            ),
            []
        );
        assert_eq!(
            dialog.view_model().refusal.as_deref(),
            Some("This payee has since been deleted.")
        );
        assert_eq!(
            send(&mut dialog, vec![DeletePayeeEvent::Cancel]),
            [RouterCall::CloseDialog(None)]
        );
    }
}
