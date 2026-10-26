//! The Add category and Edit category Dialogs (`docs/ux/desktop-mockups/18-categories/README.md`'s
//! 5b and 5c) on the shared `crate::dialog` frame: Name, Type (segmented, dimmed while locked),
//! Parent category (the shared `select_field` dropdown), Monthly budget and the notice.
//!
//! The select takes the arrows, `Enter` and `Space`, and swallows other typing rather than letting
//! it reach the Shell. On Type the arrows switch it, and typing is swallowed.

use gpui::{AnyElement, App, SharedString, div, prelude::*, px};
use lib_categories::presenter::{
    CategoryField, CategoryFormEvent, CategoryFormPresenter, CategoryFormViewModel,
};
use lib_core::CategoryTypes;
use lib_viper::view::SelectView;

use super::{CategoriesDialog, Clicks, View, send};
use crate::form::viper::{choose_option, chosen_label, enter, labels, select_key, synced, typed};
use crate::{
    dialog,
    form::{
        dialog_fields::{label, suffixed_label, text_field},
        select::SelectState,
        select_field::{self, SelectFieldProps},
    },
    shell::view::dialog_host::{DialogKey, DialogOutcome},
    theme::color,
};

/// The Dialogs' width: the handoff's 5b and 5c.
const WIDTH: gpui::Pixels = px(440.0);

fn handle(presenter: &mut CategoryFormPresenter, event: CategoryFormEvent) {
    send(presenter, event, CategoryFormPresenter::handle);
}

pub(super) fn key(
    presenter: &mut CategoryFormPresenter,
    select: &mut SelectState,
    key: DialogKey,
) -> DialogOutcome {
    let view = presenter.view_model();
    if matches!(key, DialogKey::Tab | DialogKey::BackTab) {
        // Leaving the select commits an open list's highlight, as a click on it would.
        if select.is_open() {
            let (_, value) = select_key(select, &view.parent, DialogKey::Enter);
            if let Some(id) = value {
                handle(presenter, CategoryFormEvent::Parent(id));
            }
        }
        let event = if key == DialogKey::Tab {
            CategoryFormEvent::NextField
        } else {
            CategoryFormEvent::PrevField
        };
        handle(presenter, event);
        return DialogOutcome::Handled;
    }
    match view.focused {
        CategoryField::Parent => {
            let (taken, value) = select_key(select, &view.parent, key);
            if let Some(id) = value {
                handle(presenter, CategoryFormEvent::Parent(id));
            }
            match key {
                DialogKey::Enter if !taken => enter(view.frame.can_confirm),
                // A select swallows typing rather than letting it fall through to the Shell.
                DialogKey::Char(_) | DialogKey::Backspace => DialogOutcome::Handled,
                _ if taken => DialogOutcome::Handled,
                _ => DialogOutcome::Ignored,
            }
        }
        CategoryField::Type => match key {
            DialogKey::Enter => enter(view.frame.can_confirm),
            DialogKey::Up | DialogKey::Down => {
                if let Some(other) = other_type(&view.category_type) {
                    handle(presenter, CategoryFormEvent::Type(other));
                }
                DialogOutcome::Handled
            }
            DialogKey::Char(_) | DialogKey::Backspace => DialogOutcome::Handled,
            _ => DialogOutcome::Ignored,
        },
        CategoryField::Name => text_key(
            presenter,
            &view.name.text,
            &view,
            key,
            CategoryFormEvent::Name,
        ),
        CategoryField::Budget => text_key(
            presenter,
            &view.budget.field.text,
            &view,
            key,
            CategoryFormEvent::Budget,
        ),
    }
}

/// The type the segmented control isn't on.
fn other_type(view: &SelectView<CategoryTypes>) -> Option<CategoryTypes> {
    view.options
        .iter()
        .map(|choice| choice.value.clone())
        .find(|value| Some(value) != view.chosen.as_ref())
}

