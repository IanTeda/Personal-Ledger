//! The Units page's Dialogs: Add, Edit and Delete unit. [`UnitsDialog`] is the value the Dialog
//! host holds while one is open (`OpenDialog::Units`). It draws itself (`Dialog::render`), every
//! click inside it reaching the form as a [`Change`] through the host's one edit callback, and
//! reads itself back for the UI tests.

pub mod add;
pub mod delete;
pub mod edit;
pub mod sources;

use std::rc::Rc;

use gpui::{AnyElement, App, Window, div, prelude::*, px};
use lib_units::{UnitId, UnitRow};

use self::sources::test_not_built;
use super::form::{DeleteUnitForm, UnitField, UnitForm};
use crate::{
    chrome::dialog_host::{
        Dialog, DialogActions, DialogKey, DialogOutcome, DialogReadback, DialogScope,
    },
    dialog,
    form::{
        dialog_fields::{label, text_field, two_up},
        field::TextField,
        select_field::{self, SelectFieldProps},
    },
    theme::color,
};

/// The open Units Dialog. Edit and Delete carry the row's [`UnitId`], so the confirm knows which
/// row to overwrite or remove even if the table shifts under the open Dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnitsDialog {
    Add(UnitForm),
    Edit(UnitId, UnitForm),
    Delete(UnitId, DeleteUnitForm),
}

/// Applies a [`Change`] to the open Units Dialog ([`UnitsDialog::apply_change`]) through the
/// host's edit callback, then repaints.
pub type OnChange = Rc<dyn Fn(Change, &mut Window, &mut App)>;

/// What a click inside a Units Dialog changes on its form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    /// A click on a field: focuses a text field, or focuses a select and toggles its list.
    Field(UnitField),
    /// A click on row `.1` of the open list behind select `.0`.
    Option(UnitField, usize),
}

/// The handlers a Units Dialog binds: changes to the open form, cancel and confirm.
#[derive(Clone)]
pub struct UnitsDialogHandlers {
    pub on_change: OnChange,
    pub on_cancel: dialog::OnClick,
    pub on_confirm: dialog::OnClick,
}

impl UnitsDialogHandlers {
    /// A change edits the open Units Dialog through `actions`.
    fn new(actions: &DialogActions) -> Self {
        let on_change: OnChange = {
            let actions = actions.clone();
            Rc::new(move |change, _window, cx| {
                actions.edit(cx, |dialog: &mut UnitsDialog, _| {
                    dialog.apply_change(change);
                    None
                });
            })
        };
        Self {
            on_change,
            on_cancel: actions.cancel.clone(),
            on_confirm: actions.confirm.clone(),
        }
    }
}

impl UnitsDialog {
    /// Draws the open Dialog. Delete reads its row from `units`; a row that has gone draws
    /// nothing rather than panicking mid-render.
    fn draw(&self, units: &[UnitRow], handlers: UnitsDialogHandlers, cx: &App) -> AnyElement {
        match self {
            Self::Add(form) => add::render(form, handlers, cx),
            Self::Edit(_, form) => edit::render(form, handlers, cx),
            Self::Delete(id, form) => match units.iter().find(|row| row.id == *id) {
                Some(row) => delete::render(row, form, handlers, cx),
                None => div().into_any_element(),
            },
        }
    }

    /// Applies a click's change to the Add or Edit form; Delete has no field to click into.
    pub fn apply_change(&mut self, change: Change) {
        if let Self::Add(form) | Self::Edit(_, form) = self {
            match change {
                Change::Field(field) if field.is_select() => form.click_select(field),
                Change::Field(field) => form.focus(field),
                Change::Option(field, index) => form.choose_option(field, index),
            }
        }
    }
}

impl Dialog for UnitsDialog {
    fn handle_own_key(&mut self, key: DialogKey) -> Option<DialogOutcome> {
        match self {
            Self::Add(form) | Self::Edit(_, form) => form.handle_own_key(key),
            Self::Delete(..) => None,
        }
    }

    fn focused_text(&mut self) -> Option<&mut TextField> {
        match self {
            Self::Add(form) | Self::Edit(_, form) => form.focused_text(),
            Self::Delete(_, form) => Some(&mut form.confirm_input),
        }
    }

    fn cycle_field(&mut self) {
        if let Self::Add(form) | Self::Edit(_, form) = self {
            form.cycle_focus(false);
        }
    }

    fn close_open_select(&mut self) -> bool {
        match self {
            Self::Add(form) | Self::Edit(_, form) => form.close_open_select(),
            Self::Delete(..) => false,
        }
    }

    fn is_valid(&self) -> bool {
        match self {
            Self::Add(form) | Self::Edit(_, form) => form.is_valid(),
            Self::Delete(_, form) => form.is_valid(),
        }
    }

