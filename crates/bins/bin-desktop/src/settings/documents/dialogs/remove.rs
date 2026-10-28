//! The Remove **Document type** dialog and Other's notice. Remove asks a destination only when the
//! type has Filed files; Other, the Default, gets a notice instead. A pure render-helper.

use gpui::{AnyElement, App, div, prelude::*, px};
use lib_documents::types::{DocumentTypeRow, RemoveError};

use crate::{
    dialog,
    form::select::SelectState,
    form::select_field::{self, OnOptionClick, SelectFieldProps},
    theme::color,
};

pub struct RemoveHandlers {
    pub on_field_click: dialog::OnClick,
    pub on_option_click: OnOptionClick,
    pub on_cancel: dialog::OnClick,
    pub on_confirm: dialog::OnClick,
}

pub struct RemoveProps<'a> {
    pub row: &'a DocumentTypeRow,
    pub destinations: &'a [String],
    pub destination: &'a SelectState,
    /// What the domain refused the confirm with.
    pub error: Option<RemoveError>,
    pub handlers: RemoveHandlers,
}

/// Remove: red treatment and a destination dropdown while the type has Filed files, a plain
/// confirm otherwise.
pub fn render_remove(props: RemoveProps<'_>, cx: &App) -> AnyElement {
    let RemoveProps {
        row,
        destinations,
        destination,
        error,
        handlers,
    } = props;
    let has_files = row.files > 0;
    let files = i64::from(row.files);
    let mut body: Vec<AnyElement> = if has_files {
        vec![
            div()
                .text_size(px(13.0))
                .child(crate::msg::desktop_document_types_remove_with_files(
                    &row.name, files,
                ))
                .into_any_element(),
            select_field::render(
                SelectFieldProps {
                    id: "document-type-destination".into(),
                    label: crate::msg::desktop_document_types_remove_destination().into(),
                    options: destinations,
                    state: destination,
                    focused: true,
                    accent: false,
                    read_only: None,
                    on_field_click: handlers.on_field_click,
                    on_option_click: handlers.on_option_click,
                },
                cx,
            ),
            dialog::info_panel(crate::msg::desktop_document_types_remove_moved_note(), cx)
                .into_any_element(),
        ]
    } else {
        vec![
            div()
                .text_size(px(13.0))
                .child(crate::msg::desktop_document_types_remove_no_files(
                    &row.name,
                ))
                .into_any_element(),
        ]
    };
    if error.is_some() {
        body.push(
            div()
                .text_size(px(11.5))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_color(color::accent_text(cx))
                .child(crate::msg::desktop_document_types_remove_refused())
                .into_any_element(),
        );
    }
    let confirm = if has_files {
        crate::msg::desktop_document_types_remove_confirm_move(files)
    } else {
        crate::msg::desktop_document_types_remove_confirm()
    };
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(
            crate::msg::desktop_document_types_dialog_remove_title(),
            has_files,
            cx,
        ))
        .child(dialog::body(body))
        .child(dialog::action_row(
            [
                dialog::cancel_button("remove-document-type-cancel", handlers.on_cancel, cx)
                    .into_any_element(),
                dialog::confirm_button(
                    "remove-document-type-confirm",
                    confirm,
                    true,
                    has_files,
                    handlers.on_confirm,
                    cx,
                )
                .into_any_element(),
            ],
            cx,
        ));
    dialog::overlay(dialog::WIDTH, has_files, card, cx)
}

/// Other's notice: why it has no remove action.
pub fn render_default_notice(on_close: dialog::OnClick, cx: &App) -> AnyElement {
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(
            crate::msg::desktop_document_types_dialog_default_title(),
            false,
            cx,
        ))
        .child(dialog::body([dialog::info_panel(
            crate::msg::desktop_document_types_default_notice(),
            cx,
        )
        .into_any_element()]))
        .child(dialog::action_row(
            [dialog::confirm_button(
                "document-type-notice-close",
                crate::msg::desktop_document_types_dialog_close(),
                true,
                false,
                on_close,
                cx,
            )
            .into_any_element()],
            cx,
        ));
    dialog::overlay(dialog::WIDTH, false, card, cx)
}
