//! The Create ledger Dialog, in two stages under a step strip (#1344): Base unit, the chosen file's
//! name (read-only), then Code and Type, Name and Precision, as Add unit draws them; then Ledger,
//! the Ledger name, Owner and Financial year starts, each with its line of help. An Entity in the
//! Dialog host (`shell::view::dialog_entity`); every Dialog command and typed key reaches the
//! Presenter as a `lib-viper` `DialogInput` and every click as a Presenter event.

use std::rc::Rc;

use gpui::{AnyElement, App, SharedString, Window, div, prelude::*, px};
use lib_core::FinancialYearStart;
use lib_ledger_creation::presenter::{
    CreateLedgerEvent, CreateLedgerPresenter, CreateLedgerViewModel, LedgerField, LedgerStage,
};
use lib_shell::Hint;
use lib_units::UnitKind;
use lib_viper::view::{SelectList, SelectView, TextFieldView};

use crate::dialog::{self, OnClick};
use crate::form::dialog_fields::{label, optional_label, text_field, two_up};
use crate::form::select_field::{self, SelectFieldProps};
use crate::form::viper::{chosen_label, drawn, labels};
use crate::shell::view::dialog_entity::{DialogView, ModuleDialogClicks};
use crate::theme::color;
use crate::units::help;

type Clicks = ModuleDialogClicks<CreateLedgerPresenter>;

/// The open select's list scroll window, the widget state the View keeps beside the Presenter.
#[derive(Default)]
pub struct CreateLedgerWidgets {
    scroll: usize,
}

/// `lib-ledger-creation`'s [`CreateLedgerPresenter`] hosted in a `ModuleDialog`: a confirmed
/// create lands on the root Router, which opens the new Ledger.
impl DialogView for CreateLedgerPresenter {
    type Widgets = CreateLedgerWidgets;

    fn kind(_view: &CreateLedgerViewModel) -> &'static str {
        "CreateLedger"
    }

    fn render(
        view: &CreateLedgerViewModel,
        widgets: &mut CreateLedgerWidgets,
        clicks: &Clicks,
        cx: &App,
    ) -> AnyElement {
        render(view, &mut widgets.scroll, clicks, cx)
    }

    fn readback(view: &CreateLedgerViewModel) -> Vec<(&'static str, String)> {
        let focused = [
            (view.code.focused, "Code"),
            (view.kind.focused, "Type"),
            (view.name.focused, "Name"),
            (view.precision.focused, "Precision"),
            (view.ledger_name.focused, "Ledger name"),
            (view.owner.focused, "Owner"),
            (view.financial_year.focused, "Financial year starts"),
        ]
        .into_iter()
        .find_map(|(focused, name)| focused.then(|| name.to_string()))
        .unwrap_or_default();
        let stage = match view.stage {
            LedgerStage::BaseUnit => "BaseUnit",
            LedgerStage::Ledger => "Ledger",
        };
        vec![
            ("title", view.frame.title.clone()),
            ("stage", stage.to_string()),
            (
                "steps",
                view.steps
                    .iter()
                    .map(|(_, step)| step.as_str())
                    .collect::<Vec<_>>()
                    .join("|"),
            ),
            ("submit", view.frame.submit.clone()),
            ("file", view.file.text.clone()),
            ("code", view.code.text.clone()),
            ("name", view.name.text.clone()),
            ("precision", view.precision.text.clone()),
            ("kind", chosen_label(&view.kind).unwrap_or_default()),
            ("kinds", labels(&view.kind).join("|")),
            ("ledger_name", view.ledger_name.text.clone()),
            (
                "ledger_name_error",
                view.ledger_name_error.clone().unwrap_or_default(),
            ),
            ("owner", view.owner.text.clone()),
            (
                "financial_year",
                chosen_label(&view.financial_year).unwrap_or_default(),
            ),
            ("months", labels(&view.financial_year).join("|")),
            ("field", focused),
            ("select_open", view.list.is_open().to_string()),
            ("can_confirm", view.frame.can_confirm.to_string()),
        ]
    }

    /// The stage showing and, while the Ledger name is refused, why.
    fn hints(view: &CreateLedgerViewModel) -> Vec<Hint> {
        view.hints.clone()
    }
}

