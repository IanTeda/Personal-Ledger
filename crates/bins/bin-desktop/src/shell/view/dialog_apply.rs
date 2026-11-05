//! The Dialog host's confirm cycle (ADR-0037): a form turns itself into a command, the command
//! runs, and the Dialog closes only when it landed.
//!
//! A form answers [`DialogForm::command`] with the command its fields describe, or a
//! [`FormError`] when a field is not valid yet, and takes a domain refusal back through
//! [`DialogForm::refuse`] so it can show it as a Message; the domain libs stay locale-free. Each
//! Dialog left on the host, such as Tracing's, runs its command in its own `ConfirmDialog`.

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
