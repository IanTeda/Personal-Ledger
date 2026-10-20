//! The Dialog host's confirm cycle (ADR-0037): a form turns itself into a command, the command
//! runs, and the Dialog closes only when it landed.
//!
//! A form answers [`DialogForm::command`] with the command its fields describe, or a
//! [`FormError`] when a field is not valid yet, and takes a domain refusal back through
//! [`DialogForm::refuse`] so it can show it as a Message; the domain libs stay locale-free.
//!
//! A page whose command is a [`LedgerCommand`] confirms through [`confirm_ledger`], which runs it
//! through [`LedgerStore::apply`] and takes the Toast from the Outcome's [`OutcomeToast`]. The
//! pages not moved yet still go through their own [`Apply`] and [`confirm`] (#719).

use gpui::{App, Entity};
use lib_ledger::LedgerCommand;

use crate::{
    chrome::dialog_host::{DialogToast, Landed},
    ledger_store::LedgerStore,
};

/// A form whose fields don't describe a command yet; the Dialog stays open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormError;

/// A Dialog's form, as the host's confirm cycle sees it.
pub trait DialogForm {
    type Command;
    type Refusal;

    /// The command the fields describe, or [`FormError`] while they are not valid.
    fn command(&self) -> Result<Self::Command, FormError>;

    /// Shows a refusal the domain made, inline in the form, as a Message.
    fn refuse(&mut self, refusal: Self::Refusal);
}

/// Runs a command of type `C` against the data the Dialog edits.
pub trait Apply<C> {
    type Outcome;
    type Refusal;

    fn apply(&mut self, command: C, cx: &mut App) -> Result<Self::Outcome, Self::Refusal>;
}

/// One confirm: `Some(outcome)` when the command landed and the Dialog should close, `None` when
/// the form is not valid or the domain refused it (the refusal is already on the form).
pub fn confirm<F, A>(form: &mut F, apply: &mut A, cx: &mut App) -> Option<A::Outcome>
where
    F: DialogForm,
    A: Apply<F::Command, Refusal = F::Refusal>,
{
    let command = form.command().ok()?;
    match apply.apply(command, cx) {
        Ok(outcome) => Some(outcome),
        Err(refusal) => {
            form.refuse(refusal);
            None
        }
    }
}

/// The Toast a landed command's Outcome raises. Most Outcomes have none, so that's the default.
pub trait OutcomeToast {
    fn toast(&self) -> Option<DialogToast> {
        None
    }
}

