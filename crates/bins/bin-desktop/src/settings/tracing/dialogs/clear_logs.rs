//! The Tracing page's **Clear logs** confirm (issue #502), on the shared `crate::dialog` frame's
//! destructive variant, an Entity in the Dialog host (`shell::view::dialog_entity`). Nothing to
//! type: `enter` or the button clears, `esc` or Cancel keeps. Every Dialog command reaches
//! `lib-settings`' `ClearLogsPresenter` as a `DialogInput`; this draws its view model, hands a
//! confirm back to the page and tells the host to close.

use gpui::{
    AnyElement, App, Context, EventEmitter, Render, WeakEntity, Window, div, prelude::*, px,
};
use lib_settings::{ClearLogsOutcome, ClearLogsPresenter, ClearLogsViewModel};
use lib_shell::{DialogCommand, Hint};
use lib_viper::dialog::DialogInput;
use lib_viper::keymap::{BACK, Keymap, SELECT};

use crate::dialog;
use crate::settings::tracing::TracingPage;
use crate::shell::view::dialog_entity::DialogReadback;
use crate::shell::view::dialog_entity::{DialogClicks, DialogClosed, HostRouter, HostedDialog};

/// The open Clear logs Dialog: its Presenter and the page its confirm lands on.
pub struct ClearLogsDialog {
    presenter: ClearLogsPresenter,
    page: WeakEntity<TracingPage>,
}

impl ClearLogsDialog {
    pub fn new(presenter: ClearLogsPresenter, page: WeakEntity<TracingPage>) -> Self {
        Self { presenter, page }
    }

    /// Hands a confirm to the page and tells the host, once the Presenter has closed the Dialog.
    fn closed(&self, router: HostRouter<ClearLogsOutcome>, cx: &mut Context<'_, Self>) {
        let Some(outcome) = router.closed() else {
            return;
        };
        if outcome.is_some() {
            // A page that has gone has nothing to land on.
            let _ = self.page.update(cx, |page, cx| page.cleared(cx));
        }
        cx.emit(DialogClosed);
    }
}

impl EventEmitter<DialogClosed> for ClearLogsDialog {}

impl HostedDialog for ClearLogsDialog {
    fn input(&mut self, input: DialogInput, cx: &mut Context<'_, Self>) {
        let mut router = HostRouter::default();
        self.presenter.input(input, &mut router);
        self.closed(router, cx);
    }

    fn can_confirm(&self) -> bool {
        true
    }

    fn confirm(&mut self, cx: &mut Context<'_, Self>) {
        let mut router = HostRouter::default();
        self.presenter.confirm(&mut router);
        self.closed(router, cx);
    }

    fn cancel(&mut self, cx: &mut Context<'_, Self>) {
        let mut router = HostRouter::default();
        self.presenter.cancel(&mut router);
        self.closed(router, cx);
    }

    fn close_open_select(&mut self) -> bool {
        false
    }

    /// A plain confirm has at most the one confirm field, so no `tab`.
    fn hints(&self, keymap: &Keymap<DialogCommand>) -> Option<Vec<Hint>> {
        let key = |command| keymap.chord(command).unwrap_or_default().to_string();
        Some(vec![
            Hint::new(key(BACK), crate::msg::desktop_hint_cancel()),
            Hint::new(key(SELECT), crate::msg::desktop_hint_confirm()),
        ])
    }

    /// Clear logs shows no field, so its readback is the kind alone.
    fn readback(&self, _cx: &App) -> DialogReadback {
        DialogReadback {
            kind: "ClearLogs",
            values: Vec::new(),
        }
    }
}

impl Render for ClearLogsDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let clicks = DialogClicks::new(cx);
        render(&self.presenter.view_model(), &clicks, cx)
    }
}

fn render(
    view: &ClearLogsViewModel,
    clicks: &DialogClicks<ClearLogsDialog>,
    cx: &App,
) -> AnyElement {
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(view.title.clone(), true, cx))
        .child(dialog::body([div()
            .text_size(px(13.0))
            .child(view.warning.clone())
            .into_any_element()]))
        .child(dialog::action_row(
            [
                dialog::cancel_button("clear-logs-cancel", clicks.cancel.clone(), cx)
                    .into_any_element(),
                dialog::confirm_button(
                    "clear-logs-confirm",
                    view.confirm.clone(),
                    true,
                    true,
                    clicks.confirm.clone(),
                    cx,
                )
                .into_any_element(),
            ],
            cx,
        ));

    dialog::overlay(dialog::WIDTH, true, card, cx)
}