    fn render(&self, scope: &DialogScope, actions: &DialogActions, cx: &App) -> Option<AnyElement> {
        Some(self.draw(
            scope.ledger.read(cx).ledger().units(),
            UnitsDialogHandlers::new(actions),
            cx,
        ))
    }

    /// Add and Edit read back their fields, the Source list on offer and whether a select is
    /// open; Delete its typed-back code.
    fn readback(&self, _scope: &DialogScope, _cx: &App) -> Option<DialogReadback> {
        let (kind, values) = match self {
            Self::Add(form) | Self::Edit(_, form) => {
                let kind = if matches!(self, Self::Add(_)) {
                    "AddUnit"
                } else {
                    "EditUnit"
                };
                (
                    kind,
                    vec![
                        ("code", form.code.text().to_string()),
                        ("name", form.name.text().to_string()),
                        ("kind", format!("{:?}", form.selected_kind())),
                        (
                            "source",
                            form.source.value().unwrap_or_default().to_string(),
                        ),
                        ("sources", form.options_for(UnitField::Source).join("|")),
                        ("symbol", form.symbol.text().to_string()),
                        ("quote", form.quote.value().unwrap_or_default().to_string()),
                        ("manual", form.is_manual().to_string()),
                        ("visible", form.visible_fields().join("|")),
                        ("field", format!("{:?}", form.focused_field)),
                        ("select_open", form.any_select_open().to_string()),
                    ],
                )
            }
            Self::Delete(_, form) => (
                "DeleteUnit",
                vec![("confirm", form.confirm_input.text().to_string())],
            ),
        };
        Some(DialogReadback { kind, values })
    }
}

/// The element ids one Units dialog's fields carry, so Add and Edit stay distinct.
#[derive(Clone, Copy)]
pub(super) struct FieldIds {
    pub code: &'static str,
    pub kind: &'static str,
    pub name: &'static str,
    pub source: &'static str,
    pub symbol: &'static str,
    pub quote: &'static str,
    pub test: &'static str,
}

/// The fields Add and Edit share: Code and Type, Name, then the PRICE SOURCE group. Manual entry
/// hides Symbol, Quoted in and Test price.
pub(super) fn form_fields(
    ids: FieldIds,
    form: &UnitForm,
    handlers: &UnitsDialogHandlers,
    cx: &App,
) -> Vec<AnyElement> {
    let focused = |field: UnitField| form.focused_field == field;
    let option_click = |field: UnitField| -> select_field::OnOptionClick {
        let on_change = handlers.on_change.clone();
        Rc::new(move |index: usize, window: &mut Window, cx: &mut App| {
            on_change(Change::Option(field, index), window, cx)
        })
    };
    let select = |id, text: String, field: UnitField, options: &[String], state| {
        select_field::render(
            SelectFieldProps {
                id,
                label: text.into(),
                options,
                state,
                focused: focused(field),
                accent: false,
                read_only: None,
                on_field_click: on_change(handlers, Change::Field(field)),
                on_option_click: option_click(field),
            },
            cx,
        )
    };
    let kinds = form.options_for(UnitField::Type);
    let sources = form.options_for(UnitField::Source);
    let quotes = form.options_for(UnitField::Quote);

    let mut fields = vec![
        two_up([
            text_field(
                ids.code,
                label(lib_locale::msg::column_code()),
                form.code.text(),
                &crate::msg::desktop_settings_units_code_placeholder(),
                focused(UnitField::Code),
                on_change(handlers, Change::Field(UnitField::Code)),
                cx,
            ),
            select(
                ids.kind,
                lib_locale::msg::column_type(),
                UnitField::Type,
                &kinds,
                &form.kind,
            ),
        ]),
        text_field(
            ids.name,
            label(lib_locale::msg::column_name()),
            form.name.text(),
            &crate::msg::desktop_settings_units_name_placeholder(),
            focused(UnitField::Name),
            on_change(handlers, Change::Field(UnitField::Name)),
            cx,
        ),
        group_label(crate::msg::desktop_settings_units_group_source(), cx),
        div()
            .child(select(
                ids.source,
                crate::msg::desktop_settings_units_field_source(),
                UnitField::Source,
                &sources,
                &form.source,
            ))
            .child(help(crate::msg::desktop_settings_units_source_help(), cx))
            .into_any_element(),
    ];
    if !form.is_manual() {
        fields.push(two_up([
            div()
                .flex_1()
                .min_w(px(0.0))
                .child(symbol_field(ids.symbol, form, handlers, cx))
                .child(help(
                    crate::msg::desktop_settings_units_symbol_help(form.source_name()),
                    cx,
                ))
                .into_any_element(),
            select(
                ids.quote,
                crate::msg::desktop_settings_units_field_quote(),
                UnitField::Quote,
                &quotes,
                &form.quote,
            ),
        ]));
        fields.push(
            test_not_built(
                ids.test,
                crate::msg::desktop_settings_units_test_price(),
                cx,
            )
            .into_any_element(),
        );
    }
    fields
}

