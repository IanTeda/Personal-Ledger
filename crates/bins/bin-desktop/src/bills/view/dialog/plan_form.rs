//! The Add and Edit bill plan Dialog (8c) on the shared `crate::dialog` chrome. One View serves
//! both: Edit shows the Unit locked and the Active checkbox, Add a Unit select and no Active. The
//! body scrolls past 480px, the handoff's own cap, since this form has more fields than any other
//! Dialog.
//!
//! Keys: `Tab`/`Shift-Tab` move between the fields shown; on a select `↑`/`↓` step it or move its
//! open list's highlight, and `Space` and `Enter` open or commit its list; `Space`, `←` and `→` flip
//! Fixed/Estimated and `Space` flips Active. `Enter` elsewhere confirms once the form would.

use gpui::{AnyElement, App, SharedString, div, prelude::*, px};
use lib_bills::presenter::{PlanField, PlanFormEvent, PlanFormPresenter, PlanFormViewModel};
use lib_viper::view::{SelectView, TextFieldView};

use super::{BillsDialog, Clicks, View, send};
use crate::{
    chrome::dialog_host::{DialogKey, DialogOutcome},
    dialog,
    form::{
        dialog_fields::{label, suffixed_label, text_field, two_up},
        fields::segmented_labelled,
        select::SelectState,
        select_field::{self, SelectFieldProps},
        viper::{
            choose_option, chosen_label, enter, labels, select_key, synced, toggle_select, typed,
        },
    },
    theme::color,
};

/// The Dialog's width: 8c's, wider than the other Dialogs for its field count.
const WIDTH: gpui::Pixels = px(480.0);
/// The body's scroll cap.
const BODY_MAX_HEIGHT: gpui::Pixels = px(480.0);

/// The open state of the form's five selects.
#[derive(Debug, Clone, Default)]
pub(super) struct PlanSelects {
    category: SelectState,
    unit: SelectState,
    account: SelectState,
    payee: SelectState,
    recurrence: SelectState,
}

impl PlanSelects {
    /// Closes every list. Returns whether one was open.
    pub(super) fn close(&mut self) -> bool {
        let open = self.any_open();
        for state in self.all_mut() {
            state.cancel();
        }
        open
    }

    fn all_mut(&mut self) -> [&mut SelectState; 5] {
        [
            &mut self.category,
            &mut self.unit,
            &mut self.account,
            &mut self.payee,
            &mut self.recurrence,
        ]
    }

    fn any_open(&self) -> bool {
        [
            &self.category,
            &self.unit,
            &self.account,
            &self.payee,
            &self.recurrence,
        ]
        .iter()
        .any(|state| state.is_open())
    }
}

fn handle(presenter: &mut PlanFormPresenter, event: PlanFormEvent) {
    send(presenter, event, PlanFormPresenter::handle);
}

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

/// A select key on `field`'s select: steps or opens its list, and sends a new choice.
fn select(
    presenter: &mut PlanFormPresenter,
    view: &PlanFormViewModel,
    selects: &mut PlanSelects,
    field: PlanField,
    key: DialogKey,
) -> bool {
    match field {
        PlanField::Category => pick(
            presenter,
            &mut selects.category,
            &view.category,
            key,
            PlanFormEvent::Category,
        ),
        PlanField::Unit if view.unit_locked.is_none() => pick(
            presenter,
            &mut selects.unit,
            &view.unit,
            key,
            PlanFormEvent::Unit,
        ),
        PlanField::Account => pick(
            presenter,
            &mut selects.account,
            &view.account,
            key,
            PlanFormEvent::Account,
        ),
        PlanField::Payee => pick(
            presenter,
            &mut selects.payee,
            &view.payee,
            key,
            PlanFormEvent::Payee,
        ),
        PlanField::Recurrence => pick(
            presenter,
            &mut selects.recurrence,
            &view.recurrence,
            key,
            PlanFormEvent::Recurrence,
        ),
        _ => false,
    }
}

fn pick<T: Copy + PartialEq>(
    presenter: &mut PlanFormPresenter,
    state: &mut SelectState,
    view: &SelectView<T>,
    key: DialogKey,
    event: fn(T) -> PlanFormEvent,
) -> bool {
    let (taken, value) = select_key(state, view, key);
    if let Some(value) = value {
        handle(presenter, event(value));
    }
    taken
}

