//! Renders the **Edit budget** dialog (`docs/ux/desktop/Budgets_v2/limits-9a-9g.md`'s 9e) on the
//! shared `crate::dialog` chrome at the handoff's 480px: the Category picker when opened from
//! **+ Budget a category**, Amount per month, Starting, the Applies to and Rollover segmented
//! controls, the before/after summary and the note. `Shell` owns the live form
//! (`limit_form::LimitForm`) and every keystroke while it is open.

use std::rc::Rc;

use gpui::{AnyElement, App, SharedString, Window, div, prelude::*, px};
use lib_core::Money;

use crate::{
    budgets::{BudgetError, Rollover, Span},
    dialog,
    format::amount,
    limit_form::{LimitField, LimitForm, LimitOptions, LimitPreview, ROLLOVERS, SPANS},
    theme::color,
    transaction_rows::EMPTY_CELL,
    view::accounts::{
        add_dialog::{label, text_field},
        select_field::{self, SelectFieldProps},
    },
};

/// The dialog's width: the handoff's 9e.
pub const WIDTH: gpui::Pixels = px(480.0);

pub type OnFieldClick = Rc<dyn Fn(LimitField, &mut Window, &mut App)>;
/// Called with the select's field and the clicked row.
pub type OnOptionClick = Rc<dyn Fn(LimitField, usize, &mut Window, &mut App)>;
pub type OnSpanClick = Rc<dyn Fn(Span, &mut Window, &mut App)>;
pub type OnRolloverClick = Rc<dyn Fn(Rollover, &mut Window, &mut App)>;

pub struct LimitDialogHandlers {
    pub on_field_click: OnFieldClick,
    pub on_option_click: OnOptionClick,
    pub on_span_click: OnSpanClick,
    pub on_rollover_click: OnRolloverClick,
    /// `Stop budgeting…`, offered while editing a budgeted Category.
    pub on_stop: Option<dialog::OnClick>,
    pub on_cancel: dialog::OnClick,
    pub on_confirm: dialog::OnClick,
}

pub struct LimitDialogProps<'a> {
    /// The fixed Category's name; `None` while the picker chooses it.
    pub category: Option<String>,
    pub form: &'a LimitForm,
    pub options: &'a LimitOptions,
    pub preview: Option<&'a LimitPreview>,
    /// Already formatted: the month left unchanged, the starting month.
    pub unchanged_month: Option<String>,
    pub month: String,
    /// The Budget is the one Categories 5c's Monthly budget field reads and writes.
    pub mirrors_category_field: bool,
    pub valid: bool,
    pub handlers: LimitDialogHandlers,
}

fn span_label(span: Span) -> String {
    match span {
        Span::MonthOnly => crate::msg::desktop_budgets_limit_span_month_only(),
        Span::Onward => crate::msg::desktop_budgets_limit_span_onward(),
    }
}

fn rollover_label(rollover: Rollover) -> String {
    match rollover {
        Rollover::None => crate::msg::desktop_budgets_limit_rollover_none(),
        Rollover::CarryUnspent => crate::msg::desktop_budgets_limit_rollover_unspent(),
        Rollover::CarryBoth => crate::msg::desktop_budgets_limit_rollover_both(),
    }
}

pub fn error_text(error: &BudgetError) -> String {
    match error {
        BudgetError::Archived => crate::msg::desktop_budgets_limit_error_archived(),
        BudgetError::NegativeAmount | BudgetError::InvalidAmount => {
            crate::msg::desktop_budgets_limit_error_amount()
        }
        _ => crate::msg::desktop_budgets_limit_error_refused(),
    }
}

pub fn error_line(text: String, cx: &App) -> AnyElement {
    div()
        .mt(px(6.0))
        .text_size(px(11.5))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_color(color::accent_text(cx))
        .child(text)
        .into_any_element()
}

