//! The Delete institution Dialog's View on the shared `crate::dialog` frame's destructive variant:
//! the typed-back name, which holds every typed key, or, when an Account names the Institution, the
//! refusal with the Accounts' names and only a Close button.

use gpui::{AnyElement, App, SharedString, div, prelude::*, px};
use lib_institutions::presenter::{DeleteInstitutionPresenter, DeleteInstitutionViewModel};
use lib_shell::Hint;

use crate::dialog;
use crate::shell::view::dialog_entity::{DialogView, ModuleDialogClicks, framed};
use crate::theme::color;

const WIDTH: gpui::Pixels = px(440.0);

/// The open Dialog's clicks.
type Clicks = ModuleDialogClicks<DeleteInstitutionPresenter>;

/// The `lib-institutions` Delete Presenter's [`DialogView`], hosted in a `ModuleDialog`
/// (ADR-0056).
impl DialogView for DeleteInstitutionPresenter {
    type Widgets = ();

    fn kind(_view: &DeleteInstitutionViewModel) -> &'static str {
        "DeleteInstitution"
    }

    fn render(
        view: &DeleteInstitutionViewModel,
        _widgets: &mut (),
        clicks: &Clicks,
        cx: &App,
    ) -> AnyElement {
        render_view(view, clicks, cx)
    }

    /// The title, the typed-back name, what the Dialog says and the Accounts it lists, for a test.
    /// A refusal has no typed-back name, and reads `refused`.
    fn readback(view: &DeleteInstitutionViewModel) -> Vec<(&'static str, String)> {
        framed(&view.frame, readback(view))
    }

    fn hints(view: &DeleteInstitutionViewModel) -> Vec<Hint> {
        view.hints.clone()
    }
}

/// The typed-back name, what the Dialog says and the Accounts it lists, for a test.
fn readback(view: &DeleteInstitutionViewModel) -> Vec<(&'static str, String)> {
    vec![
        (
            "confirm",
            view.confirm
                .as_ref()
                .map(|field| field.text.clone())
                .unwrap_or_default(),
        ),
        ("notice", view.notice.clone()),
        ("accounts", view.accounts.join(", ")),
        ("more", view.more.clone().unwrap_or_default()),
        ("refused", view.confirm.is_none().to_string()),
    ]
}

fn render_view(view: &DeleteInstitutionViewModel, actions: &Clicks, cx: &App) -> AnyElement {
    let mut fields = vec![
        div()
            .text_size(px(13.0))
            .child(view.notice.clone())
            .into_any_element(),
    ];
    if !view.accounts.is_empty() {
        fields.push(accounts_list(view, cx));
    }
    fields.extend(
        view.hint
            .clone()
            .map(|hint| dialog::info_panel(hint, cx).into_any_element()),
    );
    if let Some(confirm) = &view.confirm {
        fields.push(confirm_field(
            &confirm.label,
            &confirm.text,
            &confirm.placeholder,
            cx,
        ));
    }
    let buttons = match view.confirm {
        Some(_) => vec![
            dialog::cancel_button("delete-institution-cancel", actions.cancel.clone(), cx)
                .into_any_element(),
            dialog::confirm_button(
                "delete-institution-confirm",
                view.frame.submit.clone(),
                view.frame.can_confirm,
                true,
                actions.confirm.clone(),
                cx,
            )
            .into_any_element(),
        ],
        // A refusal offers Close and nothing destructive.
        None => vec![
            dialog::confirm_button(
                "delete-institution-close",
                view.frame.submit.clone(),
                true,
                false,
                actions.cancel.clone(),
                cx,
            )
            .into_any_element(),
        ],
    };
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(
            view.frame.title.clone(),
            view.frame.destructive,
            cx,
        ))
        .child(dialog::body(fields))
        .child(dialog::action_row(buttons, cx));
    dialog::overlay(WIDTH, view.frame.destructive, card, cx)
}

/// The Accounts naming the Institution, one to a line, then `and N more`.
fn accounts_list(view: &DeleteInstitutionViewModel, cx: &App) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .text_size(px(13.0))
        .children(view.accounts.iter().enumerate().map(|(index, name)| {
            let id = SharedString::from(format!("delete-institution-account-{index}"));
            div()
                .debug_selector({
                    let id = id.clone();
                    move || id.to_string()
                })
                .id(id)
                .font_weight(gpui::FontWeight::BOLD)
                .child(name.clone())
        }))
        .children(view.more.clone().map(|more| {
            div()
                .debug_selector(|| "delete-institution-more".to_string())
                .id("delete-institution-more")
                .text_color(color::muted(cx))
                .child(more)
        }))
        .into_any_element()
}

/// The typed-back name, its placeholder the name itself.
fn confirm_field(label: &str, text: &str, placeholder: &str, cx: &App) -> AnyElement {
    let (shown, text_color) = if text.is_empty() {
        (placeholder.to_string(), color::muted(cx))
    } else {
        (text.to_string(), color::foreground(cx))
    };
    div()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(12.0))
                .child(label.to_string()),
        )
        .child(
            div()
                .w_full()
                .py(px(8.0))
                .px(px(10.0))
                .border_1()
                .border_color(color::border(cx))
                .text_size(px(13.0))
                .text_color(text_color)
                .child(SharedString::from(shown)),
        )
        .into_any_element()
}
