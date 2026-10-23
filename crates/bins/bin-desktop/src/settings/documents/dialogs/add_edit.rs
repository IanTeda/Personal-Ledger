//! The Add and Edit **Document type** dialogs (#478's stacked-form variant A) on the shared
//! `crate::dialog` chrome. They share one renderer: Name, Tracks date, Remind and Financial year
//! stacked with a hint under each. `Shell` owns the live state and every key; this module is a
//! pure render-helper.

use std::rc::Rc;

use gpui::{AnyElement, App, SharedString, Window, div, prelude::*, px};

use lib_documents::types::{DocumentTypeRow, RemindLead, TracksDate};

use super::super::{
    form::{DocumentTypeForm, FormField, NAME_MAX, NameError, REMIND_OPTIONS, TRACKS_OPTIONS},
    render::tracks_label,
};
use crate::{
    dialog,
    form::fields::{OnSegmentClick, field_label, segmented_control, text_field},
    theme::color,
};

pub type OnFieldClick = Rc<dyn Fn(FormField, &mut Window, &mut App)>;

pub struct FormHandlers {
    pub on_field_click: OnFieldClick,
    pub on_tracks_click: OnSegmentClick<Option<TracksDate>>,
    pub on_remind_click: OnSegmentClick<Option<RemindLead>>,
    pub on_cancel: dialog::OnClick,
    pub on_confirm: dialog::OnClick,
}

/// Add (`edit` is `None`) or Edit (the row being edited, for its usage notice).
pub struct FormProps<'a> {
    pub form: &'a DocumentTypeForm,
    pub edit: Option<&'a DocumentTypeRow>,
    pub error: Option<NameError>,
    pub valid: bool,
    pub handlers: FormHandlers,
}

fn option_none() -> String {
    crate::msg::desktop_document_types_option_none()
}

fn tracks_option_label(option: Option<TracksDate>) -> String {
    option.map_or_else(option_none, tracks_label)
}

fn remind_option_label(option: Option<RemindLead>) -> String {
    match option {
        None => option_none(),
        Some(RemindLead::Days(days)) => {
            crate::msg::desktop_document_types_option_days(&days.to_string())
        }
        Some(RemindLead::Months(months)) => {
            crate::msg::desktop_document_types_option_months(&months.to_string())
        }
    }
}

fn hint(text: String, cx: &App) -> impl IntoElement {
    div()
        .mt(px(4.0))
        .text_size(px(11.5))
        .text_color(color::muted(cx))
        .child(text)
}

fn error_text(error: NameError) -> String {
    match error {
        NameError::Empty => crate::msg::desktop_document_types_error_name_empty(),
        NameError::TooLong => crate::msg::desktop_document_types_error_name_too_long(
            i64::try_from(NAME_MAX).unwrap_or(i64::MAX),
        ),
        NameError::Taken => crate::msg::desktop_document_types_error_name_taken(),
    }
}

