//! The confirm cycle (ADR-0037) of the one Settings Dialog no page owns: Clear logs.

use gpui::App;

use super::SettingsDialog;
use crate::chrome::{
    dialog_apply::{Apply, DialogForm, FormError},
    dialog_host::Dialog,
};

/// What confirming a Settings Dialog asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsCommand {
    /// Clear logs: the capture lives in the Settings View, which `Shell` empties.
    ClearLogs,
}

/// What a landed [`SettingsCommand`] did, for what follows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsOutcome {
    LogsConfirmed,
}

/// Why a Settings command didn't land: none of them can be refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsRefusal {}

impl DialogForm for SettingsDialog {
    type Command = SettingsCommand;
    type Refusal = SettingsRefusal;

    fn command(&self) -> Result<SettingsCommand, FormError> {
        if !Dialog::is_valid(self) {
            return Err(FormError);
        }
        Ok(match self {
            Self::ClearLogs => SettingsCommand::ClearLogs,
        })
    }

    fn refuse(&mut self, refusal: SettingsRefusal) {
        match refusal {}
    }
}

/// Runs [`SettingsCommand`]s.
pub struct SettingsApply;

impl Apply<SettingsCommand> for SettingsApply {
    type Outcome = SettingsOutcome;
    type Refusal = SettingsRefusal;

    fn apply(
        &mut self,
        command: SettingsCommand,
        _cx: &mut App,
    ) -> Result<SettingsOutcome, SettingsRefusal> {
        match command {
            SettingsCommand::ClearLogs => Ok(SettingsOutcome::LogsConfirmed),
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
        let mut dialog = SettingsDialog::ClearLogs;
        let outcome = cx.update(|cx| confirm(&mut dialog, &mut SettingsApply, cx));
        assert_eq!(outcome, Some(SettingsOutcome::LogsConfirmed));
    }
}
