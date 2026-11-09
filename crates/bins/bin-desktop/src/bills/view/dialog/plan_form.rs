//! The Add and Edit bill plan Dialog (8c) on the shared `crate::dialog` frame. One View serves
//! both: Edit shows the Unit locked and the Active checkbox, Add a Unit select and no Active. The
//! body scrolls past 480px, the handoff's own cap, since this form has more fields than any other
//! Dialog.
//!
//! What each key means is the Presenter's (`lib_bills::presenter::PlanFormPresenter`); this View
//! draws the form and turns its clicks into Presenter events.

use gpui::{AnyElement, App, SharedString, div, prelude::*, px};
use lib_bills::presenter::{PlanField, PlanFormEvent, PlanFormPresenter, PlanFormViewModel};
use lib_shell::Hint;
use lib_viper::view::{Choice, SelectList, SelectView, TextFieldView};

use crate::shell::view::dialog_entity::{DialogView, ModuleDialogClicks, framed};
use crate::{
    dialog,
    form::{
        dialog_fields::{label, suffixed_label, text_field, two_up},
        fields::segmented_labelled,
        select_field::{self, OnOptionClick, SelectFieldProps},
        viper::{chosen_label, drawn, labels},
    },
    theme::color,
};

/// The open form's clicks.
type Clicks = ModuleDialogClicks<PlanFormPresenter>;

/// The `lib-bills` Plan form Presenter's [`DialogView`], hosted in a `ModuleDialog` (ADR-0056).
/// Its widget state is the top row of the open list's window, which the View keeps between draws.
impl DialogView for PlanFormPresenter {
    type Widgets = usize;

    /// Edit shows Active; Add doesn't.
    fn kind(view: &PlanFormViewModel) -> &'static str {
        if view.active.is_some() { "Edit" } else { "Add" }
    }

    fn render(
        view: &PlanFormViewModel,
        scroll: &mut usize,
        clicks: &Clicks,
        cx: &App,
    ) -> AnyElement {
        render(view, scroll, clicks, cx)
    }

    /// The focused field, the drafts, the selects, whether a list is open and the refusal.
    fn readback(view: &PlanFormViewModel) -> Vec<(&'static str, String)> {
        framed(&view.frame, readback(view))
    }

    fn hints(view: &PlanFormViewModel) -> Vec<Hint> {
        view.hints.clone()
    }
}

/// The Dialog's width: 8c's, wider than the other Dialogs for its field count.
const WIDTH: gpui::Pixels = px(480.0);
/// The body's scroll cap.
const BODY_MAX_HEIGHT: gpui::Pixels = px(480.0);

/// The field with focus.
fn focused(view: &PlanFormViewModel) -> Option<PlanField> {
    view.fields.iter().copied().find(|field| match field {
        PlanField::Name => view.name.focused,
        PlanField::Category => view.category.focused,
        PlanField::Unit => view.unit.focused,
        PlanField::Account => view.account.focused,
        PlanField::Payee => view.payee.focused,
        PlanField::Amount => view.amount.focused,
        PlanField::AmountKind => view.amount_kind.focused,
        PlanField::Recurrence => view.recurrence.focused,
        PlanField::FirstDue => view.first_due.focused,
        PlanField::EndsOn => view.ends_on.as_ref().is_some_and(|field| field.focused),
        PlanField::AttentionLead => view.lead.focused,
        PlanField::Active => view.active.as_ref().is_some_and(|active| active.focused),
    })
}

/// What a click on `field` sends: Active toggles, a select focuses and opens or closes its list,
/// a text field focuses.
fn field_click(field: PlanField) -> PlanFormEvent {
    match field {
        PlanField::Active => PlanFormEvent::ToggleActive,
        PlanField::Category
        | PlanField::Unit
        | PlanField::Account
        | PlanField::Payee
        | PlanField::Recurrence => PlanFormEvent::ToggleList(field),
        _ => PlanFormEvent::Focus(field),
    }
}