/// A labelled `.seg`: the chosen cell filled dark, an accent border while focused. Each cell is
/// its text, whether it is the chosen one, and its click.
fn segmented(
    id: &'static str,
    title: String,
    focused: bool,
    cells: Vec<(String, bool, dialog::OnClick)>,
    cx: &App,
) -> AnyElement {
    div()
        .child(label(title))
        .child(
            div()
                .flex()
                .border_1()
                .border_color(if focused {
                    color::accent(cx)
                } else {
                    color::border(cx)
                })
                .children(cells.into_iter().enumerate().map(
                    |(index, (text, chosen, on_click))| {
                        div()
                            .id(SharedString::from(format!("{id}-{index}")))
                            .cursor_pointer()
                            .flex_1()
                            .py(px(8.0))
                            .px(px(10.0))
                            .text_size(px(12.5))
                            .text_align(gpui::TextAlign::Center)
                            .font_weight(gpui::FontWeight::EXTRA_BOLD)
                            .when(chosen, |this| {
                                this.bg(color::foreground(cx))
                                    .text_color(color::background(cx))
                            })
                            .on_click(move |_event, window, cx| on_click(window, cx))
                            .child(text)
                    },
                )),
        )
        .into_any_element()
}

fn money(value: &Money) -> String {
    amount(value).1
}

/// One summary line: what it is on the left, the figures on the right.
fn preview_line(what: String, figures: String, strong: bool, cx: &App) -> AnyElement {
    div()
        .flex()
        .justify_between()
        .gap(px(12.0))
        .text_size(px(12.5))
        .child(div().text_color(color::muted(cx)).child(what))
        .child(
            div()
                .when(strong, |this| {
                    this.font_weight(gpui::FontWeight::EXTRA_BOLD)
                })
                .child(figures),
        )
        .into_any_element()
}

fn preview_panel(
    preview: &LimitPreview,
    unchanged_month: Option<String>,
    month: &str,
    cx: &App,
) -> AnyElement {
    let or_dash = |value: Option<&Money>| value.map_or_else(|| EMPTY_CELL.to_string(), money);
    let mut lines = Vec::new();
    if let (Some((_, was)), Some(label)) = (&preview.unchanged, unchanged_month) {
        lines.push(preview_line(
            label,
            format!(
                "{} {}",
                or_dash(was.as_ref()),
                crate::msg::desktop_budgets_limit_preview_unchanged()
            ),
            false,
            cx,
        ));
    }
    lines.push(preview_line(
        match preview.span {
            Span::Onward => crate::msg::desktop_budgets_limit_preview_onward(month),
            Span::MonthOnly => crate::msg::desktop_budgets_limit_preview_month_only(month),
        },
        crate::msg::desktop_budgets_limit_preview_change(
            &or_dash(preview.before.as_ref()),
            &money(&preview.after),
        ),
        true,
        cx,
    ));
    lines.push(preview_line(
        crate::msg::desktop_budgets_limit_preview_total(month),
        crate::msg::desktop_budgets_limit_preview_change(
            &money(&preview.total_before),
            &money(&preview.total_after),
        ),
        false,
        cx,
    ));
    div()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .p(px(12.0))
        .border_1()
        .border_color(color::border(cx))
        .children(lines)
        .into_any_element()
}

