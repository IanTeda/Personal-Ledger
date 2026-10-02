//! The Documents dialogs on the shared `crate::dialog` chrome: **Add document**, **Edit document**
//! and **Import files**. `Shell` owns the live forms (`documents_form`) and every keystroke while
//! one is open; this module only draws them.

use std::rc::Rc;

use gpui::{AnyElement, App, SharedString, Window, div, prelude::*, px};

use crate::{
    dialog,
    documents_form::{
        DocumentField, DocumentForm, DocumentOptions, FieldProblem, FormProblems, ImportForm,
    },
    theme::color,
    view::accounts::{
        add_dialog::{label, text_field, two_up},
        select_field::{self, SelectFieldProps},
    },
};

pub type OnFieldClick = Rc<dyn Fn(DocumentField, &mut Window, &mut App)>;
pub type OnOptionClick = Rc<dyn Fn(DocumentField, usize, &mut Window, &mut App)>;

pub struct FormHandlers {
    pub on_field_click: OnFieldClick,
    pub on_option_click: OnOptionClick,
    pub on_cancel: dialog::OnClick,
    pub on_confirm: dialog::OnClick,
}

pub struct FormProps<'a> {
    pub form: &'a DocumentForm,
    pub options: &'a DocumentOptions,
    pub problems: &'a FormProblems,
    pub valid: bool,
    pub handlers: FormHandlers,
}

/// Add (no `form.is_edit`) and Edit share one renderer; Edit drops the Path and the file note.
pub fn render_form(props: FormProps<'_>, cx: &App) -> AnyElement {
    let FormProps {
        form,
        options,
        problems,
        valid,
        handlers,
    } = props;
    let FormHandlers {
        on_field_click,
        on_option_click,
        on_cancel,
        on_confirm,
    } = handlers;
    let focused = |field: DocumentField| form.focused == field;
    let click = |field: DocumentField| -> dialog::OnClick {
        let on_field_click = on_field_click.clone();
        Rc::new(move |window: &mut Window, cx: &mut App| on_field_click(field, window, cx))
    };
    let select = |id: &'static str,
                  text: SharedString,
                  field: DocumentField,
                  list: &[String],
                  state: &crate::select::SelectState| {
        let on_option_click = on_option_click.clone();
        select_field::render(
            SelectFieldProps {
                id,
                label: text,
                options: list,
                state,
                focused: focused(field),
                accent: false,
                read_only: None,
                on_field_click: click(field),
                on_option_click: Rc::new(move |index: usize, window: &mut Window, cx: &mut App| {
                    on_option_click(field, index, window, cx)
                }),
            },
            cx,
        )
    };
    let with_error = |field: AnyElement, problem: Option<&FieldProblem>| -> AnyElement {
        div()
            .flex_1()
            .min_w(px(0.0))
            .child(field)
            .children(
                problem
                    .and_then(problem_text)
                    .map(|text| error_line(text, cx)),
            )
            .into_any_element()
    };

    let (id, title, submit) = if form.is_edit {
        (
            "edit-document",
            crate::msg::desktop_documents_edit_title(),
            crate::msg::desktop_documents_edit_submit(),
        )
    } else {
        (
            "add-document",
            crate::msg::desktop_documents_add_title(),
            crate::msg::desktop_documents_add_submit(),
        )
    };

    let mut fields: Vec<AnyElement> = Vec::new();
    if !form.is_edit {
        fields.push(with_error(
            text_field(
                "document-path",
                label(crate::msg::desktop_documents_field_path()),
                &form.path,
                &crate::msg::desktop_documents_field_path_placeholder(),
                focused(DocumentField::Path),
                click(DocumentField::Path),
                cx,
            ),
            problems.path.as_ref(),
        ));
    }
    fields.push(with_error(
        text_field(
            "document-title",
            label(crate::msg::desktop_documents_field_title()),
            &form.title,
            "",
            focused(DocumentField::Title),
            click(DocumentField::Title),
            cx,
        ),
        problems.title.as_ref(),
    ));
    fields.push(two_up([
        select(
            "document-type",
            crate::msg::desktop_documents_field_type().into(),
            DocumentField::Type,
            &options.type_labels,
            &form.doc_type,
        ),
        with_error(
            text_field(
                "document-date",
                label(crate::msg::desktop_documents_field_date()),
                &form.date,
                "",
                focused(DocumentField::Date),
                click(DocumentField::Date),
                cx,
            ),
            problems.date.as_ref(),
        ),
    ]));
    fields.push(select(
        "document-key-kind",
        crate::msg::desktop_documents_field_key_kind().into(),
        DocumentField::KeyKind,
        &options.key_labels,
        &form.key_kind,
    ));
    if form.key_kind(options).is_some() {
        fields.push(with_error(
            text_field(
                "document-key-date",
                label(crate::msg::desktop_documents_field_key_date()),
                &form.key_date,
                "",
                focused(DocumentField::KeyDate),
                click(DocumentField::KeyDate),
                cx,
            ),
            problems.key_date.as_ref(),
        ));
        fields.push(checkbox(
            form.reminder,
            focused(DocumentField::Reminder),
            click(DocumentField::Reminder),
            cx,
        ));
    }
    if !form.is_edit {
        fields.push(
            dialog::info_panel(crate::msg::desktop_documents_add_note(), cx).into_any_element(),
        );
    }

    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(title, false, cx))
        .child(dialog::body(fields))
        .child(dialog::action_row(
            [
                dialog::cancel_button(format!("{id}-cancel"), on_cancel, cx).into_any_element(),
                dialog::confirm_button(
                    format!("{id}-confirm"),
                    submit,
                    valid,
                    false,
                    on_confirm,
                    cx,
                )
                .into_any_element(),
            ],
            cx,
        ));
    dialog::overlay(dialog::WIDTH, false, card, cx)
}