/// A key on a text field: typing edits it, `Enter` confirms a valid form, the arrows do nothing.
fn text_key(
    presenter: &mut CategoryFormPresenter,
    text: &str,
    view: &CategoryFormViewModel,
    key: DialogKey,
    event: fn(String) -> CategoryFormEvent,
) -> DialogOutcome {
    match key {
        DialogKey::Enter => enter(view.frame.can_confirm),
        DialogKey::Up | DialogKey::Down => DialogOutcome::Handled,
        _ => match typed(text, key) {
            Some(text) => {
                handle(presenter, event(text));
                DialogOutcome::Handled
            }
            None => DialogOutcome::Ignored,
        },
    }
}

fn form(dialog: &mut CategoriesDialog) -> Option<(&mut CategoryFormPresenter, &mut SelectState)> {
    match &mut dialog.view {
        View::Form(presenter, select) => Some((presenter, select)),
        View::Delete(_) => None,
    }
}

/// A click on a text field focuses it.
fn click_field(dialog: &mut CategoriesDialog, field: CategoryField) {
    if let Some((presenter, select)) = form(dialog) {
        select.cancel();
        handle(presenter, CategoryFormEvent::Focus(field));
    }
}

/// A click on the select's closed field focuses it and toggles its list.
fn click_select(dialog: &mut CategoriesDialog) {
    if let Some((presenter, select)) = form(dialog) {
        handle(presenter, CategoryFormEvent::Focus(CategoryField::Parent));
        let view = presenter.view_model();
        *select = synced(select, &view.parent);
        if select.is_open() {
            select.cancel();
        } else {
            select.open(&labels(&view.parent));
        }
    }
}

fn click_option(dialog: &mut CategoriesDialog, index: usize) {
    if let Some((presenter, select)) = form(dialog)
        && let Some(id) = choose_option(select, &presenter.view_model().parent, index)
    {
        handle(presenter, CategoryFormEvent::Parent(id));
    }
}

fn click_type(dialog: &mut CategoriesDialog, category_type: CategoryTypes) {
    if let Some((presenter, select)) = form(dialog) {
        select.cancel();
        handle(presenter, CategoryFormEvent::Type(category_type));
    }
}

pub(super) fn render(
    view: &CategoryFormViewModel,
    select: &SelectState,
    actions: &Clicks,
    cx: &App,
) -> AnyElement {
    let id = if view.editing {
        "edit-category"
    } else {
        "add-category"
    };
    let element_id = |suffix: &str| SharedString::from(format!("{id}-{suffix}"));
    // `select_field` and `text_field` take a `'static` id, so these are spelt out.
    let (name_id, parent_id, budget_id) = if view.editing {
        (
            "edit-category-name",
            "edit-category-parent",
            "edit-category-budget",
        )
    } else {
        (
            "add-category-name",
            "add-category-parent",
            "add-category-budget",
        )
    };
    let options = labels(&view.parent);
    let on_field = |field: CategoryField| actions.click(move |dialog| click_field(dialog, field));
    let mut fields = vec![
        div()
            .child(text_field(
                name_id,
                label(view.name.label.clone()),
                &view.name.text,
                &view.name.placeholder,
                view.name.focused,
                on_field(CategoryField::Name),
                cx,
            ))
            .children(view.name_error.clone().map(|error| error_line(error, cx)))
            .into_any_element(),
        type_field(view, actions, cx),
        select_field::render(
            SelectFieldProps {
                id: parent_id,
                label: view.parent.label.clone().into(),
                options: &options,
                state: &synced(select, &view.parent),
                focused: view.parent.focused,
                accent: false,
                read_only: None,
                on_field_click: actions.click(click_select),
                on_option_click: actions.option(click_option),
            },
            cx,
        ),
        budget_field(view, budget_id, on_field(CategoryField::Budget), cx),
        notice(view.notice.clone(), cx),
    ];
    fields.extend(view.refusal.clone().map(|text| error_line(text, cx)));
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(view.frame.title.clone(), false, cx))
        .child(dialog::body(fields))
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

