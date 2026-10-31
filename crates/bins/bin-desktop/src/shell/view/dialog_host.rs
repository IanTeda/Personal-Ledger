//! The legacy Dialog host: the slot the Dialogs not yet on a module's Presenter live in while
//! open, and the keyboard handling they share. A module's Dialog (Units, Bills, Pay, Payees) is an
//! Entity in `shell::view::dialog_entity` instead; these six move there with their slices
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
//! A hosted Dialog (ADR-0037's 2026-10-18 amendment) also draws itself ([`Dialog::render`]),
//! describes itself for the UI tests ([`Dialog::readback`]) and confirms itself
//! ([`ConfirmDialog::confirm`]), so it lives in its page's folder and needs nothing of `Root` but
//! the host. Its page opens it with [`open_dialog`], naming the opener its outcome goes back to.
//! Its clicks edit it through [`DialogActions::edit`], the one callback `Root` supplies. The
//! Dialogs not moved yet return `None` from both and `Root` still draws and confirms them by
//! variant.

use std::{fmt, rc::Rc};

use chrono::NaiveDate;
use gpui::{AnyElement, App, Context, Entity, EventEmitter, WeakEntity};
use lib_toast::ToastKind;

use crate::{
    budgets::BudgetsDialog, dialog::OnClick, documents::form::DocumentsDialog,
    form::field::TextField, ledger_store::LedgerStore,
    settings::documents::dialogs::DocumentTypesDialog, settings::tracing::dialogs::TracingDialog,
    view::event::ViewEvent, view::help::HelpDialog,
};

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
#[derive(Clone)]
pub struct DialogScope {
    pub ledger: Entity<LedgerStore>,
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

/// A confirm that landed: the Toast the host raises, if any, and what the opener is handed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Landed<O> {
    pub toast: Option<DialogToast>,
    pub outcome: O,
}

/// A hosted Dialog's confirm: its form's command runs through the Ledger store
/// (`dialog_apply::confirm_ledger`), or on its own when it needs no Ledger.
pub trait ConfirmDialog: Dialog + DialogVariant + 'static {
    type Outcome: 'static;

    /// `Some` when the command landed and the Dialog should close, `None` while the form is not
    /// valid or the domain refused it (the refusal is already on the form).
    fn confirm(&mut self, scope: &DialogScope, cx: &mut App) -> Option<Landed<Self::Outcome>>;
}

/// The open Dialog's confirm with its opener bound: a landed one has handed its outcome back and
/// leaves the host only its Toast.
pub type HostConfirm = Rc<dyn Fn(&mut OpenDialog, &DialogScope, &mut App) -> Option<Landed<()>>>;

/// A hosted Dialog a page asked the host to open, carrying the confirm that hands its outcome
/// back to the page.
#[derive(Clone)]
pub struct HostedOpen {
    pub dialog: OpenDialog,
    pub confirm: HostConfirm,
}

impl fmt::Debug for HostedOpen {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HostedOpen")
            .field("dialog", &self.dialog)
            .finish_non_exhaustive()
    }
}

/// Two opens are the same when they open the same Dialog with the same opener.
impl PartialEq for HostedOpen {
    fn eq(&self, other: &Self) -> bool {
        self.dialog == other.dialog && Rc::ptr_eq(&self.confirm, &other.confirm)
    }
}

impl Eq for HostedOpen {}