fn render(
    view: &CreateLedgerViewModel,
    scroll: &mut usize,
    actions: &Clicks,
    cx: &App,
) -> AnyElement {
    let body = match view.stage {
        LedgerStage::BaseUnit => base_unit_body(view, scroll, actions, cx),
        LedgerStage::Ledger => ledger_body(view, scroll, actions, cx),
    };
    let mut buttons = vec![
        dialog::cancel_button("create-ledger-cancel", actions.cancel.clone(), cx)
            .into_any_element(),
    ];
    if view.stage == LedgerStage::Ledger {
        buttons.push(secondary_button(
            "create-ledger-back",
            view.back.clone(),
            actions.send(CreateLedgerEvent::Back),
            cx,
        ));
    }
    buttons.push(
        dialog::confirm_button(
            "create-ledger-confirm",
            view.frame.submit.clone(),
            view.frame.can_confirm,
            false,
            actions.confirm.clone(),
            cx,
        )
        .into_any_element(),
    );
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(view.frame.title.clone(), false, cx))
        .child(steps(view, actions, cx))
        .child(dialog::body(body))
        .child(dialog::action_row(buttons, cx));
    dialog::overlay(dialog::WIDTH, false, card, cx)
}

/// The step strip under the title, the current stage's step marked: a click on a step sends it to
/// the Presenter, which goes Back to Base unit, or on to Ledger only when Next would.
fn steps(view: &CreateLedgerViewModel, actions: &Clicks, cx: &App) -> AnyElement {
    div()
        .flex()
        .gap(px(20.0))
        .px(px(20.0))
        .pt(px(12.0))
        .children(view.steps.iter().map(|(stage, step)| {
            let current = *stage == view.stage;
            let id: &'static str = match stage {
                LedgerStage::BaseUnit => "create-ledger-step-base-unit",
                LedgerStage::Ledger => "create-ledger-step-ledger",
            };
            let on_click = actions.send(CreateLedgerEvent::Step(*stage));
            div()
                .id(id)
                .debug_selector(move || id.to_string())
                .cursor_pointer()
                .pb(px(4.0))
                .border_b(px(2.0))
                .text_size(px(12.0))
                .font_weight(if current {
                    gpui::FontWeight::EXTRA_BOLD
                } else {
                    gpui::FontWeight::NORMAL
                })
                .text_color(if current {
                    color::foreground(cx)
                } else {
                    color::muted(cx)
                })
                .border_color(if current {
                    color::accent(cx)
                } else {
                    gpui::transparent_black().into()
                })
                .on_click(move |_event, window, cx| on_click(window, cx))
                .child(step.clone())
                .into_any_element()
        }))
        .into_any_element()
}

/// A footer button beside Cancel in its outline style, for Back.
fn secondary_button(id: &'static str, text: String, on_click: OnClick, cx: &App) -> AnyElement {
    div()
        .debug_selector(move || id.to_string())
        .id(id)
        .cursor_pointer()
        .py(px(8.0))
        .px(px(16.0))
        .border_1()
        .border_color(color::border(cx))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_color(color::foreground(cx))
        .on_click(move |_event, window, cx| on_click(window, cx))
        .child(SharedString::from(text))
        .into_any_element()
}

/// A text field of the form that a click focuses.
fn text(
    id: &'static str,
    field_label: AnyElement,
    field: &TextFieldView,
    to: LedgerField,
    actions: &Clicks,
    cx: &App,
) -> AnyElement {
    text_field(
        id,
        field_label,
        &field.text,
        &field.placeholder,
        field.focused,
        actions.send(CreateLedgerEvent::Focus(to)),
        cx,
    )
}

/// What a select's clicks send: `toggle` for a click on the field, `pick` built from the value of
/// the option clicked.
struct SelectEvents<T> {
    toggle: CreateLedgerEvent,
    pick: fn(T) -> CreateLedgerEvent,
}

