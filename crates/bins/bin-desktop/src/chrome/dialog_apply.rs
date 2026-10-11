//! The Dialog host's confirm cycle (ADR-0037): a form turns itself into a command, an [`Apply`]
//! runs it, and the Dialog closes only when it landed.
//!
//! A form answers [`DialogForm::command`] with the command its fields describe, or a
//! [`FormError`] when a field is not valid yet, and takes a domain refusal back through
//! [`DialogForm::refuse`] so it can show it as a Message; the domain libs stay locale-free.
//! [`Apply`] is implemented against the per-domain stores today and `LedgerStore` later, so the
//! forms and [`confirm`] don't change when the data moves.

use gpui::App;

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
}