impl HostedOpen {
    /// `dialog`, opened by `page`: after a landed confirm, `opener` runs on the page with the
    /// outcome. A page that has gone is skipped, and the Toast is still raised.
    pub fn new<D, P>(
        dialog: D,
        page: WeakEntity<P>,
        opener: impl Fn(&mut P, D::Outcome, &mut Context<'_, P>) + 'static,
    ) -> Self
    where
        D: ConfirmDialog,
        P: 'static,
    {
        let confirm: HostConfirm = Rc::new(move |open, scope, cx| {
            let Landed { toast, outcome } = D::from_open_mut(open)?.confirm(scope, cx)?;
            // A page that has gone has nothing to land on.
            let _ = page.update(cx, |page, cx| opener(page, outcome, cx));
            Some(Landed { toast, outcome: () })
        });
        Self {
            dialog: dialog.into_open(),
            confirm,
        }
    }
}

/// Asks the host to open `dialog` over page `P`, which `opener` hands the outcome back to.
pub fn open_dialog<D, P>(
    dialog: D,
    opener: impl Fn(&mut P, D::Outcome, &mut Context<'_, P>) + 'static,
    cx: &mut Context<'_, P>,
) where
    D: ConfirmDialog,
    P: EventEmitter<ViewEvent> + 'static,
{
    let open = HostedOpen::new(dialog, cx.weak_entity(), opener);
    cx.emit(ViewEvent::OpenHostedDialog(Box::new(open)));
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

/// The open legacy Dialog, one variant per feature: the six not yet on a module's Presenter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenDialog {
    Tracing(TracingDialog),
    DocumentTypes(DocumentTypesDialog),
    /// Boxed: its forms carry their select choices.
    Budgets(Box<BudgetsDialog>),
    Documents(DocumentsDialog),
    ToastHistory(ToastHistoryDialog),
    Help(HelpDialog),
}

impl OpenDialog {
    fn inner(&self) -> &dyn Dialog {
        match self {
            Self::Tracing(dialog) => dialog,
            Self::DocumentTypes(dialog) => dialog,
            Self::Budgets(dialog) => &**dialog,
            Self::Documents(dialog) => dialog,
            Self::ToastHistory(dialog) => dialog,
            Self::Help(dialog) => dialog,
        }
    }

    fn inner_mut(&mut self) -> &mut dyn Dialog {
        match self {
            Self::Tracing(dialog) => dialog,
            Self::DocumentTypes(dialog) => dialog,
            Self::Budgets(dialog) => &mut **dialog,
            Self::Documents(dialog) => dialog,
            Self::ToastHistory(dialog) => dialog,
            Self::Help(dialog) => dialog,
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

/// Generates the [`DialogVariant`] impl for the `OpenDialog` variant that holds `$ty`; a boxed
/// variant passes the inner type and the box derefs.
macro_rules! dialog_variant {
    ($variant:ident, $ty:ty) => {
        impl DialogVariant for $ty {
            fn from_open(open: &OpenDialog) -> Option<&Self> {
                match open {
                    OpenDialog::$variant(dialog) => Some(dialog),
                    _ => None,
                }
            }

            fn from_open_mut(open: &mut OpenDialog) -> Option<&mut Self> {
                match open {
                    OpenDialog::$variant(dialog) => Some(dialog),
                    _ => None,
                }
            }

            fn into_open(self) -> OpenDialog {
                OpenDialog::$variant(self.into())
            }
        }
    };
}

dialog_variant!(Tracing, TracingDialog);
dialog_variant!(DocumentTypes, DocumentTypesDialog);
dialog_variant!(Budgets, BudgetsDialog);
dialog_variant!(Documents, DocumentsDialog);
dialog_variant!(Help, HelpDialog);

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
    fn a_dialog_without_text_fields_ignores_typing_but_swallows_tab() {
        let mut dialog = OpenDialog::Tracing(TracingDialog::ClearLogs);
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Char('x')),
            DialogOutcome::Ignored
        );
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Backspace),
            DialogOutcome::Ignored
        );
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Tab),
            DialogOutcome::Handled
        );
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Confirm
        );
    }

    #[test]
    fn other_keys_fall_through() {
        let mut dialog = OpenDialog::Tracing(TracingDialog::ClearLogs);
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Other),
            DialogOutcome::Ignored
        );
    }

    #[test]
    fn dialog_variant_finds_only_its_own_features_dialog() {
        let mut dialog = OpenDialog::Tracing(TracingDialog::ClearLogs);
        assert!(TracingDialog::from_open(&dialog).is_some());
        assert!(TracingDialog::from_open_mut(&mut dialog).is_some());
        assert!(DocumentTypesDialog::from_open(&dialog).is_none());
        assert!(DocumentTypesDialog::from_open_mut(&mut dialog).is_none());
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

    #[test]
    fn help_closes_on_question_mark_or_enter_and_ignores_the_rest() {
        let mut dialog = OpenDialog::Help(HelpDialog { group: None });
        for key in [DialogKey::Char('?'), DialogKey::Enter] {
            assert_eq!(handle_key(&mut dialog, key), DialogOutcome::Confirm);
        }
        for key in [DialogKey::Char('j'), DialogKey::Tab, DialogKey::Down] {
            assert_eq!(handle_key(&mut dialog, key), DialogOutcome::Ignored);
        }
        assert!(!dialog.close_open_select(), "the first Esc closes it");
    }
}
