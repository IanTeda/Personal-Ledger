//! The legacy Dialog host: the slot the Dialogs not yet on a module's Presenter live in while
//! open, and the keyboard handling they share. A module's Dialog is an Entity in
//! `shell::view::dialog_entity` instead (#991); the Dialogs here move there with their slices
//! (#820). The key handling needs no window, so a Dialog's keys are unit-tested without one.
//!
//! `Root` holds `Option<OpenDialog>` and opens and closes it only through `Root::open_dialog`
//! and `Root::close_dialog`, which also enter and leave `InputMode::Dialog`, so the slot and
//! the mode can't disagree. A key reaches the open Dialog through [`handle_key`]: the Dialog's
//! own [`Dialog::handle_own_key`] sees it first (a list Dialog's `j`/`k`, a search box), then
//! the shared handling types into the focused [`TextField`], cycles fields on `Tab` and confirms
//! on `Enter` only while the form is valid. A Dialog never touches ledger data: it answers
//! [`DialogOutcome::Confirm`] and `Root` applies the form (`Root::confirm_open_dialog`), the
//! same path the confirm button's click takes.
//!
//! `Esc` is not a key here: step 1 of the key route (`crate::input`) asks
//! [`Dialog::close_open_select`] first, closing the Dialog only when no select was open.
//!
//! The Dialog draws itself ([`Dialog::render`]) and describes itself for the UI tests
//! ([`Dialog::readback`]); its clicks edit it through [`DialogActions::edit`], the one callback
//! `Root` supplies. Only the Toast history is left here (#1043 moves it), so the host confirms
//! nothing: it is read-only.

use std::rc::Rc;

use chrono::NaiveDate;
use gpui::{AnyElement, App, Entity};
use lib_toast::ToastKind;

use crate::{dialog::OnClick, form::field::TextField, ledger_store::LedgerStore};

/// A keystroke as a Dialog sees it, already stripped of modifiers by `Root`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogKey {
    Backspace,
    Tab,
    /// `Shift-Tab`; a Dialog without its own handling treats it as `Tab`.
    BackTab,
    Up,
    Down,
    Left,
    Right,
    Enter,
    /// `Ctrl-Enter`: a multi-line box takes it to save, and any other Dialog treats it as `Enter`.
    CtrlEnter,
    /// A printable character, typed without Ctrl/Alt/Cmd/Fn.
    Char(char),
    /// A character chord held with Ctrl alone (`Ctrl-N`), for a list Dialog's own keys.
    Ctrl(char),
    /// Anything else (arrows, function keys, a modified chord).
    Other,
}

/// What `Root` does after a Dialog has seen a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogOutcome {
    /// Not the Dialog's key; `Root` lets it fall through.
    Ignored,
    Handled,
    /// The form is valid and the user confirmed: `Root` applies it and closes the Dialog.
    Confirm,
}

/// One feature's Dialogs. Every method has the default a plain confirm (no fields) wants.
pub trait Dialog {
    /// Keys this Dialog takes before the shared handling, `None` to leave the key to it.
    fn handle_own_key(&mut self, _key: DialogKey) -> Option<DialogOutcome> {
        None
    }

    /// The text field that typing and `Backspace` edit.
    fn focused_text(&mut self) -> Option<&mut TextField> {
        None
    }

    /// `Tab`: the next field in this Dialog's own focus ring. Never reaches the shell-wide zones.
    fn cycle_field(&mut self) {}

    /// Whether `Enter` (and the confirm button) may confirm.
    fn is_valid(&self) -> bool {
        true
    }

    /// The first `Esc` closes an open select only. `true` when one was open.
    fn close_open_select(&mut self) -> bool {
        false
    }

    /// The status line's hints while this Dialog is open, in place of the View's (ADR-0037).
    /// `None` leaves them to the View, as for a Dialog whose View still words its hints itself.
    fn hints(&self) -> Option<Vec<lib_shell::Hint>> {
        None
    }

