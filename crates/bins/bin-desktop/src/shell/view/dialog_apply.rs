//! The Dialog host's confirm cycle (ADR-0037): a form turns itself into a command, the command
//! runs, and the Dialog closes only when it landed.
//!
//! A form answers [`DialogForm::command`] with the command its fields describe, or a
//! [`FormError`] when a field is not valid yet, and takes a domain refusal back through
//! [`DialogForm::refuse`] so it can show it as a Message; the domain libs stay locale-free.
//!
//! A page whose command is a [`LedgerCommand`] confirms through [`confirm_ledger`], which runs it
//! through [`LedgerStore::apply`] and takes the Toast from the Outcome's [`OutcomeToast`].

use gpui::{App, Entity};
use lib_ledger::LedgerCommand;

use crate::{
    ledger_store::LedgerStore,
    shell::view::dialog_host::{DialogToast, Landed},
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
    confirm_ledger_with(form, ledger, cx, |command| command)
}

/// [`confirm_ledger`] for a page whose command needs context the form does not hold, such as the
/// Budget on show: `finish` wraps the form's command with it before it runs.
pub fn confirm_ledger_with<F, C>(
    form: &mut F,
    ledger: &Entity<LedgerStore>,
    cx: &mut App,
    finish: impl FnOnce(F::Command) -> C,
) -> Option<Landed<C::Outcome>>
where
    F: DialogForm,
    C: LedgerCommand<Refusal = F::Refusal>,
    C::Outcome: OutcomeToast,
{
    let command = finish(form.command().ok()?);
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
            ledger
                .add_tag(&lib_tags::TagDraft {
                    name: self.0.clone(),
                    color: None,
                })
                .map_err(|_| format!("{} is taken", self.0))?;
            Ok(Added(ledger.tags().len()))
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

    fn tag_count(ledger: &Entity<LedgerStore>, cx: &gpui::TestAppContext) -> usize {
        ledger.read_with(cx, |store, _| store.ledger().tags().len())
    }

    #[gpui::test]
    fn confirm_ledger_leaves_an_invalid_form_open_and_runs_nothing(cx: &mut gpui::TestAppContext) {
        let ledger = crate::ledger_store::test_ledger(cx, lib_ledger::LedgerParts::default());
        let mut form = LedgerForm::default();

        let landed = cx.update(|cx| confirm_ledger(&mut form, &ledger, cx));

        assert!(landed.is_none());
        assert!(form.shown.is_none());
        assert_eq!(tag_count(&ledger, cx), 0);
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
        assert_eq!(tag_count(&ledger, cx), 1);
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
        assert_eq!(tag_count(&ledger, cx), 1);
    }
}
