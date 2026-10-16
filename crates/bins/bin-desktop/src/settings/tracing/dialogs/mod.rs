//! The Tracing page's Dialogs: just the Clear logs confirm. [`TracingDialog`] is the value
//! `Shell`'s Dialog host holds while it is open (`OpenDialog::Tracing`).

pub mod clear_logs;

use gpui::{AnyElement, App};

use crate::{chrome::dialog_host::Dialog, dialog, form::field::TextField, settings::hints};

/// The open Tracing Dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TracingDialog {
    /// Clear logs confirm: nothing to fill in.
    ClearLogs,
}

impl TracingDialog {
    /// Draws the open Dialog.
    pub fn render(
        &self,
        on_cancel: dialog::OnClick,
        on_confirm: dialog::OnClick,
        cx: &App,
    ) -> AnyElement {
        match self {
            Self::ClearLogs => clear_logs::render(on_cancel, on_confirm, cx),
        }
    }
}

impl Dialog for TracingDialog {
    fn focused_text(&mut self) -> Option<&mut TextField> {
        None
    }

    fn is_valid(&self) -> bool {
        true
    }

    /// Clear logs is a confirm.
    fn hints(&self) -> Option<Vec<(&'static str, String)>> {
        Some(hints::confirm_dialog_hints())
    }
}