/// The caps label heading the PRICE SOURCE group, over a 2px rule.
fn group_label(text: String, cx: &App) -> AnyElement {
    div()
        .pb(px(6.0))
        .border_b(px(2.0))
        .border_color(color::structural_rule(cx))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(10.0))
        .child(lib_locale::format::upper(&text))
        .into_any_element()
}

/// A field's 11.5px muted help line under it.
fn help(text: String, cx: &App) -> impl IntoElement {
    div()
        .mt(px(6.0))
        .text_size(px(11.5))
        .text_color(color::muted(cx))
        .child(text)
}

/// The Symbol text box, in monospace: it holds the source's own id for the Unit.
fn symbol_field(
    id: &'static str,
    form: &UnitForm,
    handlers: &UnitsDialogHandlers,
    cx: &App,
) -> AnyElement {
    let focused = form.focused_field == UnitField::Symbol;
    let value = form.symbol.text();
    let caret = if focused { "\u{2502}" } else { "" };
    let (text, text_color) = if value.is_empty() {
        (
            format!(
                "{}{caret}",
                crate::msg::desktop_settings_units_symbol_placeholder()
            ),
            color::faint_text(cx),
        )
    } else {
        (format!("{value}{caret}"), color::foreground(cx))
    };
    let on_click = on_change(handlers, Change::Field(UnitField::Symbol));
    div()
        .child(label(crate::msg::desktop_settings_units_field_symbol()))
        .child(
            div()
                .id(id)
                .debug_selector(move || id.to_string())
                .cursor_pointer()
                .w_full()
                .py(px(8.0))
                .px(px(10.0))
                .border_1()
                .border_color(if focused {
                    color::accent(cx)
                } else {
                    color::border(cx)
                })
                .font_family("monospace")
                .text_size(px(13.0))
                .text_color(text_color)
                .on_click(move |_event, window, cx| on_click(window, cx))
                .child(text),
        )
        .into_any_element()
}

/// A click handler that applies `change` to the open Dialog.
fn on_change(handlers: &UnitsDialogHandlers, change: Change) -> dialog::OnClick {
    let on_change = handlers.on_change.clone();
    Rc::new(move |window: &mut Window, cx: &mut App| on_change(change, window, cx))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chrome::dialog_host::{DialogKey, DialogOutcome, OpenDialog, handle_key, type_text};

    fn unit_form(dialog: &OpenDialog) -> &UnitForm {
        match dialog {
            OpenDialog::Units(UnitsDialog::Add(form)) => form,
            other => panic!("expected Add unit, got {other:?}"),
        }
    }

    #[test]
    fn typing_and_backspace_edit_the_focused_field_only() {
        let mut dialog = OpenDialog::Units(UnitsDialog::Add(UnitForm::default()));
        type_text(&mut dialog, "aud");
        handle_key(&mut dialog, DialogKey::Backspace);
        assert_eq!(unit_form(&dialog).code.text(), "au");

        handle_key(&mut dialog, DialogKey::Tab);
        handle_key(&mut dialog, DialogKey::Tab);
        assert_eq!(unit_form(&dialog).focused_field, UnitField::Name);
        type_text(&mut dialog, "x");
        assert_eq!(unit_form(&dialog).name.text(), "x");
        assert_eq!(unit_form(&dialog).code.text(), "au");
    }

    #[test]
    fn enter_confirms_only_once_the_form_is_valid() {
        let mut dialog = OpenDialog::Units(UnitsDialog::Add(UnitForm::default()));
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Handled
        );
        type_text(&mut dialog, "aud");
        handle_key(&mut dialog, DialogKey::Tab);
        handle_key(&mut dialog, DialogKey::Tab);
        type_text(&mut dialog, "Australian dollar");
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Confirm
        );
    }

    #[test]
    fn delete_unit_confirms_only_on_the_exact_case_sensitive_code() {
        let mut dialog = OpenDialog::Units(UnitsDialog::Delete(
            UnitId::SEED_BTC,
            DeleteUnitForm::new("btc"),
        ));
        type_text(&mut dialog, "BTC");
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Handled
        );
        for _ in 0..3 {
            handle_key(&mut dialog, DialogKey::Backspace);
        }
        type_text(&mut dialog, "btc");
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Confirm
        );
    }

    #[test]
    fn a_click_change_reaches_add_and_edit_but_not_delete() {
        let mut dialog = UnitsDialog::Edit(UnitId::SEED_AUD, UnitForm::default());
        dialog.apply_change(Change::Field(UnitField::Name));
        let UnitsDialog::Edit(_, form) = &dialog else {
            unreachable!()
        };
        assert_eq!(form.focused_field, UnitField::Name);

        let mut delete = UnitsDialog::Delete(UnitId::SEED_BTC, DeleteUnitForm::new("btc"));
        let before = delete.clone();
        delete.apply_change(Change::Field(UnitField::Name));
        assert_eq!(delete, before);
    }
}
