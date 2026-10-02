//! The Documents dialogs on the shared `crate::dialog` chrome: **Add document**, **Edit document**
//! and **Import files**. `Shell` owns the live forms (`documents_form`) and every keystroke while
//! one is open; this module only draws them.

use std::rc::Rc;

use gpui::{AnyElement, App, SharedString, Window, div, prelude::*, px};

use crate::{
    dialog,
    documents_form::{
        DocumentField, DocumentForm, DocumentOptions, FactsField, FactsForm, FactsProblems,
        FieldProblem, FormProblems, ImportForm,
    },
    documents_picker::{self, KindFilter, PickerRow, PickerState, Purpose},
    theme::color,
    view::accounts::{
        add_dialog::{label, text_field, two_up},
        select_field::{self, SelectFieldProps},
    },
};

pub type OnFieldClick = Rc<dyn Fn(DocumentField, &mut Window, &mut App)>;
pub type OnOptionClick = Rc<dyn Fn(DocumentField, usize, &mut Window, &mut App)>;
pub type OnFactsFieldClick = Rc<dyn Fn(FactsField, &mut Window, &mut App)>;
pub type OnFactsOptionClick = Rc<dyn Fn(usize, &mut Window, &mut App)>;

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

pub struct FactsProps<'a> {
    pub form: &'a FactsForm,
    pub options: &'a DocumentOptions,
    pub problems: &'a FactsProblems,
    pub valid: bool,
    pub on_field_click: OnFactsFieldClick,
    pub on_option_click: OnFactsOptionClick,
    pub on_cancel: dialog::OnClick,
    pub on_confirm: dialog::OnClick,
}

