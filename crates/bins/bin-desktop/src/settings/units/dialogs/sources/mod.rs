//! The price source Dialogs: Add and Edit price source for a Custom source (16z·custom), Edit price
//! source for a built-in (16z·edit) and Delete price source (16z·del). [`PriceSourceDialog`] is the
//! value the Dialog host holds while one is open (`OpenDialog::PriceSources`). It draws itself, its
//! clicks reaching the form as a [`Change`] through the host's edit callback, and reads itself back
//! for the UI tests. Manual entry has no Dialog: it is neither edited nor deleted.

pub mod built_in;
pub mod custom;
pub mod delete;

use std::rc::Rc;

use gpui::{AnyElement, App, Window, div, prelude::*, px};
use lib_units::{PriceSourceId, UnitRow, Updates};

use crate::{
    chrome::dialog_host::{
        Dialog, DialogActions, DialogKey, DialogOutcome, DialogReadback, DialogScope,
    },
    dialog,
    form::field::TextField,
    settings::units::source_form::{
        AuthChoice, BuiltInSourceForm, CustomSourceForm, DeleteSourceForm, SourceField,
    },
    theme::color,
};

/// The open price source Dialog. Edit and Delete carry the source's id, so the confirm writes the
/// right row even if the table re-sorts under the open Dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PriceSourceDialog {
    Add(CustomSourceForm),
    EditCustom(PriceSourceId, CustomSourceForm),
    EditBuiltIn(PriceSourceId, BuiltInSourceForm),
    Delete(PriceSourceId, DeleteSourceForm),
}

/// What a click inside a price source Dialog changes on its form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    Focus(SourceField),
    Updates(Updates),
    Auth(AuthChoice),
}

pub type OnChange = Rc<dyn Fn(Change, &mut Window, &mut App)>;

/// The handlers a price source Dialog binds: changes to the open form, cancel and confirm.
#[derive(Clone)]
pub struct SourceDialogHandlers {
    pub on_change: OnChange,
    pub on_cancel: dialog::OnClick,
    pub on_confirm: dialog::OnClick,
}

