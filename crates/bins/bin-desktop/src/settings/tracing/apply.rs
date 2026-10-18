//! The Tracing Dialog's confirm cycle (ADR-0037): Clear logs, which nothing can refuse.

use gpui::App;
use lib_toast::ToastKind;

use super::dialogs::TracingDialog;
use crate::chrome::{
    dialog_apply::{self, Apply, DialogForm, FormError},
    dialog_host::{ConfirmDialog, Dialog, DialogScope, Landed},
};

/// What confirming a Tracing Dialog asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TracingCommand {
    /// Clear logs: the capture lives on the page, which `Shell` empties.
    ClearLogs,
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
    fn confirm(&mut self, _scope: &DialogScope, cx: &mut App) -> Option<Landed<TracingOutcome>> {
        let outcome = dialog_apply::confirm(self, &mut TracingApply, cx)?;
        Some(Landed {
            toast: outcome.toast(),
            outcome,
        })
    }
}

/// Runs [`TracingCommand`]s.
pub struct TracingApply;

impl Apply<TracingCommand> for TracingApply {
    type Outcome = TracingOutcome;
    type Refusal = TracingRefusal;

    fn apply(
        &mut self,
        command: TracingCommand,
        _cx: &mut App,
    ) -> Result<TracingOutcome, TracingRefusal> {
        match command {
            TracingCommand::ClearLogs => Ok(TracingOutcome::LogsConfirmed),
        }
    }
}

#[cfg(test)]
mod tests {
    use gpui::TestAppContext;

    use super::*;
    use crate::chrome::dialog_apply::confirm;

    #[gpui::test]
    fn clear_logs_only_confirms(cx: &mut TestAppContext) {
        let mut dialog = TracingDialog::ClearLogs;
        let outcome = cx.update(|cx| confirm(&mut dialog, &mut TracingApply, cx));
        assert_eq!(outcome, Some(TracingOutcome::LogsConfirmed));
    }

    #[test]
    fn a_landed_clear_logs_raises_an_info_toast() {
        let toast = TracingOutcome::LogsConfirmed.toast();
        assert_eq!(toast.map(|(kind, _)| kind), Some(ToastKind::Info));
    }
}