/// Whether `field` is a select that takes keys: the Unit's is locked on Edit.
fn is_select(view: &PlanFormViewModel, field: Option<PlanField>) -> bool {
    match field {
        Some(
            PlanField::Category | PlanField::Account | PlanField::Payee | PlanField::Recurrence,
        ) => true,
        Some(PlanField::Unit) => view.unit_locked.is_none(),
        _ => false,
    }
}

fn text_of(view: &PlanFormViewModel, field: PlanField) -> Option<&TextFieldView> {
    match field {
        PlanField::Name => Some(&view.name),
        PlanField::Amount => Some(&view.amount),
        PlanField::FirstDue => Some(&view.first_due),
        PlanField::EndsOn => view.ends_on.as_ref(),
        PlanField::AttentionLead => Some(&view.lead),
        _ => None,
    }
}

fn text_event(field: PlanField, text: String) -> Option<PlanFormEvent> {
    Some(match field {
        PlanField::Name => PlanFormEvent::Name(text),
        PlanField::Amount => PlanFormEvent::Amount(text),
        PlanField::FirstDue => PlanFormEvent::FirstDue(text),
        PlanField::EndsOn => PlanFormEvent::EndsOn(text),
        PlanField::AttentionLead => PlanFormEvent::Lead(text),
        _ => return None,
    })
}

pub(super) fn key(
    presenter: &mut PlanFormPresenter,
    selects: &mut PlanSelects,
    key: DialogKey,
) -> DialogOutcome {
    let view = presenter.view_model();
    let field = focused(&view);
    match key {
        // `Tab` commits an open list's highlight before moving on.
        DialogKey::Tab | DialogKey::BackTab => {
            if let Some(at) = field
                && selects.any_open()
            {
                select(presenter, &view, selects, at, DialogKey::Enter);
            }
            selects.close();
            handle(
                presenter,
                if key == DialogKey::Tab {
                    PlanFormEvent::NextField
                } else {
                    PlanFormEvent::PrevField
                },
            );
            return DialogOutcome::Handled;
        }
        DialogKey::Enter if !is_select(&view, field) => {
            return enter(view.frame.can_confirm);
        }
        _ => {}
    }
    let Some(field) = field else {
        return DialogOutcome::Ignored;
    };
    if is_select(&view, Some(field))
        && matches!(
            key,
            DialogKey::Up | DialogKey::Down | DialogKey::Enter | DialogKey::Char(' ')
        )
    {
        select(presenter, &view, selects, field, key);
        return DialogOutcome::Handled;
    }
    match (field, key) {
        (PlanField::AmountKind, DialogKey::Left | DialogKey::Right | DialogKey::Char(' ')) => {
            handle(presenter, PlanFormEvent::ToggleAmountKind);
        }
        (PlanField::Active, DialogKey::Char(' ')) => handle(presenter, PlanFormEvent::ToggleActive),
        _ => {
            let Some(text) = text_of(&view, field) else {
                // The selects and toggles take no text.
                return match key {
                    DialogKey::Char(_) | DialogKey::Backspace => DialogOutcome::Handled,
                    _ => DialogOutcome::Ignored,
                };
            };
            let Some(event) = typed(&text.text, key).and_then(|text| text_event(field, text))
            else {
                return DialogOutcome::Ignored;
            };
            handle(presenter, event);
        }
    }
    DialogOutcome::Handled
}

/// A click on a field: focuses it, and opens or closes a select's list.
fn click_field(dialog: &mut BillsDialog, field: PlanField) {
    let View::PlanForm(presenter, selects) = &mut dialog.view else {
        return;
    };
    if field == PlanField::Active {
        handle(presenter, PlanFormEvent::ToggleActive);
        return;
    }
    handle(presenter, PlanFormEvent::Focus(field));
    let view = presenter.view_model();
    let state = match field {
        PlanField::Category => &selects.category,
        PlanField::Unit if view.unit_locked.is_none() => &selects.unit,
        PlanField::Account => &selects.account,
        PlanField::Payee => &selects.payee,
        PlanField::Recurrence => &selects.recurrence,
        _ => {
            selects.close();
            return;
        }
    };
    // Only the clicked select's list opens; a second click closes it.
    let was_open = state.is_open();
    selects.close();
    if was_open {
        return;
    }
    match field {
        PlanField::Category => toggle_select(&mut selects.category, &view.category),
        PlanField::Unit => toggle_select(&mut selects.unit, &view.unit),
        PlanField::Account => toggle_select(&mut selects.account, &view.account),
        PlanField::Payee => toggle_select(&mut selects.payee, &view.payee),
        PlanField::Recurrence => toggle_select(&mut selects.recurrence, &view.recurrence),
        _ => {}
    }
}

