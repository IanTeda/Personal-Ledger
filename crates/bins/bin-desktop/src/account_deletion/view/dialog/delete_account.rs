//! The Delete account Dialog (`docs/ux/desktop-mockups/17-accounts/README.md`'s 3d) on the shared
//! `crate::dialog` frame's destructive variant, hosted by the generic `ModuleDialog`
//! (`shell::view::dialog_entity`): what goes, why it can't go yet while a Bill Plan pays from the
//! Account, and the typed-back name. Every key and click becomes a `lib-account-deletion`
//! Presenter event, and this draws the view model and nothing else.
//!
//! Every Dialog command and typed key reaches the Presenter as a `lib-viper` `DialogInput`: typing
//! edits the typed-back name, and `select` confirms once the Presenter would.

use std::rc::Rc;

use gpui::{AnyElement, App, Window, div, prelude::*, px};
use lib_account_deletion::presenter::{DeleteAccountPresenter, DeleteAccountViewModel};
use lib_shell::Hint;

use crate::dialog;
use crate::form::dialog_fields::text_field;
use crate::shell::view::dialog_entity::{DialogView, ModuleDialogClicks, framed};
use crate::theme::color;

/// The Dialog's width: the Accounts dialogs' 440px.
const WIDTH: gpui::Pixels = px(440.0);

/// The open Delete Dialog's clicks.
type Clicks = ModuleDialogClicks<DeleteAccountPresenter>;

/// The `lib-account-deletion` Presenter's [`DialogView`], hosted in a `ModuleDialog` (ADR-0056).
impl DialogView for DeleteAccountPresenter {
    type Widgets = ();

    fn kind(_view: &DeleteAccountViewModel) -> &'static str {
        "DeleteAccount"
    }

    fn render(
        view: &DeleteAccountViewModel,
        _widgets: &mut (),
        clicks: &Clicks,
        cx: &App,
    ) -> AnyElement {
        render(view, clicks, cx)
    }

    /// The typed-back name, the counts quoted and why it can't be deleted yet, between the title
    /// and whether it would confirm.
    fn readback(view: &DeleteAccountViewModel) -> Vec<(&'static str, String)> {
        framed(
            &view.frame,
            vec![
                ("confirm", view.confirm.text.clone()),
                ("transactions", view.usage.transactions.to_string()),
                ("budgets", view.usage.budgets.to_string()),
                ("blocked", view.blocked.clone().unwrap_or_default()),
                ("refusal", view.refusal.clone().unwrap_or_default()),
            ],
        )
    }

    fn hints(view: &DeleteAccountViewModel) -> Vec<Hint> {
        view.hints.clone()
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
