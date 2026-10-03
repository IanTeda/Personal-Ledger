//! Renders the **New budget** dialog (`docs/ux/desktop/14-budgets-v2/README.md`'s 11c) and its edit
//! mode on the shared `crate::dialog` chrome at the handoff's 640px: Name and Unit, the four
//! method cards, the on-budget Account chips and Start from. Only Category limits is built, so
//! the other three cards are drawn dimmed and take no click (#400). Edit mode locks the Unit and
//! leaves Start from out. `Shell` owns the live form (`budget_form::BudgetForm`).

use std::rc::Rc;

use gpui::{AnyElement, App, SharedString, Window, div, prelude::*, px};
use lib_locale::format::upper;

use crate::{
    budget_form::{BudgetField, BudgetForm, BudgetOptions, START_CHOICES, StartChoice},
    budgets::BudgetError,
    dialog,
    theme::color,
    view::accounts::{
        add_dialog::{label, text_field, two_up},
        select_field::{self, SelectFieldProps},
    },
};

use super::limit_dialog::{error_line, error_text, segmented};

/// The dialog's width: the handoff's 11c.
pub const WIDTH: gpui::Pixels = px(640.0);

pub type OnFieldClick = Rc<dyn Fn(BudgetField, &mut Window, &mut App)>;
/// Called with the clicked Account's id.
pub type OnAccountClick = Rc<dyn Fn(u32, &mut Window, &mut App)>;
pub type OnStartClick = Rc<dyn Fn(StartChoice, &mut Window, &mut App)>;

pub struct BudgetDialogHandlers {
    pub on_field_click: OnFieldClick,
    pub on_unit_option_click: select_field::OnOptionClick,
    pub on_account_click: OnAccountClick,
    pub on_start_click: OnStartClick,
    pub on_cancel: dialog::OnClick,
    pub on_confirm: dialog::OnClick,
}

pub struct BudgetDialogProps<'a> {
    pub form: &'a BudgetForm,
    pub options: &'a BudgetOptions,
    /// The Budget being edited, or the one "Copy categories from" names.
    pub budget_name: Option<String>,
    pub handlers: BudgetDialogHandlers,
}

/// A method card: its title, tag and description, and whether it can be chosen.
struct MethodCard {
    title: String,
    tag: String,
    detail: String,
    enabled: bool,
}

fn method_cards() -> [MethodCard; 4] {
    [
        MethodCard {
            title: crate::msg::desktop_budgets_new_method_envelope(),
            tag: crate::msg::desktop_budgets_new_method_envelope_tag(),
            detail: crate::msg::desktop_budgets_new_method_envelope_detail(),
            enabled: false,
        },
        MethodCard {
            title: crate::msg::desktop_budgets_new_method_limits(),
            tag: crate::msg::desktop_budgets_new_method_limits_tag(),
            detail: crate::msg::desktop_budgets_new_method_limits_detail(),
            enabled: true,
        },
        MethodCard {
            title: crate::msg::desktop_budgets_new_method_split(),
            tag: crate::msg::desktop_budgets_new_method_split_tag(),
            detail: crate::msg::desktop_budgets_new_method_split_detail(),
            enabled: false,
        },
        MethodCard {
            title: crate::msg::desktop_budgets_new_method_project(),
            tag: crate::msg::desktop_budgets_new_method_project_tag(),
            detail: crate::msg::desktop_budgets_new_method_project_detail(),
            enabled: false,
        },
    ]
}

/// The chosen card takes the 2px ink border and the chrome fill; the rest are dimmed and say
/// they come later.
fn method_card(card: MethodCard, cx: &App) -> AnyElement {
    div()
        .flex_1()
        .min_w(px(0.0))
        .flex()
        .flex_col()
        .gap(px(4.0))
        .p(px(12.0))
        .border(px(2.0))
        .border_color(if card.enabled {
            color::foreground(cx)
        } else {
            color::border(cx)
        })
        .when(card.enabled, |this| this.bg(color::chrome(cx)))
        .when(!card.enabled, |this| this.opacity(0.55))
        .child(
            div()
                .flex()
                .items_baseline()
                .justify_between()
                .gap(px(8.0))
                .child(
                    div()
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .text_size(px(13.0))
                        .child(card.title),
                )
                .child(
                    div()
                        .flex_none()
                        .text_size(px(10.0))
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .text_color(color::muted(cx))
                        .child(upper(&card.tag)),
                ),
        )
        .child(
            div()
                .text_size(px(11.5))
                .text_color(color::muted(cx))
                .child(card.detail),
        )
        .when(!card.enabled, |this| {
            this.child(
                div()
                    .text_size(px(10.0))
                    .font_weight(gpui::FontWeight::EXTRA_BOLD)
                    .text_color(color::muted(cx))
                    .child(upper(&crate::msg::desktop_budgets_new_method_later())),
            )
        })
        .into_any_element()
}

fn method_grid(cx: &App) -> AnyElement {
    let [envelope, limits, split, project] = method_cards();
    let row = |cards: [MethodCard; 2]| {
        div()
            .flex()
            .gap(px(10.0))
            .children(cards.into_iter().map(|card| method_card(card, cx)))
    };
    div()
        .child(label(crate::msg::desktop_budgets_new_field_method()))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(10.0))
                .child(row([envelope, limits]))
                .child(row([split, project])),
        )
        .into_any_element()
}

fn help(text: String, cx: &App) -> AnyElement {
    div()
        .mt(px(6.0))
        .text_size(px(11.5))
        .text_color(color::muted(cx))
        .child(text)
        .into_any_element()
}

