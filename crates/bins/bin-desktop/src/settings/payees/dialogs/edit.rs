//! Renders the **Edit payee** dialog (`docs/ux/desktop-mockups/20-payees/README.md`'s 6c). It is
//! the Add dialog's form in Edit mode, so it draws through the shared renderer in `add.rs`.

use gpui::{AnyElement, App};

use super::{PayeeDialogHandlers, add};
use crate::settings::payees::form::PayeeForm;

/// Draws the open Edit dialog on the Payee stored as `name`, which `splits` Splits carry.
pub fn render(
    name: &str,
    splits: usize,
    form: &PayeeForm,
    handlers: PayeeDialogHandlers,
    cx: &App,
) -> AnyElement {
    add::render(
        add::PayeeDialogMode::Edit { name, splits },
        form,
        form.options(),
        form.name_error(),
        form.is_valid(),
        handlers,
        cx,
    )
}