/// A click on row `index` of a select's open list: the event choosing that row's value. The
/// options are fixed while the Dialog is open.
fn option_click<T: Clone + 'static>(
    actions: &Clicks,
    options: &[Choice<T>],
    event: fn(T) -> PlanFormEvent,
) -> OnOptionClick {
    let values: Vec<T> = options.iter().map(|choice| choice.value.clone()).collect();
    actions.option(move |index| values.get(index).cloned().map(event).into_iter().collect())
}

/// The form, its open list's window scrolled from `scroll`, which this draw moves to follow the
/// highlight.
fn render(view: &PlanFormViewModel, scroll: &mut usize, actions: &Clicks, cx: &App) -> AnyElement {
    let editing = view.active.is_some();
    let prefix = if editing {
        "edit-bill-plan"
    } else {
        "add-bill-plan"
    };
    let click = |field: PlanField| actions.send(field_click(field));
    let option = |field: PlanField| match field {
        PlanField::Category => {
            option_click(actions, &view.category.options, PlanFormEvent::Category)
        }
        PlanField::Unit => option_click(actions, &view.unit.options, PlanFormEvent::Unit),
        PlanField::Account => option_click(actions, &view.account.options, PlanFormEvent::Account),
        PlanField::Payee => option_click(actions, &view.payee.options, PlanFormEvent::Payee),
        PlanField::Recurrence => {
            option_click(actions, &view.recurrence.options, PlanFormEvent::Recurrence)
        }
        _ => actions.option(|_| Vec::new()),
    };
    let text = |id: &'static str, field: &TextFieldView, at: PlanField, placeholder: &str| {
        text_field(
            id,
            label(field.label.clone()),
            &field.text,
            placeholder,
            field.focused,
            click(at),
            cx,
        )
    };
    fn select_element<T: PartialEq>(
        id: &'static str,
        view: &SelectView<T>,
        list: SelectList,
        scroll: &mut usize,
        read_only: Option<SharedString>,
        (on_field_click, on_option_click): (dialog::OnClick, OnOptionClick),
        cx: &App,
    ) -> AnyElement {
        let options = labels(view);
        // Only the focused select's list can be open, and only its window is kept.
        let list = if view.focused {
            list
        } else {
            SelectList::default()
        };
        let mut unused = 0;
        let scroll = if list.is_open() { scroll } else { &mut unused };
        select_field::render(
            SelectFieldProps {
                id: id.into(),
                label: view.label.clone().into(),
                options: &options,
                state: &drawn(view, list, scroll),
                focused: view.focused,
                accent: false,
                read_only,
                on_field_click,
                on_option_click,
            },
            cx,
        )
    }
    let with_error = |field: AnyElement, error: Option<&String>| -> AnyElement {
        div()
            .flex_1()
            .min_w(px(0.0))
            .child(field)
            .children(error.map(|text| error_line(text.clone(), cx)))
            .into_any_element()
    };

    let mut fields = vec![
        text(
            "bill-plan-name",
            &view.name,
            PlanField::Name,
            &view.name.placeholder,
        ),
        two_up([
            select_element(
                "bill-plan-category",
                &view.category,
                view.list,
                scroll,
                None,
                (click(PlanField::Category), option(PlanField::Category)),
                cx,
            ),
            select_element(
                "bill-plan-unit",
                &view.unit,
                view.list,
                scroll,
                view.unit_locked.clone().map(SharedString::from),
                (click(PlanField::Unit), option(PlanField::Unit)),
                cx,
            ),
        ]),
        two_up([
            select_element(
                "bill-plan-account",
                &view.account,
                view.list,
                scroll,
                view.no_accounts.clone().map(SharedString::from),
                (click(PlanField::Account), option(PlanField::Account)),
                cx,
            ),
            select_element(
                "bill-plan-payee",
                &view.payee,
                view.list,
                scroll,
                None,
                (click(PlanField::Payee), option(PlanField::Payee)),
                cx,
            ),
        ]),
        div()
            .flex()
            .items_start()
            .gap(px(16.0))
            .child(with_error(
                text(
                    "bill-plan-amount",
                    &view.amount,
                    PlanField::Amount,
                    &view.amount.placeholder,
                ),
                view.amount_error.as_ref(),
            ))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .child(label(view.amount_kind.label.clone()))
                    .child(
                        div()
                            .flex()
                            .when(view.amount_kind.focused, |this| {
                                this.border_1().border_color(color::accent(cx))
                            })
                            .child(segmented_labelled(
                                "bill-plan-amount-kind",
                                view.amount_kind
                                    .options
                                    .iter()
                                    .map(|choice| (choice.value, choice.label.clone().into()))
                                    .collect(),
                                view.amount_kind
                                    .chosen
                                    .unwrap_or(lib_bills::AmountKind::Fixed),
                                actions.choose(|kind| vec![PlanFormEvent::AmountKind(kind)]),
                                None,
                                cx,
                            )),
                    ),
            )
            .into_any_element(),
    ];

    fields.push(
        div()
            .flex()
            .items_start()
            .gap(px(16.0))
            .child(select_element(
                "bill-plan-recurrence",
                &view.recurrence,
                view.list,
                scroll,
                None,
                (click(PlanField::Recurrence), option(PlanField::Recurrence)),
                cx,
            ))
            .child(with_error(
                text(
                    "bill-plan-first-due",
                    &view.first_due,
                    PlanField::FirstDue,
                    &view.first_due.placeholder,
                ),
                view.first_due_error.as_ref(),
            ))
            .when_some(view.ends_on.as_ref(), |this, ends_on| {
                this.child(with_error(
                    text(
                        "bill-plan-ends-on",
                        ends_on,
                        PlanField::EndsOn,
                        &ends_on.placeholder,
                    ),
                    view.ends_on_error.as_ref(),
                ))
            })
            .into_any_element(),
    );
    fields.push(
        div()
            .child(suffixed_label(
                view.lead.label.clone(),
                view.lead_explainer.clone(),
                cx,
            ))
            .child(div().w(px(140.0)).child(text_field(
                "bill-plan-lead",
                div().into_any_element(),
                &view.lead.text,
                &view.lead.placeholder,
                view.lead.focused,
                click(PlanField::AttentionLead),
                cx,
            )))
            .children(
                view.lead_error
                    .as_ref()
                    .map(|error| error_line(error.clone(), cx)),
            )
            .into_any_element(),
    );
    if let Some(active) = &view.active {
        fields.push(checkbox(
            &active.label,
            active.checked,
            active.focused,
            click(PlanField::Active),
            cx,
        ));
    }
    if let Some(refusal) = &view.refusal {
        fields.push(error_line(refusal.clone(), cx));
    }

    let element_id = |suffix: &str| SharedString::from(format!("{prefix}-{suffix}"));
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(view.frame.title.clone(), false, cx))
        .child(
            div()
                .id(element_id("body"))
                .max_h(BODY_MAX_HEIGHT)
                .overflow_y_scroll()
                .child(dialog::body(fields)),
        )
        .child(dialog::action_row(
            [
                dialog::cancel_button(element_id("cancel"), actions.cancel.clone(), cx)
                    .into_any_element(),
                dialog::confirm_button(
                    element_id("confirm"),
                    view.frame.submit.clone(),
                    view.frame.can_confirm,
                    false,
                    actions.confirm.clone(),
                    cx,
                )
                .into_any_element(),
            ],
            cx,
        ));
    dialog::overlay(WIDTH, false, card, cx)
}

/// The Active checkbox: a 14px box, filled while checked.
fn checkbox(
    text: &str,
    checked: bool,
    focused: bool,
    on_click: dialog::OnClick,
    cx: &App,
) -> AnyElement {
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
        .child(text.to_string())
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

fn readback(view: &PlanFormViewModel) -> Vec<(&'static str, String)> {
    vec![
        ("name", view.name.text.clone()),
        (
            "field",
            focused(view)
                .map(|field| format!("{field:?}"))
                .unwrap_or_default(),
        ),
        ("select_open", view.list.is_open().to_string()),
        ("unit", chosen_label(&view.unit).unwrap_or_default()),
        ("account", chosen_label(&view.account).unwrap_or_default()),
        ("payee", chosen_label(&view.payee).unwrap_or_default()),
        ("amount", view.amount.text.clone()),
        ("refusal", view.refusal.clone().unwrap_or_default()),
    ]
}
