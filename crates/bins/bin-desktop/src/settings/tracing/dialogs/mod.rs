//! The Tracing page's Dialogs: just the Clear logs confirm. [`TracingDialog`] is the value
//! `Root`'s Dialog host holds while it is open (`OpenDialog::Tracing`). It draws itself
//! (`Dialog::render`), reads itself back for the UI tests and confirms itself through `apply.rs`.

pub mod clear_logs;

use gpui::{AnyElement, App};

use crate::{
    form::field::TextField,
    settings::hints,
    shell::view::dialog_host::{Dialog, DialogActions, DialogReadback, DialogScope},
};

/// The open Tracing Dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TracingDialog {
    /// Clear logs confirm: nothing to fill in.
    ClearLogs,
}

impl Dialog for TracingDialog {
    fn focused_text(&mut self) -> Option<&mut TextField> {
        None
    }

    fn is_valid(&self) -> bool {
        true
    }

    /// Clear logs is a confirm.
    fn hints(&self) -> Option<Vec<lib_shell::Hint>> {
        Some(crate::shell::view::statusline::owned_hints(
            hints::confirm_dialog_hints(),
        ))
    }

    fn render(
        &self,
        _scope: &DialogScope,
        actions: &DialogActions,
        cx: &App,
    ) -> Option<AnyElement> {
        match self {
            Self::ClearLogs => Some(clear_logs::render(
                actions.cancel.clone(),
                actions.confirm.clone(),
                cx,
            )),
        }
    }

    /// Clear logs shows no field, so its readback is the kind alone.
    fn readback(&self, _scope: &DialogScope, _cx: &App) -> Option<DialogReadback> {
        let kind = match self {
            Self::ClearLogs => "ClearLogs",
        };
        Some(DialogReadback {
            kind,
            values: Vec::new(),
        })
    }
}
