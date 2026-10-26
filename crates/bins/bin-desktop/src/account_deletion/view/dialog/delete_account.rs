//! The Delete account Dialog (`docs/ux/desktop-mockups/17-accounts/README.md`'s 3d) on the shared
//! `crate::dialog` frame's destructive variant, an Entity in the Dialog host
//! (`shell::view::dialog_entity`): what goes, why it can't go yet while a Bill Plan pays from the
//! Account, and the typed-back name. Every key and click becomes a `lib-account-deletion`
//! Presenter event, and this draws the view model and nothing else.
//!
//! Keys: typing edits the typed-back name, and `select` confirms once the Presenter would.
//! Confirm and Cancel go to the Presenter, which closes the Dialog through its Router; a confirmed
//! outcome is handed to the page, and the host closes on [`DialogClosed`].

use std::rc::Rc;

use gpui::{AnyElement, App, Context, Entity, EventEmitter, Render, Window, div, prelude::*, px};
use lib_account_deletion::AccountDeleted;
use lib_account_deletion::presenter::{
    DeleteAccountEvent, DeleteAccountPresenter, DeleteAccountViewModel,
};
use lib_shell::DialogCommand;
use lib_viper::keymap::{BACK, Keymap, SELECT};

use crate::account_deletion::router::DialogRouter;
use crate::bills::DesktopPorts;
use crate::dialog;
use crate::form::dialog_fields::text_field;
use crate::form::viper::{enter, typed};
use crate::ledger_store::LedgerStore;
use crate::shell::view::dialog_entity::{DialogClicks, DialogClosed, HostedDialog};
use crate::shell::view::dialog_host::{DialogKey, DialogOutcome, DialogReadback};
use crate::shell::view::statusline::Hint;
use crate::theme::color;

/// The Dialog's width: the Accounts dialogs' 440px.
const WIDTH: gpui::Pixels = px(440.0);

/// The open Delete Dialog's clicks.
type Clicks = DialogClicks<DeleteAccountDialog>;

/// What a landed delete hands the page that opened it.
pub(crate) type Landed = Box<dyn Fn(AccountDeleted, &mut App)>;

/// The open Delete account Dialog: its Presenter, the Bills ports and Ledger read copy its confirm
/// brings in line, and the page's landing.
pub struct DeleteAccountDialog {
    presenter: DeleteAccountPresenter,
    bills: DesktopPorts,
    ledger: Entity<LedgerStore>,
    landed: Landed,
}

impl DeleteAccountDialog {
    pub(crate) fn new(
        presenter: DeleteAccountPresenter,
        bills: &DesktopPorts,
        ledger: &Entity<LedgerStore>,
        landed: Landed,
    ) -> Self {
        Self {
            presenter,
            bills: bills.clone(),
            ledger: ledger.clone(),
            landed,
        }
    }

    fn view(&self) -> DeleteAccountViewModel {
        self.presenter.view_model()
    }

    /// Sends `event` to the Presenter, returning what its Router was asked.
    fn send(&mut self, event: DeleteAccountEvent) -> DialogRouter {
        let mut router = DialogRouter::default();
        self.presenter.handle(event, &mut router);
        router
    }

    /// Hands a landed delete to the page and tells the host, once the Presenter has closed the
    /// Dialog.
    fn closed(&self, router: DialogRouter, cx: &mut Context<'_, Self>) {
        let Some(outcome) = router.closed else {
            return;
        };
        if let Some(outcome) = outcome {
            (self.landed)(outcome, cx);
        }
        cx.emit(DialogClosed);
    }
}

impl EventEmitter<DialogClosed> for DeleteAccountDialog {}

impl HostedDialog for DeleteAccountDialog {
    /// Every key but `select` types into the typed-back name.
    fn key(&mut self, key: DialogKey, _cx: &mut Context<'_, Self>) -> DialogOutcome {
        let view = self.view();
        match key {
            DialogKey::Enter => enter(view.frame.can_confirm),
            DialogKey::Tab | DialogKey::BackTab => DialogOutcome::Handled,
            _ => match typed(&view.confirm.text, key) {
                Some(text) => {
                    self.send(DeleteAccountEvent::ConfirmText(text));
                    DialogOutcome::Handled
                }
                None => DialogOutcome::Ignored,
            },
        }
    }

    fn can_confirm(&self) -> bool {
        self.view().frame.can_confirm
    }

