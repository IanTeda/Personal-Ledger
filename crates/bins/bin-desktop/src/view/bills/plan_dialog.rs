//! Renders the **Add bill plan** and **Edit bill plan** dialogs (`docs/ux/desktop-mockups/12-bills/README.md`'s
//! 8c) on the shared `crate::dialog` chrome. One renderer serves both: Edit shows the Unit locked
//! and the Active checkbox, Add a Unit select and no Active. The body scrolls past 480px, the
//! handoff's own cap, since this form has more fields than any other dialog. `Shell` owns the live
//! form (`bills::form::BillPlanForm`) and every keystroke while it is open.

use std::rc::Rc;

use gpui::{AnyElement, App, SharedString, Window, div, prelude::*, px};

use crate::{
    bills::form::{BillPlanField, BillPlanForm, BillPlanOptions, FieldError, FormErrors},
    bills::{AmountKind, BillError},
    dialog,
    theme::color,
    view::{
        accounts::{
            add_dialog::{label, suffixed_label, text_field, two_up},
            select_field::{self, SelectFieldProps},
        },
        form_fields::segmented_control,
    },
};

/// The dialog's width: 8c's, wider than the other dialogs for its field count.
pub const WIDTH: gpui::Pixels = px(480.0);
/// The body's scroll cap.
const BODY_MAX_HEIGHT: gpui::Pixels = px(480.0);
const AMOUNT_KINDS: [AmountKind; 2] = [AmountKind::Fixed, AmountKind::Estimated];

pub type OnFieldClick = Rc<dyn Fn(BillPlanField, &mut Window, &mut App)>;
pub type OnOptionClick = Rc<dyn Fn(BillPlanField, usize, &mut Window, &mut App)>;
pub type OnAmountKindClick = Rc<dyn Fn(AmountKind, &mut Window, &mut App)>;

pub struct PlanDialogHandlers {
    pub on_field_click: OnFieldClick,
    pub on_option_click: OnOptionClick,
    pub on_amount_kind_click: OnAmountKindClick,
    pub on_cancel: dialog::OnClick,
    pub on_confirm: dialog::OnClick,
}

pub struct PlanDialogProps<'a> {
    /// The stored name of the Plan being edited, which the title keeps while Name is retyped;
    /// `None` for Add.
    pub editing: Option<&'a str>,
    pub form: &'a BillPlanForm,
    pub options: &'a BillPlanOptions,
    pub errors: FormErrors,
    pub valid: bool,
    pub handlers: PlanDialogHandlers,
}

fn amount_kind_label(kind: AmountKind) -> String {
    match kind {
        AmountKind::Fixed => crate::msg::desktop_bills_amount_fixed(),
        AmountKind::Estimated => crate::msg::desktop_bills_amount_estimated(),
    }
}

