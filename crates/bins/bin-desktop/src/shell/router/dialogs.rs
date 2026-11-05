//! The Dialog host plumbing of `Root`: opening and closing a module's Dialog Entity. An `impl Root`
//! block, not the Dialog host itself (`shell::view::dialog_entity`).

use super::Root;
use crate::navigation::nav::NounLabel;
use crate::shell::view::dialog_entity::DialogReadback;
use crate::view::help::{HelpDialog, HelpGroup};
use gpui::{AppContext as _, Context};
use lib_shell::DialogInput;
use lib_shell::HelpPresenter;

use crate::shell::view::dialog_entity::DialogHandle;

impl Root {
    /// Opens a module's Dialog Entity, closing the host once its Presenter closes it.
    pub(super) fn host_dialog(&mut self, dialog: DialogHandle, cx: &mut Context<'_, Self>) {
        self.close_dialog();
        let shell = cx.weak_entity();
        let id = dialog.id();
        let closed = dialog.on_closed(
            move |cx| {
                // A Shell gone with its window has no Dialog left to close.
                let _ = shell.update(cx, |shell, cx| {
                    if shell.hosted_dialog().is_some_and(|open| open.id() == id) {
                        shell.close_dialog();
                        cx.notify();
                    }
                });
            },
            cx,
        );
        self.state.hosted = Some((dialog, closed));
        self.presenter.dialog(DialogInput::Opened);
    }

    /// The open module Dialog, if one is.
    pub(crate) fn hosted_dialog(&self) -> Option<&DialogHandle> {
        self.state.hosted.as_ref().map(|(dialog, _)| dialog)
    }

    /// The open hosted Dialog's state as labelled values, for a test; `None` when no Dialog or
    /// one not yet hosted is open.
    #[doc(hidden)]
    pub fn dialog_readback(&self, cx: &gpui::App) -> Option<DialogReadback> {
        Some(self.hosted_dialog()?.readback(cx))
    }

    /// Opens the Help Dialog over the active View, carrying the page group that View adds to the
    /// global shortcuts.
    pub(super) fn open_help(&mut self, cx: &mut Context<'_, Self>) {
        let group = self.active_view_entity(cx).help_group(cx);
        let presenter = HelpPresenter::open(self.presenter.keymap(), NounLabel::label, group);
        let dialog = cx.new(|_| HelpDialog::new(presenter));
        self.host_dialog(DialogHandle::new(dialog), cx);
    }

    /// Whether the Help Dialog is the open Dialog.
    #[doc(hidden)]
    pub fn help_open(&self) -> bool {
        self.help_dialog().is_some()
    }

    fn help_dialog(&self) -> Option<gpui::Entity<HelpDialog>> {
        self.hosted_dialog()?.downcast::<HelpDialog>()
    }

    /// The open Help Dialog's page group, for a test; `None` while it is not open or carries
    /// none.
    #[doc(hidden)]
    pub fn help_group(&self, cx: &gpui::App) -> Option<HelpGroup> {
        self.help_dialog()?.read(cx).group()
    }

    /// Closes the open Dialog without applying it.
    pub(super) fn close_dialog(&mut self) {
        if self.state.hosted.take().is_some() {
            self.presenter.dialog(DialogInput::Closed);
        }
    }
}