/// Expense and Income side by side, the chosen one filled; dimmed and not clickable while the type
/// follows the parent.
fn type_field(view: &CategoryFormViewModel, actions: &Clicks, cx: &App) -> AnyElement {
    let prefix = if view.editing { "edit-type" } else { "type" };
    let locked = view.type_locked;
    div()
        .child(label(view.category_type.label.clone()))
        .child(
            div()
                .flex()
                .gap(px(8.0))
                .children(view.category_type.options.iter().map(|choice| {
                    let chosen = view.category_type.chosen.as_ref() == Some(&choice.value);
                    let suffix = match choice.value {
                        CategoryTypes::Expense => "expense",
                        CategoryTypes::Income => "income",
                    };
                    let element = format!("{prefix}-{suffix}");
                    let value = choice.value.clone();
                    let on_click = actions.click(move |dialog| click_type(dialog, value.clone()));
                    div()
                        .debug_selector({
                            let element = element.clone();
                            move || element.clone()
                        })
                        .id(SharedString::from(element))
                        .cursor_pointer()
                        .px(px(12.0))
                        .py(px(6.0))
                        .border_1()
                        .border_color(if chosen {
                            color::selection_background(cx)
                        } else {
                            color::border(cx)
                        })
                        .bg(if chosen {
                            color::selection_background(cx)
                        } else {
                            color::background(cx)
                        })
                        .text_color(if chosen {
                            color::selection_text(cx)
                        } else {
                            color::foreground(cx)
                        })
                        .text_size(px(12.0))
                        .font_weight(gpui::FontWeight::BOLD)
                        .when(!locked, |this| {
                            this.on_click(move |_event, window, cx| on_click(window, cx))
                        })
                        .when(locked, |this| this.opacity(0.6))
                        .child(choice.label.clone())
                })),
        )
        .into_any_element()
}

/// The Monthly budget input, or while read-only the text the Presenter gives in its place.
fn budget_field(
    view: &CategoryFormViewModel,
    id: &'static str,
    on_click: dialog::OnClick,
    cx: &App,
) -> AnyElement {
    let field = &view.budget.field;
    let Some(read_only) = view.budget.read_only.clone() else {
        return text_field(
            id,
            suffixed_label(field.label.clone(), view.optional.clone(), cx),
            &field.text,
            &field.placeholder,
            field.focused,
            on_click,
            cx,
        );
    };
    div()
        .child(suffixed_label(
            field.label.clone(),
            view.optional.clone(),
            cx,
        ))
        .child(
            div()
                .w_full()
                .py(px(8.0))
                .px(px(10.0))
                .border_1()
                .border_color(color::border(cx))
                .bg(color::chrome(cx))
                .text_size(px(13.0))
                .text_color(color::muted(cx))
                .opacity(0.6)
                .child(read_only),
        )
        .into_any_element()
}

/// The dark-bordered note under the fields.
fn notice(text: String, cx: &App) -> AnyElement {
    div()
        .border_l(px(2.0))
        .border_color(color::foreground(cx))
        .bg(color::chrome(cx))
        .px(px(10.0))
        .py(px(10.0))
        .text_size(px(11.5))
        .text_color(color::muted(cx))
        .child(text)
        .into_any_element()
}

/// A problem with a field or a refused confirm, under the field.
pub(super) fn error_line(text: String, cx: &App) -> AnyElement {
    div()
        .mt(px(4.0))
        .text_size(px(11.5))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_color(color::accent_text(cx))
        .child(text)
        .into_any_element()
}

/// The form's fields, for a test: the parent chosen, when there is one, and on Edit the parents
/// offered.
pub(super) fn readback(view: &CategoryFormViewModel) -> Vec<(&'static str, String)> {
    let mut values = vec![
        ("field", format!("{:?}", view.focused)),
        ("name", view.name.text.clone()),
        (
            "type",
            view.category_type
                .chosen
                .as_ref()
                .map(|chosen| format!("{chosen:?}"))
                .unwrap_or_default(),
        ),
        ("budget", view.budget.field.text.clone()),
        (
            "error",
            view.name_error
                .clone()
                .or_else(|| view.refusal.clone())
                .unwrap_or_default(),
        ),
    ];
    if view.parent.chosen.flatten().is_some() {
        values.push(("parent", chosen_label(&view.parent).unwrap_or_default()));
    }
    if view.editing {
        let parents: Vec<&str> = view
            .parent
            .options
            .iter()
            .filter(|choice| choice.value.is_some())
            .map(|choice| choice.label.as_str())
            .collect();
        values.push(("parents", parents.join(",")));
    }
    values
}