impl SourceDialogHandlers {
    fn new(actions: &DialogActions) -> Self {
        let on_change: OnChange = {
            let actions = actions.clone();
            Rc::new(move |change, _window, cx| {
                actions.edit(cx, |dialog: &mut PriceSourceDialog, _| {
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

    /// A click handler that applies `change` to the open Dialog.
    fn click(&self, change: Change) -> dialog::OnClick {
        let on_change = self.on_change.clone();
        Rc::new(move |window: &mut Window, cx: &mut App| on_change(change, window, cx))
    }
}

impl PriceSourceDialog {
    /// Applies a click's change; Delete has nothing to click into.
    pub fn apply_change(&mut self, change: Change) {
        match self {
            Self::Add(form) | Self::EditCustom(_, form) => match change {
                Change::Focus(field) if form.shows(field) => form.focused = field,
                Change::Focus(_) => {}
                Change::Updates(updates) => form.updates = updates,
                Change::Auth(auth) => form.set_auth(auth),
            },
            Self::EditBuiltIn(_, form) => match change {
                Change::Focus(field @ (SourceField::Name | SourceField::Updates)) => {
                    form.focused = field;
                }
                Change::Updates(updates) => form.updates = updates,
                Change::Focus(_) | Change::Auth(_) => {}
            },
            Self::Delete(..) => {}
        }
    }
}

/// The Units on source `id`, in table order: the ones it values.
fn units_on(units: &[UnitRow], id: PriceSourceId) -> Vec<&UnitRow> {
    units
        .iter()
        .filter(|unit| unit.link.source_id == id)
        .collect()
}

/// The Units' codes as one list, for a Message's `$codes`.
fn codes(units: &[&UnitRow]) -> String {
    units
        .iter()
        .map(|unit| unit.code.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

impl Dialog for PriceSourceDialog {
    fn handle_own_key(&mut self, key: DialogKey) -> Option<DialogOutcome> {
        match self {
            Self::Add(form) | Self::EditCustom(_, form) => form.handle_own_key(key),
            Self::EditBuiltIn(_, form) => form.handle_own_key(key),
            Self::Delete(..) => None,
        }
    }

    fn focused_text(&mut self) -> Option<&mut TextField> {
        match self {
            Self::Add(form) | Self::EditCustom(_, form) => form.focused_mut(),
            Self::EditBuiltIn(_, form) => form.focused_mut(),
            Self::Delete(_, form) => Some(&mut form.confirm_input),
        }
    }

    fn cycle_field(&mut self) {
        match self {
            Self::Add(form) | Self::EditCustom(_, form) => form.cycle_focus(false),
            Self::EditBuiltIn(_, form) => form.cycle_focus(false),
            Self::Delete(..) => {}
        }
    }

    fn is_valid(&self) -> bool {
        match self {
            Self::Add(form) | Self::EditCustom(_, form) => form.is_valid(),
            Self::EditBuiltIn(_, form) => form.is_valid(),
            Self::Delete(_, form) => form.is_valid(),
        }
    }

    fn render(&self, scope: &DialogScope, actions: &DialogActions, cx: &App) -> Option<AnyElement> {
        let handlers = SourceDialogHandlers::new(actions);
        let units = scope.ledger.read(cx).ledger().units();
        Some(match self {
            Self::Add(form) => custom::render(form, false, handlers, cx),
            Self::EditCustom(_, form) => custom::render(form, true, handlers, cx),
            Self::EditBuiltIn(id, form) => match lib_units::built_in(*id) {
                Some(source) => built_in::render(source, form, &units_on(units, *id), handlers, cx),
                None => div().into_any_element(),
            },
            Self::Delete(id, form) => delete::render(form, &units_on(units, *id), handlers, cx),
        })
    }

    /// Each form's fields by name; the API key reads back masked, as it is drawn.
    fn readback(&self, _scope: &DialogScope, _cx: &App) -> Option<DialogReadback> {
        let (kind, values) = match self {
            Self::Add(form) | Self::EditCustom(_, form) => (
                if matches!(self, Self::Add(_)) {
                    "AddPriceSource"
                } else {
                    "EditPriceSource"
                },
                vec![
                    ("name", form.name.text().to_string()),
                    ("updates", format!("{:?}", form.updates)),
                    ("address", form.address.text().to_string()),
                    ("request", form.request.text().to_string()),
                    ("price_path", form.price_path.text().to_string()),
                    ("auth", format!("{:?}", form.auth)),
                    ("key_param", form.key_param.text().to_string()),
                    ("api_key", custom::masked(form.api_key.text())),
                    ("field", format!("{:?}", form.focused)),
                ],
            ),
            Self::EditBuiltIn(_, form) => (
                "EditBuiltInPriceSource",
                vec![
                    ("name", form.name.text().to_string()),
                    ("updates", format!("{:?}", form.updates)),
                    ("field", format!("{:?}", form.focused)),
                ],
            ),
            Self::Delete(_, form) => (
                "DeletePriceSource",
                vec![("confirm", form.confirm_input.text().to_string())],
            ),
        };
        Some(DialogReadback { kind, values })
    }
}

/// The Updates option's Message.
pub(crate) fn updates_label(updates: Updates) -> String {
    match updates {
        Updates::Every15Minutes => {
            crate::msg::desktop_settings_price_source_updates_every_15_minutes()
        }
        Updates::Hourly => crate::msg::desktop_settings_price_source_updates_hourly(),
        Updates::Daily => crate::msg::desktop_settings_price_source_updates_daily(),
        Updates::ManualOnly => crate::msg::desktop_settings_price_source_updates_manual_only(),
    }
}

/// A field's 11.5px muted help line under it.
fn help(text: String, cx: &App) -> impl IntoElement {
    div()
        .mt(px(4.0))
        .text_size(px(11.5))
        .text_color(color::muted(cx))
        .child(text)
}

/// A test button that has nothing behind it yet: drawn disabled beside its note, since fetching a
/// price is still undecided.
fn test_not_built(id: &'static str, label: String, cx: &App) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(px(10.0))
        .child(
            div()
                .debug_selector(move || id.to_string())
                .py(px(7.0))
                .px(px(12.0))
                .border_1()
                .border_color(color::border(cx))
                .opacity(0.45)
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(12.0))
                .child(label),
        )
        .child(
            div()
                .text_size(px(11.5))
                .text_color(color::faint_text(cx))
                .child(crate::msg::desktop_settings_price_source_test_not_built()),
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chrome::dialog_host::{OpenDialog, handle_key, type_text};

    fn form(dialog: &OpenDialog) -> &CustomSourceForm {
        match dialog {
            OpenDialog::PriceSources(source) => match &**source {
                PriceSourceDialog::Add(form) => form,
                other => panic!("expected Add price source, got {other:?}"),
            },
            other => panic!("expected a price source Dialog, got {other:?}"),
        }
    }

    #[test]
    fn typing_fills_the_custom_form_and_enter_confirms_once_it_is_valid() {
        let mut dialog = OpenDialog::PriceSources(Box::new(PriceSourceDialog::Add(
            CustomSourceForm::default(),
        )));
        type_text(&mut dialog, "Broker");
        handle_key(&mut dialog, DialogKey::Tab);
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Char('x')),
            DialogOutcome::Handled,
            "Updates swallows typing"
        );
        handle_key(&mut dialog, DialogKey::Tab);
        type_text(&mut dialog, "https://broker.example");
        handle_key(&mut dialog, DialogKey::Tab);
        type_text(&mut dialog, "/{symbol}");
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Handled
        );
        handle_key(&mut dialog, DialogKey::Tab);
        type_text(&mut dialog, "/price");
        assert_eq!(form(&dialog).name.text(), "Broker");
        assert_eq!(form(&dialog).request.text(), "/{symbol}");
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Confirm
        );
    }

    #[test]
    fn a_click_cannot_focus_a_hidden_key_field_or_change_a_built_ins_auth() {
        let mut dialog = PriceSourceDialog::Add(CustomSourceForm::default());
        dialog.apply_change(Change::Focus(SourceField::ApiKey));
        dialog.apply_change(Change::Updates(Updates::Hourly));
        let PriceSourceDialog::Add(form) = &dialog else {
            unreachable!()
        };
        assert_eq!(form.focused, SourceField::Name);
        assert_eq!(form.updates, Updates::Hourly);

        let mut built_in = PriceSourceDialog::EditBuiltIn(
            lib_units::COINGECKO,
            BuiltInSourceForm::new("CoinGecko", Updates::Every15Minutes),
        );
        built_in.apply_change(Change::Focus(SourceField::Address));
        built_in.apply_change(Change::Auth(AuthChoice::ApiKey));
        let PriceSourceDialog::EditBuiltIn(_, form) = &built_in else {
            unreachable!()
        };
        assert_eq!(form.focused, SourceField::Name);
    }
}
