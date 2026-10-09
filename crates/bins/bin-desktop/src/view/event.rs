//! `ViewEvent` -- the requests a View makes of `Shell`. A View emits one of these with
//! `cx.emit` and never holds a handle to `Shell` (ADR-0032): `Shell` subscribes to each View
//! and handles every event in one place (`shell/view_events.rs`).

use lib_toast::ToastKind;

use crate::{chrome::dialog_host::OpenDialog, navigation::nav::Noun};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewEvent {
    /// Raise a Toast whose Message the View has already resolved to text.
    RaiseToast { kind: ToastKind, text: String },
    /// Open a Dialog in the Dialog host.
    OpenDialog(OpenDialog),
    /// Make `Noun` the active destination, as the rail or a `g`-jump would.
    Navigate(Noun),
    /// Replace the status message shown in the Chrome.
    SetStatus(String),
}
