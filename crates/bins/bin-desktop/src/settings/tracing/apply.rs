//! The Tracing Dialog's confirm cycle (ADR-0037): Clear logs, which nothing can refuse and which
//! runs without the Ledger.

use gpui::App;
use lib_toast::ToastKind;

use super::dialogs::TracingDialog;
use crate::shell::view::{
    dialog_apply::{DialogForm, FormError},
    dialog_host::{ConfirmDialog, Dialog, DialogScope, Landed},
};

/// What confirming a Tracing Dialog asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TracingCommand {
    /// Clear logs: the capture lives on the page, which `Root` empties.
    ClearLogs,
}

impl TracingCommand {
    /// Clear logs lands whatever the page holds, so nothing here can refuse.
    pub fn run(self) -> TracingOutcome {
        match self {
            Self::ClearLogs => TracingOutcome::LogsConfirmed,
        }
    }
}

/// What a landed [`TracingCommand`] did, for what follows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TracingOutcome {
    LogsConfirmed,
}

impl TracingOutcome {
    /// The Toast a landed Clear logs raises.
    pub fn toast(&self) -> Option<(ToastKind, String)> {
        match self {
            Self::LogsConfirmed => Some((
                ToastKind::Info,
                crate::msg::desktop_settings_tracing_toast_cleared(),
            )),
        }
    }
}

/// Why a Tracing command didn't land: none of them can be refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TracingRefusal {}

impl DialogForm for TracingDialog {
    type Command = TracingCommand;
    type Refusal = TracingRefusal;

    fn command(&self) -> Result<TracingCommand, FormError> {
        if !Dialog::is_valid(self) {
            return Err(FormError);
        }
        Ok(match self {
            Self::ClearLogs => TracingCommand::ClearLogs,
        })
    }

    fn refuse(&mut self, refusal: TracingRefusal) {
        match refusal {}
    }
}

impl ConfirmDialog for TracingDialog {
    type Outcome = TracingOutcome;

    /// A landed Clear logs raises its Toast and hands the outcome to the page.
    fn confirm(&mut self, _scope: &DialogScope, _cx: &mut App) -> Option<Landed<TracingOutcome>> {
        let outcome = self.command().ok()?.run();
        Some(Landed {
            toast: outcome.toast(),
            outcome,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clear_logs_only_confirms() {
        let command = TracingDialog::ClearLogs.command();
        assert_eq!(command, Ok(TracingCommand::ClearLogs));
        assert_eq!(
            TracingCommand::ClearLogs.run(),
            TracingOutcome::LogsConfirmed
        );
    }

    #[test]
    fn a_landed_clear_logs_raises_an_info_toast() {
        let toast = TracingOutcome::LogsConfirmed.toast();
        assert_eq!(toast.map(|(kind, _)| kind), Some(ToastKind::Info));
    }
}