/// The select `field` draws, its list open only while it has focus.
fn select<T: Copy + PartialEq + 'static>(
    id: &'static str,
    field: &SelectView<T>,
    (list, scroll): (SelectList, &mut usize),
    SelectEvents { toggle, pick }: SelectEvents<T>,
    actions: &Clicks,
    cx: &App,
) -> AnyElement {
    let list = if field.focused {
        list
    } else {
        SelectList::default()
    };
    let options = labels(field);
    let values: Vec<T> = field.options.iter().map(|choice| choice.value).collect();
    select_field::render(
        SelectFieldProps {
            id: id.into(),
            label: field.label.clone().into(),
            options: &options,
            state: &drawn(field, list, scroll),
            focused: field.focused,
            accent: false,
            read_only: None,
            on_field_click: actions.send(toggle),
            on_option_click: actions.option(move |index| {
                values
                    .get(index)
                    .map(|value| vec![pick(*value)])
                    .unwrap_or_default()
            }),
        },
        cx,
    )
}

fn base_unit_body(
    view: &CreateLedgerViewModel,
    scroll: &mut usize,
    actions: &Clicks,
    cx: &App,
) -> Vec<AnyElement> {
    let kind = select::<UnitKind>(
        "create-ledger-type",
        &view.kind,
        (view.list, scroll),
        SelectEvents {
            toggle: CreateLedgerEvent::ToggleKind,
            pick: CreateLedgerEvent::Kind,
        },
        actions,
        cx,
    );
    let file = text_field(
        "create-ledger-file",
        label(view.file.label.clone()),
        &view.file.text,
        "",
        false,
        Rc::new(|_window: &mut Window, _cx: &mut App| {}),
        cx,
    );
    vec![
        file,
        div().child(view.help.clone()).into_any_element(),
        two_up([
            text(
                "create-ledger-code",
                label(view.code.label.clone()),
                &view.code,
                LedgerField::Code,
                actions,
                cx,
            ),
            kind,
        ]),
        div()
            .child(two_up([
                text(
                    "create-ledger-name",
                    label(view.name.label.clone()),
                    &view.name,
                    LedgerField::Name,
                    actions,
                    cx,
                ),
                text(
                    "create-ledger-precision",
                    label(view.precision.label.clone()),
                    &view.precision,
                    LedgerField::Precision,
                    actions,
                    cx,
                ),
            ]))
            .child(help(view.precision_help.clone(), cx))
            .into_any_element(),
    ]
}

fn ledger_body(
    view: &CreateLedgerViewModel,
    scroll: &mut usize,
    actions: &Clicks,
    cx: &App,
) -> Vec<AnyElement> {
    let name = div()
        .child(text(
            "create-ledger-ledger-name",
            label(view.ledger_name.label.clone()),
            &view.ledger_name,
            LedgerField::LedgerName,
            actions,
            cx,
        ))
        .child(help(view.ledger_name_help.clone(), cx))
        .when_some(view.ledger_name_error.clone(), |this, error| {
            this.child(
                div()
                    .debug_selector(|| "create-ledger-ledger-name-error".to_string())
                    .mt(px(4.0))
                    .text_size(px(12.0))
                    .text_color(color::negative_text(cx))
                    .child(error),
            )
        })
        .into_any_element();
    let owner = div()
        .child(text(
            "create-ledger-owner",
            optional_label(view.owner.label.clone(), cx),
            &view.owner,
            LedgerField::Owner,
            actions,
            cx,
        ))
        .child(help(view.owner_help.clone(), cx))
        .into_any_element();
    let financial_year = div()
        .child(select::<FinancialYearStart>(
            "create-ledger-financial-year",
            &view.financial_year,
            (view.list, scroll),
            SelectEvents {
                toggle: CreateLedgerEvent::ToggleMonth,
                pick: CreateLedgerEvent::Month,
            },
            actions,
            cx,
        ))
        .child(help(view.financial_year_help.clone(), cx))
        .into_any_element();
    vec![
        div().child(view.ledger_help.clone()).into_any_element(),
        name,
        owner,
        financial_year,
    ]
}