pub fn render(props: PlanDialogProps<'_>, cx: &App) -> AnyElement {
    let PlanDialogProps {
        editing,
        form,
        options,
        errors,
        valid,
        handlers,
    } = props;
    let PlanDialogHandlers {
        on_field_click,
        on_option_click,
        on_amount_kind_click,
        on_cancel,
        on_confirm,
    } = handlers;
    let focused = |field: BillPlanField| form.focused == field;
    let click = |field: BillPlanField| -> dialog::OnClick {
        let on_field_click = on_field_click.clone();
        Rc::new(move |window: &mut Window, cx: &mut App| on_field_click(field, window, cx))
    };
    let option_click = |field: BillPlanField| -> select_field::OnOptionClick {
        let on_option_click = on_option_click.clone();
        Rc::new(move |index: usize, window: &mut Window, cx: &mut App| {
            on_option_click(field, index, window, cx)
        })
    };
    let select = |id: &'static str,
                  label: SharedString,
                  field: BillPlanField,
                  list: &[String],
                  state: &crate::form::select::SelectState,
                  read_only: Option<SharedString>| {
        select_field::render(
            SelectFieldProps {
                id,
                label,
                options: list,
                state,
                focused: focused(field),
                accent: false,
                read_only,
                on_field_click: click(field),
                on_option_click: option_click(field),
            },
            cx,
        )
    };
    let with_error = |field: AnyElement, error: Option<String>| -> AnyElement {
        div()
            .flex_1()
            .min_w(px(0.0))
            .child(field)
            .children(error.map(|text| error_line(text, cx)))
            .into_any_element()
    };

    let (id, title, submit) = match editing {
        None => (
            "add-bill-plan",
            crate::msg::desktop_bills_plan_add_title(),
            crate::msg::desktop_bills_plan_add_submit(),
        ),
        Some(name) => (
            "edit-bill-plan",
            crate::msg::desktop_bills_plan_edit_title(name),
            crate::msg::desktop_bills_plan_edit_submit(),
        ),
    };
    let element_id = |suffix: &str| SharedString::from(format!("{id}-{suffix}"));
    let unit = form.unit_code().unwrap_or("").to_uppercase();

    let mut fields = vec![
        text_field(
            "bill-plan-name",
            label(lib_locale::msg::column_name()),
            form.name.text(),
            &crate::msg::desktop_bills_plan_name_placeholder(),
            focused(BillPlanField::Name),
            click(BillPlanField::Name),
            cx,
        ),
        two_up([
            select(
                "bill-plan-category",
                crate::msg::desktop_bills_plan_field_category().into(),
                BillPlanField::Category,
                &options.category_labels,
                &form.category,
                None,
            ),
            select(
                "bill-plan-unit",
                lib_locale::msg::column_unit().into(),
                BillPlanField::Unit,
                &options.units,
                &form.unit,
                form.is_edit.then(|| unit.clone().into()),
            ),
        ]),
        two_up([
            select(
                "bill-plan-account",
                crate::msg::desktop_bills_column_account().into(),
                BillPlanField::Account,
                &options.account_labels,
                &form.account,
                options
                    .account_labels
                    .is_empty()
                    .then(|| crate::msg::desktop_bills_plan_no_accounts().into()),
            ),
            select(
                "bill-plan-payee",
                format!(
                    "{} {}",
                    crate::msg::desktop_bills_plan_field_payee(),
                    crate::msg::desktop_field_optional()
                )
                .into(),
                BillPlanField::Payee,
                &options.payee_labels,
                &form.payee,
                None,
            ),
        ]),
        div()
            .flex()
            .items_start()
            .gap(px(16.0))
            .child(with_error(
                text_field(
                    "bill-plan-amount",
                    label(crate::msg::desktop_bills_plan_field_amount()),
                    form.amount.text(),
                    "0.00",
                    focused(BillPlanField::Amount),
                    click(BillPlanField::Amount),
                    cx,
                ),
                errors.amount.map(field_error),
            ))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .child(label(crate::msg::desktop_bills_plan_field_amount_kind()))
                    .child(
                        div()
                            .flex()
                            .when(focused(BillPlanField::AmountKind), |this| {
                                this.border_1().border_color(color::accent(cx))
                            })
                            .child(segmented_control(
                                "bill-plan-amount-kind",
                                &AMOUNT_KINDS,
                                form.amount_kind,
                                amount_kind_label,
                                on_amount_kind_click,
                                cx,
                            )),
                    ),
            )
            .into_any_element(),
    ];

    let recurring = form.shows(BillPlanField::EndsOn);
    fields.push(
        div()
            .flex()
            .items_start()
            .gap(px(16.0))
            .child(select(
                "bill-plan-recurrence",
                crate::msg::desktop_bills_plan_field_recurrence().into(),
                BillPlanField::Recurrence,
                &options.recurrence_labels,
                &form.recurrence,
                None,
            ))
            .child(with_error(
                text_field(
                    "bill-plan-first-due",
                    label(crate::msg::desktop_bills_plan_field_first_due()),
                    form.first_due.text(),
                    &lib_locale::msg::date_word_today(),
                    focused(BillPlanField::FirstDue),
                    click(BillPlanField::FirstDue),
                    cx,
                ),
                errors.first_due.clone(),
            ))
            .when(recurring, |this| {
                this.child(with_error(
                    text_field(
                        "bill-plan-ends-on",
                        label(crate::msg::desktop_bills_plan_field_ends_on()),
                        form.ends_on.text(),
                        &crate::msg::desktop_bills_plan_ends_on_placeholder(),
                        focused(BillPlanField::EndsOn),
                        click(BillPlanField::EndsOn),
                        cx,
                    ),
                    errors.ends_on.clone().or_else(|| {
                        errors
                            .ends_before_first_due
                            .then(crate::msg::desktop_bills_plan_error_ends_before_first_due)
                    }),
                ))
            })
            .into_any_element(),
    );
    fields.push(
        div()
            .child(suffixed_label(
                crate::msg::desktop_bills_plan_field_lead(),
                crate::msg::desktop_bills_plan_lead_explainer(),
                cx,
            ))
            .child(div().w(px(140.0)).child(text_field(
                "bill-plan-lead",
                div().into_any_element(),
                form.attention_lead.text(),
                &crate::msg::desktop_bills_plan_lead_placeholder(),
                focused(BillPlanField::AttentionLead),
                click(BillPlanField::AttentionLead),
                cx,
            )))
            .children(errors.lead.map(|error| error_line(field_error(error), cx)))
            .into_any_element(),
    );
    if form.is_edit {
        fields.push(checkbox(
            form.is_active,
            focused(BillPlanField::Active),
            click(BillPlanField::Active),
            cx,
        ));
    }
    if let Some(error) = &form.error {
        fields.push(error_line(submit_error(error), cx));
    }

    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(title, false, cx))
        .child(
            div()
                .id(element_id("body"))
                .max_h(BODY_MAX_HEIGHT)
                .overflow_y_scroll()
                .child(dialog::body(fields)),
        )
        .child(dialog::action_row(
            [
                dialog::cancel_button(element_id("cancel"), on_cancel, cx).into_any_element(),
                dialog::confirm_button(element_id("confirm"), submit, valid, false, on_confirm, cx)
                    .into_any_element(),
            ],
            cx,
        ));

    dialog::overlay(WIDTH, false, card, cx)
}