    /// Draws a hosted Dialog, its clicks reaching it through `actions`. `None` for a Dialog
    /// `Root` still draws by variant.
    fn render(
        &self,
        _scope: &DialogScope,
        _actions: &DialogActions,
        _cx: &App,
    ) -> Option<AnyElement> {
        None
    }

    /// A hosted Dialog's state as the UI tests read it back. `None` for a Dialog whose page still
    /// reports it through its snapshot.
    fn readback(&self, _scope: &DialogScope, _cx: &App) -> Option<DialogReadback> {
        None
    }
}

/// A Toast the host raises once a Dialog has acted: its kind and its Message, already text.
pub type DialogToast = (ToastKind, String);

/// What a hosted Dialog reads and writes through: the Ledger, and the day the Shell was built on.
/// The Dialogs left in this host (the Toast history) open with no Ledger open too,
/// so the Ledger is `None` then.
#[derive(Clone)]
pub struct DialogScope {
    pub ledger: Option<Entity<LedgerStore>>,
    pub today: NaiveDate,
}

/// The one "edit the open Dialog" callback `Root` supplies: it runs the change on the open
/// Dialog, raises the Toast the change returns and repaints.
pub type OnEdit =
    Rc<dyn Fn(&mut App, &mut dyn FnMut(&mut OpenDialog, &mut App) -> Option<DialogToast>)>;

/// The clicks a drawing Dialog binds: edits to itself, and the host's cancel and confirm.
#[derive(Clone)]
pub struct DialogActions {
    pub edit: OnEdit,
    pub cancel: OnClick,
    pub confirm: OnClick,
}

impl DialogActions {
    /// Runs `change` on the open `D`; a no-op when another feature's Dialog is open.
    pub fn edit<D: DialogVariant>(
        &self,
        cx: &mut App,
        change: impl FnOnce(&mut D, &mut App) -> Option<DialogToast>,
    ) {
        let mut change = Some(change);
        (self.edit)(cx, &mut |open, cx| {
            let dialog = D::from_open_mut(open)?;
            change.take()?(dialog, cx)
        });
    }
}

/// A hosted Dialog's state as labelled values, read back by the UI tests: `kind` names which
/// Dialog is open (`AddTag`), and each value a field it shows (`name`, `confirm`).
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialogReadback {
    pub kind: &'static str,
    pub values: Vec<(&'static str, String)>,
}

impl DialogReadback {
    /// The value labelled `label`, if the Dialog shows one.
    pub fn value(&self, label: &str) -> Option<&str> {
        self.values
            .iter()
            .find(|(name, _)| *name == label)
            .map(|(_, value)| value.as_str())
    }
}

/// The session Toast history: read-only, so it takes no key but `Esc`, which the key route turns
/// into closing it. Showing Toasts hide behind it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToastHistoryDialog;

impl Dialog for ToastHistoryDialog {
    /// The list scrolls by pointer, so every key is left to the shell.
    fn handle_own_key(&mut self, _key: DialogKey) -> Option<DialogOutcome> {
        Some(DialogOutcome::Ignored)
    }
}

/// The open legacy Dialog, one variant per feature: the ones not yet on a module's Presenter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenDialog {
    ToastHistory(ToastHistoryDialog),
}

impl OpenDialog {
    fn inner(&self) -> &dyn Dialog {
        match self {
            Self::ToastHistory(dialog) => dialog,
        }
    }

    fn inner_mut(&mut self) -> &mut dyn Dialog {
        match self {
            Self::ToastHistory(dialog) => dialog,
        }
    }
}

/// A feature's Dialog as one [`OpenDialog`] variant holds it, so `Root::dialog_mut::<T>()` finds
/// the open Dialog by type instead of one accessor per feature.
pub trait DialogVariant: Sized {
    fn from_open(open: &OpenDialog) -> Option<&Self>;
    fn from_open_mut(open: &mut OpenDialog) -> Option<&mut Self>;
    fn into_open(self) -> OpenDialog;
}

impl DialogVariant for ToastHistoryDialog {
    fn from_open(open: &OpenDialog) -> Option<&Self> {
        let OpenDialog::ToastHistory(dialog) = open;
        Some(dialog)
    }

