//! The Delete category Dialog (`docs/ux/desktop-mockups/18-categories/README.md`'s 5d) on the
//! shared `crate::dialog` frame's destructive variant: the warning naming the Category's
//! transactions, the Uncategorised notice with its Budgets, and the typed-back name, which holds
//! every typed key.

use gpui::{AnyElement, App, SharedString, div, prelude::*, px};
use lib_category_deletion::presenter::{DeleteCategoryPresenter, DeleteCategoryViewModel};
use lib_shell::Hint;

use crate::shell::view::dialog_entity::{DialogView, ModuleDialogClicks, framed};
use crate::{dialog, theme::color};

/// The Dialog's width: the handoff's 5d.
const WIDTH: gpui::Pixels = px(440.0);

/// The open Dialog's clicks.
type Clicks = ModuleDialogClicks<DeleteCategoryPresenter>;

/// The `lib-category-deletion` Presenter's [`DialogView`], hosted in a `ModuleDialog` (ADR-0056).
impl DialogView for DeleteCategoryPresenter {
    type Widgets = ();

    fn kind(_view: &DeleteCategoryViewModel) -> &'static str {
        "DeleteCategory"
    }

    fn render(
        view: &DeleteCategoryViewModel,
        _widgets: &mut (),
        clicks: &Clicks,
        cx: &App,
    ) -> AnyElement {
        render(view, clicks, cx)
    }

    /// The typed-back name and the warning, between the title and whether it would confirm.
    fn readback(view: &DeleteCategoryViewModel) -> Vec<(&'static str, String)> {
        framed(&view.frame, readback(view))
    }

    fn hints(view: &DeleteCategoryViewModel) -> Vec<Hint> {
        view.hints.clone()
    }
}

fn render(view: &DeleteCategoryViewModel, actions: &Clicks, cx: &App) -> AnyElement {
    let mut fields = vec![
        warning(view),
        dialog::info_panel(view.reference.clone(), cx).into_any_element(),
        confirm_field(
            &view.confirm.label,
            &view.confirm.text,
            &view.confirm.placeholder,
            cx,
        ),
    ];
    fields.extend(view.refusal.clone().map(|text| error_line(text, cx)));
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(view.frame.title.clone(), true, cx))
        .child(dialog::body(fields))
        .child(dialog::action_row(
            [
                dialog::cancel_button("delete-category-cancel", actions.cancel.clone(), cx)
                    .into_any_element(),
                dialog::confirm_button(
                    "delete-category-confirm",
                    view.frame.submit.clone(),
                    view.frame.can_confirm,
                    true,
                    actions.confirm.clone(),
                    cx,
                )
                .into_any_element(),
            ],
            cx,
        ));
    dialog::overlay(WIDTH, true, card, cx)
}

fn warning(view: &DeleteCategoryViewModel) -> AnyElement {
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

/// A refused confirm, under the field, as the Categories form draws its own.
fn error_line(text: String, cx: &App) -> AnyElement {
    div()
        .mt(px(4.0))
        .text_size(px(11.5))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_color(color::accent_text(cx))
        .child(text)
        .into_any_element()
}

/// The typed-back name and the warning, for a test.
fn readback(view: &DeleteCategoryViewModel) -> Vec<(&'static str, String)> {
    vec![
        ("confirm", view.confirm.text.clone()),
        (
            "warning",
            view.warning
                .iter()
                .map(|segment| segment.text.as_str())
                .collect(),
        ),
        ("refusal", view.refusal.clone().unwrap_or_default()),
    ]
}