/// A click on row `index` of `field`'s open list.
fn click_option(dialog: &mut BillsDialog, field: PlanField, index: usize) {
    let View::PlanForm(presenter, selects) = &mut dialog.view else {
        return;
    };
    let view = presenter.view_model();
    let event = match field {
        PlanField::Category => {
            choose_option(&mut selects.category, &view.category, index).map(PlanFormEvent::Category)
        }
        PlanField::Unit => {
            choose_option(&mut selects.unit, &view.unit, index).map(PlanFormEvent::Unit)
        }
        PlanField::Account => {
            choose_option(&mut selects.account, &view.account, index).map(PlanFormEvent::Account)
        }
        PlanField::Payee => {
            choose_option(&mut selects.payee, &view.payee, index).map(PlanFormEvent::Payee)
        }
        PlanField::Recurrence => choose_option(&mut selects.recurrence, &view.recurrence, index)
            .map(PlanFormEvent::Recurrence),
        _ => None,
    };
    if let Some(event) = event {
        handle(presenter, event);
    }
}

pub(super) fn render(
    view: &PlanFormViewModel,
    selects: &PlanSelects,
    actions: &Clicks,
    cx: &App,
) -> AnyElement {
    let editing = view.active.is_some();
    let prefix = if editing {
        "edit-bill-plan"
    } else {
        "add-bill-plan"
    };
    let click =
        |field: PlanField| BillsDialog::on_click(actions, move |dialog| click_field(dialog, field));
    let option = |field: PlanField| {
        BillsDialog::on_option(actions, move |dialog, index| {
            click_option(dialog, field, index)
        })
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
        state: &SelectState,
        read_only: Option<SharedString>,
        on_field_click: dialog::OnClick,
        on_option_click: select_field::OnOptionClick,
        cx: &App,
    ) -> AnyElement {
        let options = labels(view);
        select_field::render(
            SelectFieldProps {
                id,
                label: view.label.clone().into(),
                options: &options,
                state: &synced(state, view),
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
                &selects.category,
                None,
                click(PlanField::Category),
                option(PlanField::Category),
                cx,
            ),
            select_element(
                "bill-plan-unit",
                &view.unit,
                &selects.unit,
                view.unit_locked.clone().map(SharedString::from),
                click(PlanField::Unit),
                option(PlanField::Unit),
                cx,
            ),
        ]),
        two_up([
            select_element(
                "bill-plan-account",
                &view.account,
                &selects.account,
                view.no_accounts.clone().map(SharedString::from),
                click(PlanField::Account),
                option(PlanField::Account),
                cx,
            ),
            select_element(
                "bill-plan-payee",
                &view.payee,
                &selects.payee,
                None,
                click(PlanField::Payee),
                option(PlanField::Payee),
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
                                actions.choose(|dialog: &mut BillsDialog, kind| {
                                    if let View::PlanForm(presenter, _) = &mut dialog.view {
                                        handle(presenter, PlanFormEvent::AmountKind(kind));
                                    }
                                }),
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
                &selects.recurrence,
                None,
                click(PlanField::Recurrence),
                option(PlanField::Recurrence),
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

pub(super) fn readback(
    view: &PlanFormViewModel,
    selects: &PlanSelects,
) -> Vec<(&'static str, String)> {
    vec![
        ("name", view.name.text.clone()),
        (
            "field",
            focused(view)
                .map(|field| format!("{field:?}"))
                .unwrap_or_default(),
        ),
        ("select_open", selects.any_open().to_string()),
        ("unit", chosen_label(&view.unit).unwrap_or_default()),
        ("account", chosen_label(&view.account).unwrap_or_default()),
        ("amount", view.amount.text.clone()),
        ("refusal", view.refusal.clone().unwrap_or_default()),
    ]
}