pub fn render_form(props: FormProps<'_>, cx: &App) -> AnyElement {
    let FormProps {
        form,
        edit,
        error,
        valid,
        handlers,
    } = props;
    let (prefix, title, submit) = if edit.is_some() {
        (
            "edit-document-type",
            crate::msg::desktop_document_types_dialog_edit_title(),
            crate::msg::desktop_document_types_dialog_save(),
        )
    } else {
        (
            "add-document-type",
            crate::msg::desktop_document_types_dialog_add_title(),
            crate::msg::desktop_document_types_dialog_add_submit(),
        )
    };
    let click = |field: FormField| -> dialog::OnClick {
        let on_click = handlers.on_field_click.clone();
        Rc::new(move |window: &mut Window, cx: &mut App| on_click(field, window, cx))
    };
    // The error shows once the name has been touched; the confirm is disabled either way.
    let shown_error = error.filter(|_| form.touched);
    let remind_enabled = form.remind_enabled();

    let mut fields: Vec<AnyElement> = Vec::new();
    if let Some(row) = edit {
        let dated = if row.tracks_date.is_some() {
            row.files
        } else {
            0
        };
        fields.push(
            dialog::info_panel(
                crate::msg::desktop_document_types_edit_usage(
                    i64::from(row.files),
                    i64::from(dated),
                ),
                cx,
            )
            .into_any_element(),
        );
    }
    fields.push(
        div()
            .child(text_field(
                "document-type-name",
                crate::msg::desktop_document_types_field_name(),
                form.name.text(),
                &crate::msg::desktop_document_types_field_name_placeholder(),
                form.focused == FormField::Name,
                click(FormField::Name),
                cx,
            ))
            .child(match shown_error {
                Some(error) => div()
                    .mt(px(4.0))
                    .text_size(px(11.5))
                    .text_color(color::accent_text(cx))
                    .child(error_text(error))
                    .into_any_element(),
                None => hint(
                    crate::msg::desktop_document_types_field_name_hint(
                        i64::try_from(NAME_MAX).unwrap_or(i64::MAX),
                    ),
                    cx,
                )
                .into_any_element(),
            })
            .into_any_element(),
    );
    fields.push(
        div()
            .child(field_label(
                crate::msg::desktop_document_types_field_tracks_date(),
            ))
            .child(segmented_control(
                "document-type-tracks",
                &TRACKS_OPTIONS,
                form.tracks_date,
                tracks_option_label,
                handlers.on_tracks_click.clone(),
                cx,
            ))
            .child(hint(
                crate::msg::desktop_document_types_field_tracks_date_hint(),
                cx,
            ))
            .into_any_element(),
    );
    fields.push(
        div()
            .when(!remind_enabled, |this| this.opacity(0.45))
            .child(field_label(
                crate::msg::desktop_document_types_field_remind(),
            ))
            .child(segmented_control(
                "document-type-remind",
                &REMIND_OPTIONS,
                form.remind,
                remind_option_label,
                handlers.on_remind_click.clone(),
                cx,
            ))
            .child(hint(
                crate::msg::desktop_document_types_field_remind_hint(),
                cx,
            ))
            .into_any_element(),
    );
    fields.push(
        div()
            .child(field_label(
                crate::msg::desktop_document_types_field_financial_year(),
            ))
            .child(financial_year_chip(
                form.financial_year,
                form.focused == FormField::FinancialYear,
                click(FormField::FinancialYear),
                cx,
            ))
            .child(hint(
                crate::msg::desktop_document_types_field_financial_year_hint(),
                cx,
            ))
            .into_any_element(),
    );

    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(title, false, cx))
        .child(dialog::body(fields))
        .child(dialog::action_row(
            [
                dialog::cancel_button(
                    SharedString::from(format!("{prefix}-cancel")),
                    handlers.on_cancel,
                    cx,
                )
                .into_any_element(),
                dialog::confirm_button(
                    SharedString::from(format!("{prefix}-confirm")),
                    submit,
                    valid,
                    false,
                    handlers.on_confirm,
                    cx,
                )
                .into_any_element(),
            ],
            cx,
        ));
    dialog::overlay(dialog::WIDTH, false, card, cx)
}

/// The Financial year toggle chip: filled while on, accent-bordered while focused.
fn financial_year_chip(
    on: bool,
    focused: bool,
    on_click: dialog::OnClick,
    cx: &App,
) -> impl IntoElement {
    div().flex().child(
        div()
            .id("document-type-financial-year")
            .debug_selector(|| "document-type-financial-year".to_string())
            .cursor_pointer()
            .py(px(7.0))
            .px(px(12.0))
            .border_1()
            .border_color(if focused {
                color::accent(cx)
            } else {
                color::divider(cx)
            })
            .text_size(px(13.0))
            .when(on, |this| {
                this.bg(color::accent(cx)).text_color(color::background(cx))
            })
            .on_click(move |_event, window, cx| on_click(window, cx))
            .child(if on {
                crate::msg::desktop_document_types_yes()
            } else {
                crate::msg::desktop_document_types_no()
            }),
    )
}