pub struct ImportProps<'a> {
    pub form: &'a ImportForm,
    /// A path-by-path report of why a line cannot be imported, shown under the field.
    pub notes: Vec<String>,
    pub valid: bool,
    pub on_cancel: dialog::OnClick,
    pub on_confirm: dialog::OnClick,
}

pub fn render_import(props: ImportProps<'_>, cx: &App) -> AnyElement {
    let ImportProps {
        form,
        notes,
        valid,
        on_cancel,
        on_confirm,
    } = props;
    let lines = if form.paths.is_empty() {
        div()
            .text_color(color::faint_text(cx))
            .child(format!(
                "{}\u{2502}",
                crate::msg::desktop_documents_field_paths_placeholder()
            ))
            .into_any_element()
    } else {
        div()
            .flex()
            .flex_col()
            .children(
                form.paths
                    .split('\n')
                    .map(|line| div().min_h(px(16.0)).child(line.to_string())),
            )
            .child(div().text_color(color::accent(cx)).child("\u{2502}"))
            .into_any_element()
    };
    let field = div()
        .child(label(crate::msg::desktop_documents_field_paths()))
        .child(
            div()
                .id("documents-import-paths")
                .w_full()
                .min_h(px(96.0))
                .py(px(8.0))
                .px(px(10.0))
                .border_1()
                .border_color(color::accent(cx))
                .text_size(px(12.5))
                .child(lines),
        )
        .children(notes.into_iter().map(|note| error_line(note, cx)))
        .into_any_element();
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(
            crate::msg::desktop_documents_import_title(),
            false,
            cx,
        ))
        .child(dialog::body([
            field,
            dialog::info_panel(crate::msg::desktop_documents_import_note(), cx).into_any_element(),
        ]))
        .child(dialog::action_row(
            [
                dialog::cancel_button("import-documents-cancel", on_cancel, cx).into_any_element(),
                dialog::confirm_button(
                    "import-documents-confirm",
                    crate::msg::desktop_documents_import_submit(),
                    valid,
                    false,
                    on_confirm,
                    cx,
                )
                .into_any_element(),
            ],
            cx,
        ));
    dialog::overlay(dialog::WIDTH, false, card, cx)
}

/// A problem as the dialog words it. A blank date hint (a Key Date still to type) says nothing.
pub fn problem_text(problem: &FieldProblem) -> Option<String> {
    match problem {
        FieldProblem::NoPath => Some(crate::msg::desktop_documents_error_no_path()),
        FieldProblem::PathMissing(path) => {
            Some(crate::msg::desktop_documents_error_path_missing(path))
        }
        FieldProblem::Unsupported => Some(crate::msg::desktop_documents_error_unsupported()),
        FieldProblem::Already(title) => Some(crate::msg::desktop_documents_error_already(title)),
        FieldProblem::NoTitle => Some(crate::msg::desktop_documents_error_no_title()),
        FieldProblem::Date(hint) if hint.is_empty() => None,
        FieldProblem::Date(hint) => Some(hint.clone()),
    }
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

/// The Key Date reminder checkbox: a 14px box, filled while checked.
fn checkbox(checked: bool, focused: bool, on_click: dialog::OnClick, cx: &App) -> AnyElement {
    div()
        .id("document-reminder")
        .cursor_pointer()
        .flex()
        .items_center()
        .gap(px(8.0))
        .text_size(px(13.0))
        .on_click(move |_event, window, cx| on_click(window, cx))
        .child(
            div()
                .size(px(14.0))
                .flex()
                .items_center()
                .justify_center()
                .border_1()
                .border_color(if focused {
                    color::accent(cx)
                } else {
                    color::foreground(cx)
                })
                .when(checked, |this| {
                    this.bg(color::foreground(cx))
                        .text_color(color::background(cx))
                        .text_size(px(10.0))
                        .child("\u{2713}")
                }),
        )
        .child(crate::msg::desktop_documents_field_reminder())
        .into_any_element()
}