    /// Sends Confirm to the Presenter, which deletes, then brings the Ledger's read copies in
    /// line. A refusal stays on the view model and keeps the Dialog open.
    fn confirm(&mut self, cx: &mut Context<'_, Self>) {
        let router = self.send(DeleteAccountEvent::Confirm);
        crate::account_deletion::refresh(&self.bills, &self.ledger, cx);
        self.closed(router, cx);
    }

    /// A cancelled delete has written nothing.
    fn cancel(&mut self, cx: &mut Context<'_, Self>) {
        let router = self.send(DeleteAccountEvent::Cancel);
        self.closed(router, cx);
    }

    /// The Dialog has no select.
    fn close_open_select(&mut self) -> bool {
        false
    }

    fn hints(&self, keymap: &Keymap<DialogCommand>) -> Option<Vec<Hint>> {
        let key = |command| keymap.chord(command).unwrap_or_default().to_string();
        Some(vec![
            (key(BACK), crate::msg::desktop_hint_cancel()),
            (key(SELECT), crate::msg::desktop_hint_confirm()),
        ])
    }

    /// The typed-back name, the counts quoted, why it can't be deleted yet and whether it would
    /// confirm, for a test.
    fn readback(&self, _cx: &App) -> DialogReadback {
        let view = self.view();
        DialogReadback {
            kind: "DeleteAccount",
            values: vec![
                ("title", view.frame.title),
                ("confirm", view.confirm.text),
                ("transactions", view.usage.transactions.to_string()),
                ("budgets", view.usage.budgets.to_string()),
                ("blocked", view.blocked.unwrap_or_default()),
                ("refusal", view.refusal.unwrap_or_default()),
                ("can_confirm", view.frame.can_confirm.to_string()),
            ],
        }
    }
}

impl Render for DeleteAccountDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        render(&self.view(), &Clicks::new(cx), cx)
    }
}

fn render(view: &DeleteAccountViewModel, actions: &Clicks, cx: &App) -> AnyElement {
    let mut fields = vec![
        warning(view),
        dialog::info_panel(view.reference.clone(), cx).into_any_element(),
    ];
    fields.extend(view.blocked.clone().map(|text| callout(text, cx)));
    fields.push(text_field(
        "delete-account-confirm-input",
        confirm_label(&view.confirm.label),
        &view.confirm.text,
        &view.confirm.placeholder,
        view.confirm.focused,
        Rc::new(|_window: &mut Window, _cx: &mut App| {}),
        cx,
    ));
    fields.extend(view.refusal.clone().map(|text| error_line(text, cx)));

    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(view.frame.title.clone(), true, cx))
        .child(dialog::body(fields))
        .child(dialog::action_row(
            [
                dialog::cancel_button("delete-account-cancel", actions.cancel.clone(), cx)
                    .into_any_element(),
                dialog::confirm_button(
                    "delete-account-confirm",
                    view.frame.submit.clone(),
                    view.frame.can_confirm,
                    view.frame.destructive,
                    actions.confirm.clone(),
                    cx,
                )
                .into_any_element(),
            ],
            cx,
        ));
    dialog::overlay(WIDTH, true, card, cx)
}

/// `This account has a balance of **−2,318.44 aud** and **204 transactions.** Deleting it cannot
/// be undone.`, the two figures bold, as in the mockup.
fn warning(view: &DeleteAccountViewModel) -> AnyElement {
    div()
        .flex()
        .flex_wrap()
        .text_size(px(13.0))
        .children(view.warning.iter().map(|segment| {
            let text = div().child(segment.text.clone());
            match segment.tag.as_deref() {
                Some("strong") => text.font_weight(gpui::FontWeight::EXTRA_BOLD),
                _ => text,
            }
        }))
        .into_any_element()
}

/// Why it can't be deleted yet, on the destructive frame's red-bordered callout.
fn callout(text: String, cx: &App) -> AnyElement {
    div()
        .p(px(10.0))
        .bg(color::chrome(cx))
        .border_l(px(2.0))
        .border_color(color::negative(cx))
        .text_size(px(11.5))
        .text_color(color::muted(cx))
        .child(text)
        .into_any_element()
}

fn confirm_label(label: &str) -> AnyElement {
    div()
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(12.0))
        .mb(px(6.0))
        .child(label.to_string())
        .into_any_element()
}

fn error_line(text: String, cx: &App) -> AnyElement {
    div()
        .mt(px(6.0))
        .text_size(px(11.5))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_color(color::accent_text(cx))
        .child(text)
        .into_any_element()
}