/// "Active — generate future Schedule rows": a 14px box, filled while checked.
fn checkbox(checked: bool, focused: bool, on_click: dialog::OnClick, cx: &App) -> AnyElement {
    div()
        .id("bill-plan-active")
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
        .child(crate::msg::desktop_bills_plan_active())
        .into_any_element()
}

fn field_error(error: FieldError) -> String {
    match error {
        FieldError::AmountNotPositive => crate::msg::desktop_bills_plan_error_amount(),
        FieldError::LeadNotWholeDays => crate::msg::desktop_bills_plan_error_lead(),
    }
}

/// A refused Save. The form's own checks catch most of these first; what's left is a stub the
/// selects couldn't fill (no expense Category, no Account in the Unit).
fn submit_error(error: &BillError) -> String {
    match error {
        BillError::NameRequired => crate::msg::desktop_bills_plan_error_name(),
        BillError::CategoryNotExpenseLeaf => crate::msg::desktop_bills_plan_error_category(),
        BillError::AccountRequired | BillError::UnitMismatch => {
            crate::msg::desktop_bills_plan_error_account()
        }
        BillError::AmountNotPositive => crate::msg::desktop_bills_plan_error_amount(),
        BillError::EndsBeforeFirstDue => {
            crate::msg::desktop_bills_plan_error_ends_before_first_due()
        }
        // Not reachable from the form: One-shot hides Ends On, Edit locks the Unit, and the rest
        // come from settlement.
        other => other.to_string(),
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
