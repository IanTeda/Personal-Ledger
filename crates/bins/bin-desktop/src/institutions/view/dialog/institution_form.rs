//! The Add institution Dialog (issue #187's "2e") on the shared `crate::dialog` frame, the one
//! dialog in this map whose own raw markup sets `width:460px` instead of the shared
//! [`dialog::WIDTH`] (see that constant's own doc).
//!
//! Three field shapes, none reused verbatim from the Unit dialogs:
//! - **Institution name**: the dialog's only text field, so it holds the typing whenever it has
//!   focus; a click into it focuses it and closes the Usual unit list.
//! - **Products**: multi-select chips (`docs/ux/desktop-mockups/styles.css`'s `.chip` role), the
//!   first *multi*-select control in this crate; every other "pick one" control (segmented
//!   controls, dot radios) only ever holds one selection at a time.
//! - **Usual unit**: the shared dropdown (`crate::form::select_field`), optional, so its first
//!   option is None. It takes the arrows, `Enter` and `Space` while focused; `Tab` moves between it
//!   and the name.

use gpui::{AnyElement, App, SharedString, div, prelude::*, px};
use lib_institutions::presenter::{
    InstitutionField, InstitutionFormEvent, InstitutionFormPresenter, InstitutionFormViewModel,
};
use lib_shell::Hint;

use crate::form::viper::{chosen_label, drawn, labels};
use crate::shell::view::dialog_entity::{DialogView, ModuleDialogClicks, framed};
use crate::{
    dialog,
    form::colour_field::error_line,
    form::fields::{field_label, text_field},
    form::select_field::{self, SelectFieldProps},
    theme::color,
};

const WIDTH: gpui::Pixels = px(460.0);

/// The open form's clicks.
type Clicks = ModuleDialogClicks<InstitutionFormPresenter>;

/// The `lib-institutions` form Presenter's [`DialogView`], hosted in a `ModuleDialog` (ADR-0056).
/// Its widget state is the top row of the Usual unit list's window, which the View keeps between
/// draws.
impl DialogView for InstitutionFormPresenter {
    type Widgets = usize;

    fn kind(view: &InstitutionFormViewModel) -> &'static str {
        if view.editing {
            "EditInstitution"
        } else {
            "AddInstitution"
        }
    }

    fn render(
        view: &InstitutionFormViewModel,
        scroll: &mut usize,
        clicks: &Clicks,
        cx: &App,
    ) -> AnyElement {
        render(view, scroll, clicks, cx)
    }

    /// The Name draft, the checked products (comma-joined variant names) and the chosen Usual
    /// unit, between the title and whether it would confirm.
    fn readback(view: &InstitutionFormViewModel) -> Vec<(&'static str, String)> {
        framed(&view.frame, readback(view))
    }

    fn hints(view: &InstitutionFormViewModel) -> Vec<Hint> {
        view.hints.clone()
    }
}

fn render(
    view: &InstitutionFormViewModel,
    scroll: &mut usize,
    actions: &Clicks,
    cx: &App,
) -> AnyElement {
    let unit_options = labels(&view.usual_unit);
    let unit_state = drawn(&view.usual_unit, view.usual_unit_list, scroll);
    // A click on row `index` of the open list picks its Unit.
    let units: Vec<_> = view
        .usual_unit
        .options
        .iter()
        .map(|choice| choice.value)
        .collect();
    let on_unit = actions.option(move |index| {
        units
            .get(index)
            .map(|unit| InstitutionFormEvent::UsualUnit(*unit))
            .into_iter()
            .collect()
    });
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(view.frame.title.clone(), false, cx))
        .child(dialog::body([
            div()
                .child(
                    text_field(
                        "add-institution-name",
                        view.name.label.clone(),
                        &view.name.text,
                        &view.name.placeholder,
                        view.name.focused,
                        // A click on the name focuses it, closing the list.
                        actions.send(InstitutionFormEvent::Focus(InstitutionField::Name)),
                        cx,
                    )
                    .into_any_element(),
                )
                .children(view.name_error.clone().map(|error| error_line(error, cx)))
                .into_any_element(),
            products_field(view, actions, cx).into_any_element(),
            select_field::render(
                SelectFieldProps {
                    id: "institution-usual-unit".into(),
                    label: view.usual_unit.label.clone().into(),
                    options: &unit_options,
                    state: &unit_state,
                    focused: view.usual_unit.focused,
                    accent: false,
                    read_only: None,
                    // A click on the closed field focuses it and toggles its list.
                    on_field_click: actions.send(InstitutionFormEvent::ToggleUnits),
                    on_option_click: on_unit,
                },
                cx,
            ),
        ]))
        .child(dialog::action_row(
            [
                dialog::cancel_button("add-institution-cancel", actions.cancel.clone(), cx)
                    .into_any_element(),
                dialog::confirm_button(
                    "add-institution-confirm",
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

/// The `.chip` multi-select row (`docs/ux/desktop-mockups/16-settings/README.md`'s Dialog
/// components: `display:flex; align-items:center; gap:6px; padding:6px 10px; border:1px solid
/// rgba(32,30,29,.30); font-size:12px; cursor:pointer`, selected takes the dark treatment).
fn products_field(view: &InstitutionFormViewModel, actions: &Clicks, cx: &App) -> impl IntoElement {
    div().child(field_label(view.products_label.clone())).child(
        div()
            .flex()
            .flex_wrap()
            .gap(px(8.0))
            .mt(px(4.0))
            .children(view.products.iter().map(|chip| {
                let product = chip.product;
                let id = SharedString::from(format!(
                    "add-institution-chip-{}",
                    format!("{product:?}").to_lowercase()
                ));
                let on_click = actions.send(InstitutionFormEvent::ToggleProduct(product));
                div()
                    .debug_selector({
                        let id = id.clone();
                        move || id.to_string()
                    })
                    .id(id)
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .py(px(6.0))
                    .px(px(10.0))
                    .border_1()
                    .border_color(color::border(cx))
                    .text_size(px(12.0))
                    .when(chip.checked, |this| {
                        this.bg(color::selection_background(cx))
                            .text_color(color::selection_text(cx))
                    })
                    .on_click(move |_event, window, cx| on_click(window, cx))
                    .child(chip.label.clone())
            })),
    )
}

/// The form's fields, for a test: the Name draft and the checked products (comma-joined variant
/// names).
fn readback(view: &InstitutionFormViewModel) -> Vec<(&'static str, String)> {
    vec![
        ("name", view.name.text.clone()),
        (
            "types",
            view.products
                .iter()
                .filter(|chip| chip.checked)
                .map(|chip| format!("{:?}", chip.product))
                .collect::<Vec<_>>()
                .join(","),
        ),
        (
            "usual_unit",
            chosen_label(&view.usual_unit).unwrap_or_default(),
        ),
        ("focused", format!("{:?}", view.focused)),
        ("error", view.name_error.clone().unwrap_or_default()),
    ]
}
