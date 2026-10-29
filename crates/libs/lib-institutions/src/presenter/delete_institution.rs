//! The Delete institution Dialog: the destructive frame, the Institution's name typed back
//! exactly (case-sensitive) before **Delete institution** enables.
//!
//! An Institution an Account names can't be deleted, so the Dialog opens refused instead: it says
//! how many Accounts use it, lists the first few by name, and offers only **Close**. Usage is read
//! when it opens and again when it confirms, so an Account that came to name the Institution under
//! the open Dialog turns it into the refusal rather than deleting. The Dialog raises its own Toast
//! on landing, so the page only learns what it removed.

use lib_core::InstitutionId;
use lib_toast::{Notice, ToastKind};
use lib_viper::view::{DialogFrame, TextFieldView};

use crate::interactor::{DeleteInstitution, DeleteInstitutionRefusal};
use crate::msg;
use crate::protocols::{InstitutionUse, InstitutionsOutcome, InstitutionsRouter, SharedPorts};

/// How many of the Accounts naming the Institution the refusal lists by name; the rest are
/// counted.
const NAMES_SHOWN: usize = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeleteInstitutionEvent {
    /// The confirmation field's whole new text.
    ConfirmText(String),
    /// Deletes once the name is typed back; in a refusal, closes the Dialog.
    Confirm,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteInstitutionViewModel {
    /// A refusal's frame has the Close button as its `submit`, never confirmable or destructive.
    pub frame: DialogFrame,
    /// What the Dialog says about the Institution: what deleting does, or the refusal's
    /// "Used by N accounts", or that it has since gone.
    pub notice: String,
    /// The first Accounts naming the Institution, in the Accounts' order; empty unless refused
    /// for use.
    pub accounts: Vec<String>,
    /// `and N more`, when more Accounts name it than are listed.
    pub more: Option<String>,
    /// What to do about a refusal for use.
    pub hint: Option<String>,
    /// The typed-back name; `None` once the Dialog is a refusal, which has nothing to type.
    pub confirm: Option<TextFieldView>,
}

enum State {
    /// No Account names it: the name typed back confirms.
    Unused,
    /// Accounts name it, so only Close is offered.
    InUse(InstitutionUse),
    /// It was deleted under the open Dialog.
    Gone,
}

pub struct DeleteInstitutionPresenter {
    ports: SharedPorts,
    id: InstitutionId,
    name: String,
    state: State,
    confirm_text: String,
}

impl DeleteInstitutionPresenter {
    /// The Dialog for Institution `id`, refused from the start when Accounts name it, or `None`
    /// when it is gone.
    pub fn new(ports: SharedPorts, id: InstitutionId) -> Option<Self> {
        let (name, uses) = {
            let ports = ports.borrow();
            (
                ports.institutions().institution(id)?.name,
                ports.usage().institution_usage(id),
            )
        };
        Some(Self {
            ports,
            id,
            name,
            state: if uses.is_used() {
                State::InUse(uses)
            } else {
                State::Unused
            },
            confirm_text: String::new(),
        })
    }

    pub fn handle(&mut self, event: DeleteInstitutionEvent, router: &mut dyn InstitutionsRouter) {
        match event {
            DeleteInstitutionEvent::ConfirmText(text) => {
                if matches!(self.state, State::Unused) {
                    self.confirm_text = text;
                }
            }
            DeleteInstitutionEvent::Confirm => self.confirm(router),
            DeleteInstitutionEvent::Cancel => router.close_dialog(None),
        }
    }

    pub fn view_model(&self) -> DeleteInstitutionViewModel {
        let refused = !matches!(self.state, State::Unused);
        let (notice, accounts, more, hint) = match &self.state {
            State::Unused => (msg::institutions_delete_warning(), Vec::new(), None, None),
            State::InUse(uses) => (
                msg::institutions_delete_used_by(count(uses.accounts.len())),
                uses.accounts.iter().take(NAMES_SHOWN).cloned().collect(),
                uses.accounts
                    .len()
                    .checked_sub(NAMES_SHOWN)
                    .filter(|left| *left > 0)
                    .map(|left| msg::institutions_delete_and_more(&left.to_string())),
                Some(msg::institutions_delete_used_by_hint()),
            ),
            State::Gone => (msg::institutions_error_not_found(), Vec::new(), None, None),
        };
        DeleteInstitutionViewModel {
            frame: DialogFrame {
                title: msg::institutions_delete_title(&self.name),
                submit: if refused {
                    msg::institutions_delete_close()
                } else {
                    msg::institutions_delete_submit()
                },
                can_confirm: self.can_confirm(),
                destructive: !refused,
                hints: Vec::new(),
            },
            notice,
            accounts,
            more,
            hint,
            confirm: (!refused).then(|| TextFieldView {
                label: msg::institutions_delete_confirm_label(&self.name),
                text: self.confirm_text.clone(),
                placeholder: self.name.clone(),
                focused: true,
            }),
        }
    }

    /// The row after this Institution, else the one before it.
    fn neighbour(&self) -> Option<InstitutionId> {
        let ids: Vec<InstitutionId> = self
            .ports
            .borrow()
            .institutions()
            .institutions()
            .into_iter()
            .map(|institution| institution.id)
            .collect();
        let at = ids.iter().position(|row| *row == self.id)?;
        ids.get(at + 1)
            .or_else(|| at.checked_sub(1).and_then(|before| ids.get(before)))
            .copied()
    }

    /// The typed name must match exactly, case and all; a refusal never confirms.
    fn can_confirm(&self) -> bool {
        matches!(self.state, State::Unused) && self.confirm_text == self.name
    }

    fn confirm(&mut self, router: &mut dyn InstitutionsRouter) {
        if !matches!(self.state, State::Unused) {
            router.close_dialog(None);
            return;
        }
        if !self.can_confirm() {
            return;
        }
        // Read before the row goes, from the list as it is now, not as it was when the Dialog opened.
        let next = self.neighbour();
        let landed = DeleteInstitution { id: self.id }.run(&mut *self.ports.borrow_mut());
        match landed {
            Ok(deleted) => {
                router.notify(Notice::Toast(
                    ToastKind::Success,
                    msg::institutions_delete_toast(&deleted.name),
                ));
                router.close_dialog(Some(InstitutionsOutcome {
                    id: deleted.id,
                    name: deleted.name,
                    next,
                }));
            }
            Err(DeleteInstitutionRefusal::InUse(uses)) => self.state = State::InUse(uses),
            Err(DeleteInstitutionRefusal::NotFound) => self.state = State::Gone,
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

    use super::*;
    use crate::InstitutionRepository as _;
    use crate::fake::{InMemoryPorts, RecordingRouter, RouterCall};
    use crate::presenter::test_support::stub;

    struct Dialog {
        ports: Rc<RefCell<InMemoryPorts>>,
        presenter: DeleteInstitutionPresenter,
        router: RecordingRouter,
    }

    impl Dialog {
        fn open(id: InstitutionId) -> Self {
            let (ports, shared) = stub();
            Self::on(ports, shared, id)
        }

        fn on(ports: Rc<RefCell<InMemoryPorts>>, shared: SharedPorts, id: InstitutionId) -> Self {
            Self {
                ports,
                presenter: DeleteInstitutionPresenter::new(shared, id).expect("a stub row"),
                router: RecordingRouter::default(),
            }
        }

        fn send(&mut self, event: DeleteInstitutionEvent) -> Vec<RouterCall> {
            self.presenter.handle(event, &mut self.router);
            self.router.take()
        }

        fn type_text(&mut self, text: &str) {
            self.send(DeleteInstitutionEvent::ConfirmText(text.to_string()));
        }

        fn view(&self) -> DeleteInstitutionViewModel {
            self.presenter.view_model()
        }

        fn name_accounts(&self, id: InstitutionId, names: &[&str]) {
            self.ports
                .borrow_mut()
                .usage
                .accounts
                .insert(id, names.iter().map(ToString::to_string).collect());
        }
    }

    #[test]
    fn an_unused_institution_asks_for_its_name_typed_back_before_it_enables() {
        let mut dialog = Dialog::open(InstitutionId::SEED_AMEX);

        let view = dialog.view();
        assert_eq!(
            view.frame.title,
            "Delete institution \u{2014} American Express"
        );
        assert_eq!(view.frame.submit, "Delete institution");
        assert!(view.frame.destructive);
        assert!(!view.frame.can_confirm);
        assert_eq!(
            view.notice,
            "No account uses this institution. Deleting it cannot be undone."
        );
        let confirm = view.confirm.expect("a typed-back field");
        assert_eq!(confirm.label, "Type American Express to confirm");
        assert_eq!(confirm.placeholder, "American Express");

        dialog.type_text("American");
        assert!(!dialog.view().frame.can_confirm);
        dialog.type_text("american express");
        assert!(!dialog.view().frame.can_confirm, "capitals count");
        dialog.type_text("American Express");
        assert!(dialog.view().frame.can_confirm);
    }

    #[test]
    fn confirming_before_the_name_is_typed_back_does_nothing() {
        let mut dialog = Dialog::open(InstitutionId::SEED_AMEX);

        assert_eq!(dialog.send(DeleteInstitutionEvent::Confirm), []);
        assert_eq!(dialog.ports.borrow().institutions.institutions.len(), 7);
    }

    #[test]
    fn confirming_deletes_raises_a_toast_and_closes_with_what_went() {
        let mut dialog = Dialog::open(InstitutionId::SEED_AMEX);
        dialog.type_text("American Express");

        let calls = dialog.send(DeleteInstitutionEvent::Confirm);

        assert_eq!(
            calls,
            [
                RouterCall::Notice(Notice::Toast(
                    ToastKind::Success,
                    "Deleted American Express".into()
                )),
                RouterCall::CloseDialog(Some(InstitutionsOutcome {
                    id: InstitutionId::SEED_AMEX,
                    name: "American Express".into(),
                    next: Some(InstitutionId::SEED_VANGUARD),
                })),
            ]
        );
        let ports = dialog.ports.borrow();
        assert_eq!(ports.institutions.institutions.len(), 6);
        assert_eq!(
            ports.institutions.institution(InstitutionId::SEED_AMEX),
            None
        );
    }

    #[test]
    fn the_row_to_select_next_is_read_when_the_delete_is_confirmed() {
        let mut dialog = Dialog::open(InstitutionId::SEED_AMEX);
        dialog.type_text("American Express");
        dialog
            .ports
            .borrow_mut()
            .institutions
            .institutions
            .retain(|row| row.id != InstitutionId::SEED_VANGUARD);

        let calls = dialog.send(DeleteInstitutionEvent::Confirm);

        assert!(matches!(
            calls.last(),
            Some(RouterCall::CloseDialog(Some(InstitutionsOutcome {
                next: Some(InstitutionId::SEED_WESTPAC),
                ..
            })))
        ));
    }

    #[test]
    fn cancel_closes_with_nothing_deleted() {
        let mut dialog = Dialog::open(InstitutionId::SEED_AMEX);
        dialog.type_text("American Express");

        assert_eq!(
            dialog.send(DeleteInstitutionEvent::Cancel),
            [RouterCall::CloseDialog(None)]
        );
        assert_eq!(dialog.ports.borrow().institutions.institutions.len(), 7);
    }

    #[test]
    fn an_institution_in_use_opens_refused_with_up_to_five_names_then_and_n_more() {
        let (ports, shared) = stub();
        let names = [
            "Everyday",
            "Savings",
            "Offset",
            "Holiday",
            "Emergency",
            "Car",
            "Gifts",
        ];
        ports.borrow_mut().usage.accounts.insert(
            InstitutionId::SEED_ANZ,
            names.iter().map(ToString::to_string).collect(),
        );

        let dialog = Dialog::on(ports, shared, InstitutionId::SEED_ANZ);

        let view = dialog.view();
        assert_eq!(view.notice, "Used by 7 accounts");
        assert_eq!(view.accounts, names[..5]);
        assert_eq!(view.more.as_deref(), Some("and 2 more"));
        assert_eq!(view.confirm, None, "nothing to type back");
        assert_eq!(view.frame.submit, "Close");
        assert!(!view.frame.can_confirm);
        assert!(!view.frame.destructive);
    }

    #[test]
    fn a_refusal_lists_five_or_fewer_names_without_a_more_line() {
        let (ports, shared) = stub();
        ports.borrow_mut().usage.accounts.insert(
            InstitutionId::SEED_ANZ,
            ["Everyday", "Savings", "Offset", "Holiday", "Emergency"]
                .map(String::from)
                .to_vec(),
        );
        let five = Dialog::on(ports.clone(), shared.clone(), InstitutionId::SEED_ANZ);
        assert_eq!(five.view().accounts.len(), 5);
        assert_eq!(five.view().more, None);

        ports
            .borrow_mut()
            .usage
            .accounts
            .insert(InstitutionId::SEED_AMEX, vec!["Amex Platinum".into()]);
        let one = Dialog::on(ports, shared, InstitutionId::SEED_AMEX);
        assert_eq!(one.view().notice, "Used by 1 account");
        assert_eq!(one.view().accounts, ["Amex Platinum"]);
    }

    #[test]
    fn a_refusal_ignores_typing_and_close_leaves_everything_as_it_was() {
        let (ports, shared) = stub();
        ports
            .borrow_mut()
            .usage
            .accounts
            .insert(InstitutionId::SEED_AMEX, vec!["Amex Platinum".into()]);
        let mut dialog = Dialog::on(ports, shared, InstitutionId::SEED_AMEX);

        dialog.type_text("American Express");
        assert!(!dialog.view().frame.can_confirm);
        assert_eq!(
            dialog.send(DeleteInstitutionEvent::Confirm),
            [RouterCall::CloseDialog(None)],
            "Confirm is Close here"
        );
        assert_eq!(dialog.ports.borrow().institutions.institutions.len(), 7);
    }

    #[test]
    fn an_account_coming_to_name_it_under_the_open_dialog_turns_it_into_the_refusal() {
        let mut dialog = Dialog::open(InstitutionId::SEED_AMEX);
        dialog.type_text("American Express");
        assert!(dialog.view().frame.can_confirm);
        dialog.name_accounts(InstitutionId::SEED_AMEX, &["Amex Platinum", "Amex Gold"]);

        assert_eq!(dialog.send(DeleteInstitutionEvent::Confirm), []);

        let view = dialog.view();
        assert_eq!(view.notice, "Used by 2 accounts");
        assert_eq!(view.accounts, ["Amex Platinum", "Amex Gold"]);
        assert_eq!(view.confirm, None);
        assert_eq!(dialog.ports.borrow().institutions.institutions.len(), 7);
        assert_eq!(
            dialog.send(DeleteInstitutionEvent::Confirm),
            [RouterCall::CloseDialog(None)]
        );
    }

    #[test]
    fn an_institution_deleted_under_the_open_dialog_says_it_has_gone() {
        let mut dialog = Dialog::open(InstitutionId::SEED_AMEX);
        dialog.type_text("American Express");
        dialog
            .ports
            .borrow_mut()
            .institutions
            .delete_institution(InstitutionId::SEED_AMEX);

        assert_eq!(dialog.send(DeleteInstitutionEvent::Confirm), []);

        let view = dialog.view();
        assert_eq!(view.notice, "That institution is no longer on the ledger");
        assert_eq!(view.confirm, None);
        assert_eq!(view.frame.submit, "Close");
    }

    #[test]
    fn a_missing_institution_opens_no_dialog() {
        let (_, shared) = stub();

        assert!(DeleteInstitutionPresenter::new(shared, InstitutionId::new()).is_none());
    }
}