/// One confirm of a form whose command is a [`LedgerCommand`]: runs it through
/// [`LedgerStore::apply`] and lands it with its Outcome's Toast, or hands the refusal back to the
/// form and stays open. A function rather than a blanket `ConfirmDialog` impl, which would
/// overlap with a Dialog, such as Tracing's, that confirms without the Ledger.
pub fn confirm_ledger<F>(
    form: &mut F,
    ledger: &Entity<LedgerStore>,
    cx: &mut App,
) -> Option<Landed<<F::Command as LedgerCommand>::Outcome>>
where
    F: DialogForm,
    F::Command: LedgerCommand<Refusal = F::Refusal>,
    <F::Command as LedgerCommand>::Outcome: OutcomeToast,
{
    let command = form.command().ok()?;
    match ledger.update(cx, |store, cx| store.apply(cx, command)) {
        Ok(outcome) => Some(Landed {
            toast: outcome.toast(),
            outcome,
        }),
        Err(refusal) => {
            form.refuse(refusal);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Form {
        text: String,
        shown: Option<String>,
    }

    impl DialogForm for Form {
        type Command = String;
        type Refusal = String;

        fn command(&self) -> Result<String, FormError> {
            if self.text.is_empty() {
                Err(FormError)
            } else {
                Ok(self.text.clone())
            }
        }

        fn refuse(&mut self, refusal: String) {
            self.shown = Some(refusal);
        }
    }

    /// Refuses a command it has seen before.
    #[derive(Default)]
    struct Seen(Vec<String>);

    impl Apply<String> for Seen {
        type Outcome = usize;
        type Refusal = String;

        fn apply(&mut self, command: String, _cx: &mut App) -> Result<usize, String> {
            if self.0.contains(&command) {
                return Err(format!("{command} is taken"));
            }
            self.0.push(command);
            Ok(self.0.len())
        }
    }

    #[gpui::test]
    fn a_landed_command_closes_and_a_refused_one_stays_open_with_the_refusal(
        cx: &mut gpui::TestAppContext,
    ) {
        let mut seen = Seen::default();
        cx.update(|cx| {
            let mut form = Form::default();
            assert_eq!(
                confirm(&mut form, &mut seen, cx),
                None,
                "invalid stays open"
            );
            assert!(seen.0.is_empty() && form.shown.is_none());

            form.text = "Netflix".into();
            assert_eq!(confirm(&mut form, &mut seen, cx), Some(1));

            assert_eq!(confirm(&mut form, &mut seen, cx), None);
            assert_eq!(form.shown.as_deref(), Some("Netflix is taken"));
        });
    }

    /// Adds an Institution unless one already has the name.
    struct AddOnce(String);

    #[derive(Debug, PartialEq, Eq)]
    struct Added(usize);

    impl OutcomeToast for Added {
        fn toast(&self) -> Option<DialogToast> {
            Some((lib_toast::ToastKind::Success, format!("row {}", self.0)))
        }
    }

    impl LedgerCommand for AddOnce {
        type Outcome = Added;
        type Refusal = String;

        fn run(self, ledger: &mut lib_ledger::Ledger) -> Result<Added, String> {
            if ledger.institutions().iter().any(|row| row.name == self.0) {
                return Err(format!("{} is taken", self.0));
            }
            ledger.add_institution(self.0, String::new());
            Ok(Added(ledger.institutions().len()))
        }
    }

    #[derive(Default)]
    struct LedgerForm {
        text: String,
        shown: Option<String>,
    }

    impl DialogForm for LedgerForm {
        type Command = AddOnce;
        type Refusal = String;

        fn command(&self) -> Result<AddOnce, FormError> {
            if self.text.is_empty() {
                Err(FormError)
            } else {
                Ok(AddOnce(self.text.clone()))
            }
        }

        fn refuse(&mut self, refusal: String) {
            self.shown = Some(refusal);
        }
    }

    fn institution_count(ledger: &Entity<LedgerStore>, cx: &gpui::TestAppContext) -> usize {
        ledger.read_with(cx, |store, _| store.ledger().institutions().len())
    }

    #[gpui::test]
    fn confirm_ledger_leaves_an_invalid_form_open_and_runs_nothing(cx: &mut gpui::TestAppContext) {
        let ledger = crate::ledger_store::test_ledger(cx, lib_ledger::LedgerParts::default());
        let mut form = LedgerForm::default();

        let landed = cx.update(|cx| confirm_ledger(&mut form, &ledger, cx));

        assert!(landed.is_none());
        assert!(form.shown.is_none());
        assert_eq!(institution_count(&ledger, cx), 0);
    }

    #[gpui::test]
    fn confirm_ledger_lands_the_command_with_its_outcome_toast(cx: &mut gpui::TestAppContext) {
        let ledger = crate::ledger_store::test_ledger(cx, lib_ledger::LedgerParts::default());
        let mut form = LedgerForm {
            text: "Bank X".into(),
            ..LedgerForm::default()
        };

        let landed = cx
            .update(|cx| confirm_ledger(&mut form, &ledger, cx))
            .expect("a new name lands");

        assert_eq!(landed.outcome, Added(1));
        assert_eq!(
            landed.toast,
            Some((lib_toast::ToastKind::Success, "row 1".into()))
        );
        assert_eq!(institution_count(&ledger, cx), 1);
    }

    #[gpui::test]
    fn confirm_ledger_hands_a_refusal_to_the_form_and_stays_open(cx: &mut gpui::TestAppContext) {
        let ledger = crate::ledger_store::test_ledger(cx, lib_ledger::LedgerParts::default());
        let mut form = LedgerForm {
            text: "Bank X".into(),
            ..LedgerForm::default()
        };
        cx.update(|cx| confirm_ledger(&mut form, &ledger, cx))
            .expect("the first one lands");

        let landed = cx.update(|cx| confirm_ledger(&mut form, &ledger, cx));

        assert!(landed.is_none());
        assert_eq!(form.shown.as_deref(), Some("Bank X is taken"));
        assert_eq!(institution_count(&ledger, cx), 1);
    }
}