    fn from_open_mut(open: &mut OpenDialog) -> Option<&mut Self> {
        let OpenDialog::ToastHistory(dialog) = open;
        Some(dialog)
    }

    fn into_open(self) -> OpenDialog {
        OpenDialog::ToastHistory(self)
    }
}

impl Dialog for OpenDialog {
    fn handle_own_key(&mut self, key: DialogKey) -> Option<DialogOutcome> {
        self.inner_mut().handle_own_key(key)
    }

    fn focused_text(&mut self) -> Option<&mut TextField> {
        self.inner_mut().focused_text()
    }

    fn cycle_field(&mut self) {
        self.inner_mut().cycle_field();
    }

    fn is_valid(&self) -> bool {
        self.inner().is_valid()
    }

    fn close_open_select(&mut self) -> bool {
        self.inner_mut().close_open_select()
    }

    fn hints(&self) -> Option<Vec<lib_shell::Hint>> {
        self.inner().hints()
    }

    fn render(&self, scope: &DialogScope, actions: &DialogActions, cx: &App) -> Option<AnyElement> {
        self.inner().render(scope, actions, cx)
    }

    fn readback(&self, scope: &DialogScope, cx: &App) -> Option<DialogReadback> {
        self.inner().readback(scope, cx)
    }
}

/// Routes `key` to `dialog`: its own keys first, then typing, `Tab` and `Enter`.
pub fn handle_key(dialog: &mut impl Dialog, key: DialogKey) -> DialogOutcome {
    if let Some(outcome) = dialog.handle_own_key(key) {
        return outcome;
    }
    match key {
        DialogKey::Backspace => match dialog.focused_text() {
            Some(field) => {
                field.backspace();
                DialogOutcome::Handled
            }
            None => DialogOutcome::Ignored,
        },
        // Swallowed even with one field, so `Tab` never moves the shell's focus zones behind
        // the scrim.
        DialogKey::Tab | DialogKey::BackTab => {
            dialog.cycle_field();
            DialogOutcome::Handled
        }
        DialogKey::Enter | DialogKey::CtrlEnter if dialog.is_valid() => DialogOutcome::Confirm,
        DialogKey::Enter | DialogKey::CtrlEnter => DialogOutcome::Handled,
        DialogKey::Char(ch) => match dialog.focused_text() {
            Some(field) => {
                field.push(ch);
                DialogOutcome::Handled
            }
            None => DialogOutcome::Ignored,
        },
        DialogKey::Up
        | DialogKey::Down
        | DialogKey::Left
        | DialogKey::Right
        | DialogKey::Ctrl(_)
        | DialogKey::Other => DialogOutcome::Ignored,
    }
}

/// Types `text` into whichever field the Dialog has focused, for the form tests that sit beside
/// each form.
#[cfg(test)]
pub(crate) fn type_text(dialog: &mut impl Dialog, text: &str) {
    for ch in text.chars() {
        assert_eq!(
            handle_key(dialog, DialogKey::Char(ch)),
            DialogOutcome::Handled
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dialog_variant_finds_the_open_dialog() {
        let mut dialog = OpenDialog::ToastHistory(ToastHistoryDialog);
        assert!(ToastHistoryDialog::from_open(&dialog).is_some());
        assert!(ToastHistoryDialog::from_open_mut(&mut dialog).is_some());
    }

    #[test]
    fn toast_history_ignores_every_key_and_confirms_nothing_by_typing() {
        let mut dialog = OpenDialog::ToastHistory(ToastHistoryDialog);
        for key in [
            DialogKey::Char('x'),
            DialogKey::Backspace,
            DialogKey::Tab,
            DialogKey::BackTab,
            DialogKey::Enter,
            DialogKey::Down,
        ] {
            assert_eq!(handle_key(&mut dialog, key), DialogOutcome::Ignored);
        }
        assert!(!dialog.close_open_select(), "the first Esc closes it");
    }
}