/// **Edit extracted facts**: merchant, date, total and Document Type of an Unfiled Document.
pub fn render_facts(props: FactsProps<'_>, cx: &App) -> AnyElement {
    let FactsProps {
        form,
        options,
        problems,
        valid,
        on_field_click,
        on_option_click,
        on_cancel,
        on_confirm,
    } = props;
    let click = |field: FactsField| -> dialog::OnClick {
        let on_field_click = on_field_click.clone();
        Rc::new(move |window: &mut Window, cx: &mut App| on_field_click(field, window, cx))
    };
    let focused = |field: FactsField| form.focused == field;
    let with_error = |field: AnyElement, text: Option<String>| -> AnyElement {
        div()
            .flex_1()
            .min_w(px(0.0))
            .child(field)
            .children(text.map(|text| error_line(text, cx)))
            .into_any_element()
    };
    let type_field = select_field::render(
        SelectFieldProps {
            id: "facts-type",
            label: crate::msg::desktop_documents_field_type().into(),
            options: &options.type_labels,
            state: &form.doc_type,
            focused: focused(FactsField::Type),
            accent: false,
            read_only: None,
            on_field_click: click(FactsField::Type),
            on_option_click: Rc::new(move |index: usize, window: &mut Window, cx: &mut App| {
                on_option_click(index, window, cx)
            }),
        },
        cx,
    );
    let fields = vec![
        text_field(
            "facts-merchant",
            label(crate::msg::desktop_documents_fact_merchant()),
            &form.merchant,
            "",
            focused(FactsField::Merchant),
            click(FactsField::Merchant),
            cx,
        )
        .into_any_element(),
        two_up([
            with_error(
                text_field(
                    "facts-date",
                    label(crate::msg::desktop_documents_field_date()),
                    &form.date,
                    "",
                    focused(FactsField::Date),
                    click(FactsField::Date),
                    cx,
                ),
                problems.date.clone().filter(|hint| !hint.is_empty()),
            ),
            with_error(
                text_field(
                    "facts-total",
                    label(crate::msg::desktop_documents_fact_total()),
                    &form.total,
                    "",
                    focused(FactsField::Total),
                    click(FactsField::Total),
                    cx,
                ),
                problems
                    .total
                    .then(crate::msg::desktop_documents_error_total),
            ),
        ]),
        type_field,
        dialog::info_panel(crate::msg::desktop_documents_facts_note(), cx).into_any_element(),
    ];
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(
            crate::msg::desktop_documents_facts_title(),
            false,
            cx,
        ))
        .child(dialog::body(fields))
        .child(dialog::action_row(
            [
                dialog::cancel_button("facts-cancel", on_cancel, cx).into_any_element(),
                dialog::confirm_button(
                    "facts-confirm",
                    crate::msg::desktop_documents_facts_submit(),
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

/// The count-first confirm for Accept all strong matches.
pub fn render_accept_all(
    count: usize,
    on_cancel: dialog::OnClick,
    on_confirm: dialog::OnClick,
    cx: &App,
) -> AnyElement {
    let shown = i64::try_from(count).unwrap_or(i64::MAX);
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(
            crate::msg::desktop_documents_accept_all_title(),
            false,
            cx,
        ))
        .child(dialog::body([
            div()
                .text_size(px(13.0))
                .child(crate::msg::desktop_documents_accept_all_body(shown))
                .into_any_element(),
            dialog::info_panel(crate::msg::desktop_documents_accept_all_note(), cx)
                .into_any_element(),
        ]))
        .child(dialog::action_row(
            [
                dialog::cancel_button("accept-all-cancel", on_cancel, cx).into_any_element(),
                dialog::confirm_button(
                    "accept-all-confirm",
                    crate::msg::desktop_documents_accept_all_submit(&count.to_string()),
                    true,
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

pub type OnPickerRow = Rc<dyn Fn(usize, &mut Window, &mut App)>;

pub struct PickerProps<'a> {
    pub state: &'a PickerState,
    pub rows: &'a [PickerRow],
    /// The Inbox's chosen Document Type, if any.
    pub type_label: Option<String>,
    pub on_pick: OnPickerRow,
}

/// The link picker in the command palette's chrome: a query line, the kind filter, an Inbox-only
/// Document Type line and a capped, scrolling list of results.
pub fn render_picker(props: PickerProps<'_>, cx: &App) -> AnyElement {
    let PickerProps {
        state,
        rows,
        type_label,
        on_pick,
    } = props;
    let selected = state.selected.min(rows.len().saturating_sub(1));
    let window =
        documents_picker::visible_window(selected, rows.len(), documents_picker::VISIBLE_ROWS);
    let title = match state.purpose {
        Purpose::Link(_) => crate::msg::desktop_documents_picker_title_link(),
        Purpose::File(_) => crate::msg::desktop_documents_picker_title_file(),
        Purpose::Follow(_) => crate::msg::desktop_documents_picker_title_follow(),
    };
    let hint = match state.purpose {
        Purpose::Link(_) => crate::msg::desktop_documents_picker_hint_toggle(),
        Purpose::File(_) => crate::msg::desktop_documents_picker_hint_pick(),
        Purpose::Follow(_) => crate::msg::desktop_documents_picker_hint_follow(),
    };
    let following = matches!(state.purpose, Purpose::Follow(_));
    let filing = matches!(state.purpose, Purpose::File(_));

    let query_line = div()
        .flex()
        .items_center()
        .gap(px(10.0))
        .px(px(16.0))
        .py(px(12.0))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(15.0))
        .text_color(color::foreground(cx))
        .child(div().text_color(color::muted(cx)).child(title))
        .child(div().flex_1().child(if state.query.is_empty() {
            div()
                .font_weight(gpui::FontWeight::NORMAL)
                .text_color(color::faint_text(cx))
                .child(crate::msg::desktop_documents_picker_placeholder())
        } else {
            div().child(state.query.clone())
        }))
        .child(div().w(px(8.0)).h(px(17.0)).bg(color::cursor(cx)));

    let kinds = div()
        .flex()
        .gap(px(14.0))
        .px(px(16.0))
        .py(px(8.0))
        .text_size(px(11.5))
        .children(
            [
                KindFilter::All,
                KindFilter::Transaction,
                KindFilter::Account,
                KindFilter::Payee,
                KindFilter::Bill,
                KindFilter::Inventory,
            ]
            .map(|kind| {
                let active = kind == state.kind;
                div()
                    .when(active, |this| {
                        this.font_weight(gpui::FontWeight::EXTRA_BOLD)
                            .text_color(color::foreground(cx))
                            .border_b_2()
                            .border_color(color::foreground(cx))
                    })
                    .when(!active, |this| this.text_color(color::muted(cx)))
                    .child(kind.label())
            }),
        );

    let type_line = filing.then(|| {
        div()
            .flex()
            .gap(px(8.0))
            .px(px(16.0))
            .py(px(6.0))
            .text_size(px(12.0))
            .child(
                div()
                    .text_color(color::faint_text(cx))
                    .child(crate::msg::desktop_documents_picker_type()),
            )
            .child(match type_label {
                Some(label) => div()
                    .font_weight(gpui::FontWeight::EXTRA_BOLD)
                    .child(format!("\u{2039} {label} \u{203a}")),
                None => div()
                    .text_color(color::muted(cx))
                    .child(crate::msg::desktop_documents_picker_type_none()),
            })
    });

    let list = div()
        .flex()
        .flex_col()
        .children(
            rows[window.clone()]
                .iter()
                .enumerate()
                .map(|(offset, row)| {
                    let index = window.start + offset;
                    let on_pick = on_pick.clone();
                    picker_row(index, row, index == selected, on_pick, cx)
                }),
        );

    div()
        .absolute()
        .top(crate::palette::TOP_OFFSET)
        .left(px(0.0))
        .right(px(0.0))
        .flex()
        .justify_center()
        .child(
            div()
                .w(crate::palette::WIDTH)
                .flex()
                .flex_col()
                .bg(color::background(cx))
                .border_2()
                .border_color(color::foreground(cx))
                .child(query_line)
                .child(div().h(px(2.0)).bg(color::foreground(cx)))
                .when(!following, |this| this.child(kinds))
                .children(type_line)
                .when(rows.is_empty(), |this| {
                    this.child(
                        div()
                            .px(px(16.0))
                            .py(px(12.0))
                            .text_size(px(12.5))
                            .text_color(color::muted(cx))
                            .child(crate::msg::desktop_documents_picker_empty()),
                    )
                })
                .child(list)
                .child(div().h(px(1.0)).bg(color::hairline(cx)))
                .child(
                    div()
                        .px(px(16.0))
                        .py(px(8.0))
                        .text_size(px(11.5))
                        .text_color(color::muted(cx))
                        .child(hint),
                ),
        )
        .into_any_element()
}

fn picker_row(
    index: usize,
    row: &PickerRow,
    selected: bool,
    on_pick: OnPickerRow,
    cx: &App,
) -> impl IntoElement {
    let (bg, text, faint) = if selected {
        (
            Some(color::selection_background(cx)),
            color::selection_text(cx),
            color::selection_muted(cx),
        )
    } else {
        (None, color::foreground(cx), color::faint_text(cx))
    };
    div()
        .id(("documents-picker-row", index))
        .debug_selector(|| format!("documents-picker-row-{index}"))
        .cursor_pointer()
        .when_some(bg, |this, bg| this.bg(bg))
        .flex()
        .items_center()
        .gap(px(12.0))
        .px(px(16.0))
        .py(px(8.0))
        .text_size(px(13.0))
        .text_color(text)
        .on_click(move |_event, window, cx| on_pick(index, window, cx))
        .child(
            div()
                .w(px(14.0))
                .flex_none()
                .child(if row.checked { "\u{2713}" } else { "" }),
        )
        .child(
            div()
                .w(px(78.0))
                .flex_none()
                .text_size(px(11.5))
                .text_color(faint)
                .child(row.kind.clone()),
        )
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis()
                .when(row.link.is_none(), |this| {
                    this.font_weight(gpui::FontWeight::EXTRA_BOLD)
                })
                .child(row.text.clone()),
        )
}