pub fn render(props: LimitDialogProps<'_>, cx: &App) -> AnyElement {
    let LimitDialogProps {
        category,
        form,
        options,
        preview,
        unchanged_month,
        month,
        mirrors_category_field,
        valid,
        handlers,
    } = props;
    let LimitDialogHandlers {
        on_field_click,
        on_option_click,
        on_span_click,
        on_rollover_click,
        on_stop,
        on_cancel,
        on_confirm,
    } = handlers;
    let field_click = |field: LimitField| -> dialog::OnClick {
        let on_click = on_field_click.clone();
        Rc::new(move |window: &mut Window, cx: &mut App| on_click(field, window, cx))
    };
    let option_click = |field: LimitField| -> select_field::OnOptionClick {
        let on_click = on_option_click.clone();
        Rc::new(move |index, window: &mut Window, cx: &mut App| on_click(field, index, window, cx))
    };

    let title = category
        .as_ref()
        .map_or_else(crate::msg::desktop_budgets_limit_title_new, |name| {
            crate::msg::desktop_budgets_limit_title(name)
        });
    let mut fields = Vec::new();
    if form.is_pick() {
        if options.category_labels.is_empty() {
            fields.push(
                dialog::info_panel(crate::msg::desktop_budgets_limit_no_categories(), cx)
                    .into_any_element(),
            );
        } else {
            fields.push(select_field::render(
                SelectFieldProps {
                    id: "budgets-limit-category",
                    label: crate::msg::desktop_budgets_limit_field_category().into(),
                    options: &options.category_labels,
                    state: &form.category,
                    focused: form.focused == LimitField::Category,
                    accent: false,
                    read_only: None,
                    on_field_click: field_click(LimitField::Category),
                    on_option_click: option_click(LimitField::Category),
                },
                cx,
            ));
        }
    }
    fields.push(
        div()
            .child(text_field(
                "budgets-limit-amount",
                label(crate::msg::desktop_budgets_limit_field_amount()),
                &form.amount,
                "0.00",
                form.focused == LimitField::Amount,
                field_click(LimitField::Amount),
                cx,
            ))
            .children(
                form.amount_invalid()
                    .then(|| error_line(crate::msg::desktop_budgets_limit_error_amount(), cx)),
            )
            .into_any_element(),
    );
    fields.push(select_field::render(
        SelectFieldProps {
            id: "budgets-limit-starting",
            label: crate::msg::desktop_budgets_limit_field_starting().into(),
            options: &options.month_labels,
            state: &form.starting,
            focused: form.focused == LimitField::Starting,
            accent: false,
            read_only: None,
            on_field_click: field_click(LimitField::Starting),
            on_option_click: option_click(LimitField::Starting),
        },
        cx,
    ));
    fields.push(segmented(
        "budgets-limit-span",
        crate::msg::desktop_budgets_limit_field_span(),
        form.focused == LimitField::Span,
        SPANS
            .into_iter()
            .map(|span| {
                let on_click = on_span_click.clone();
                let click: dialog::OnClick =
                    Rc::new(move |window: &mut Window, cx: &mut App| on_click(span, window, cx));
                (span_label(span), span == form.span, click)
            })
            .collect(),
        cx,
    ));
    fields.push(segmented(
        "budgets-limit-rollover",
        crate::msg::desktop_budgets_limit_field_rollover(),
        form.focused == LimitField::Rollover,
        ROLLOVERS
            .into_iter()
            .map(|rollover| {
                let on_click = on_rollover_click.clone();
                let click: dialog::OnClick = Rc::new(move |window: &mut Window, cx: &mut App| {
                    on_click(rollover, window, cx);
                });
                (rollover_label(rollover), rollover == form.rollover, click)
            })
            .collect(),
        cx,
    ));
    if let Some(preview) = preview {
        fields.push(preview_panel(preview, unchanged_month, &month, cx));
    }
    fields.push(
        div()
            .text_size(px(11.5))
            .text_color(color::muted(cx))
            .child(if mirrors_category_field {
                crate::msg::desktop_budgets_limit_note()
            } else {
                crate::msg::desktop_budgets_limit_note_other()
            })
            .into_any_element(),
    );
    if let Some(error) = form.error.as_ref() {
        fields.push(error_line(error_text(error), cx));
    }

    let mut buttons = Vec::new();
    if let Some(on_stop) = on_stop {
        buttons.push(
            div()
                .id("budgets-limit-stop")
                .cursor_pointer()
                .py(px(8.0))
                .px(px(16.0))
                .mr_auto()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_color(color::foreground(cx))
                .on_click(move |_event, window, cx| on_stop(window, cx))
                .child(crate::msg::desktop_budgets_limit_stop())
                .into_any_element(),
        );
    }
    buttons.push(dialog::cancel_button("budgets-limit-cancel", on_cancel, cx).into_any_element());
    buttons.push(
        dialog::confirm_button(
            "budgets-limit-confirm",
            crate::msg::desktop_budgets_limit_submit(),
            valid,
            false,
            on_confirm,
            cx,
        )
        .into_any_element(),
    );

    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(title, false, cx))
        .child(dialog::body(fields))
        .child(dialog::action_row(buttons, cx));
    dialog::overlay(WIDTH, false, card, cx)
}