/// The Account chips: a selected one is solid ink with a `✓`; the keyboard cursor takes the
/// accent border while the chips are focused.
fn account_chips(
    form: &BudgetForm,
    options: &BudgetOptions,
    on_click: &OnAccountClick,
    cx: &App,
) -> AnyElement {
    let focused = form.focused == BudgetField::Accounts;
    let chips = options
        .accounts
        .iter()
        .enumerate()
        .map(|(index, (id, name))| {
            let on_click = on_click.clone();
            let id = *id;
            let selected = form.account_ids.contains(&id);
            let cursor = focused && index == form.account_cursor;
            div()
                .id(SharedString::from(format!("budgets-new-account-{id}")))
                .cursor_pointer()
                .py(px(5.0))
                .px(px(10.0))
                .border(px(if cursor { 2.0 } else { 1.0 }))
                .border_color(if cursor {
                    color::accent(cx)
                } else if selected {
                    color::foreground(cx)
                } else {
                    color::border(cx)
                })
                .text_size(px(12.5))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .when(selected, |this| {
                    this.bg(color::foreground(cx))
                        .text_color(color::background(cx))
                })
                .on_click(move |_event, window, cx| on_click(id, window, cx))
                .child(if selected {
                    format!("\u{2713} {name}")
                } else {
                    name.clone()
                })
        });
    div()
        .child(label(crate::msg::desktop_budgets_new_field_accounts()))
        .child(div().flex().flex_wrap().gap(px(6.0)).children(chips))
        .child(help(
            if options.accounts.is_empty() {
                crate::msg::desktop_budgets_new_accounts_none()
            } else {
                crate::msg::desktop_budgets_new_accounts_help()
            },
            cx,
        ))
        .into_any_element()
}

fn save_error(error: &BudgetError) -> String {
    match error {
        BudgetError::NameTaken => crate::msg::desktop_budgets_new_error_name(),
        other => error_text(other),
    }
}

pub fn render(props: BudgetDialogProps<'_>, cx: &App) -> AnyElement {
    let BudgetDialogProps {
        form,
        options,
        budget_name,
        handlers,
    } = props;
    let BudgetDialogHandlers {
        on_field_click,
        on_unit_option_click,
        on_account_click,
        on_start_click,
        on_cancel,
        on_confirm,
    } = handlers;
    let field_click = |field: BudgetField| -> dialog::OnClick {
        let on_click = on_field_click.clone();
        Rc::new(move |window: &mut Window, cx: &mut App| on_click(field, window, cx))
    };
    let edit = form.is_edit();

    let mut fields = vec![
        two_up([
            div()
                .flex_1()
                .min_w(px(0.0))
                .child(text_field(
                    "budgets-new-name",
                    label(crate::msg::desktop_budgets_new_field_name()),
                    &form.name,
                    &crate::msg::desktop_budgets_new_name_placeholder(),
                    form.focused == BudgetField::Name,
                    field_click(BudgetField::Name),
                    cx,
                ))
                .into_any_element(),
            div()
                .flex_1()
                .min_w(px(0.0))
                .child(select_field::render(
                    SelectFieldProps {
                        id: "budgets-new-unit",
                        label: lib_locale::msg::column_unit().into(),
                        options: &options.units,
                        state: &form.unit,
                        focused: form.focused == BudgetField::Unit,
                        accent: false,
                        // Locked after creation, like a Bill Plan's.
                        read_only: edit
                            .then(|| form.unit_code().unwrap_or_default().to_string().into()),
                        on_field_click: field_click(BudgetField::Unit),
                        on_option_click: on_unit_option_click,
                    },
                    cx,
                ))
                .into_any_element(),
        ]),
        method_grid(cx),
        account_chips(form, options, &on_account_click, cx),
    ];
    if !edit {
        let cells = START_CHOICES
            .into_iter()
            .filter(|choice| *choice != StartChoice::Copy || form.can_copy())
            .map(|choice| {
                let on_click = on_start_click.clone();
                let click: dialog::OnClick =
                    Rc::new(move |window: &mut Window, cx: &mut App| on_click(choice, window, cx));
                let text = match choice {
                    StartChoice::Empty => crate::msg::desktop_budgets_new_start_empty(),
                    StartChoice::Copy => crate::msg::desktop_budgets_new_start_copy(
                        budget_name.as_deref().unwrap_or_default(),
                    ),
                    StartChoice::LastThreeMonths => crate::msg::desktop_budgets_new_start_average(),
                };
                (text, choice == form.start_from, click)
            })
            .collect();
        fields.push(
            div()
                .child(segmented(
                    "budgets-new-start",
                    crate::msg::desktop_budgets_new_field_start(),
                    form.focused == BudgetField::StartFrom,
                    cells,
                    cx,
                ))
                .child(help(crate::msg::desktop_budgets_new_start_help(), cx))
                .into_any_element(),
        );
    }
    if let Some(error) = form.error.as_ref() {
        fields.push(error_line(save_error(error), cx));
    }

    let (title, submit) = if edit {
        (
            crate::msg::desktop_budgets_new_title_edit(budget_name.as_deref().unwrap_or_default()),
            crate::msg::desktop_budgets_new_submit_edit(),
        )
    } else {
        (
            crate::msg::desktop_budgets_new_title(),
            crate::msg::desktop_budgets_new_submit(),
        )
    };
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(title, false, cx))
        .child(dialog::body(fields))
        .child(dialog::action_row(
            [
                dialog::cancel_button("budgets-new-cancel", on_cancel, cx).into_any_element(),
                dialog::confirm_button(
                    "budgets-new-confirm",
                    submit,
                    form.is_valid(),
                    false,
                    on_confirm,
                    cx,
                )
                .into_any_element(),
            ],
            cx,
        ));
    dialog::overlay(WIDTH, false, card, cx)
}
