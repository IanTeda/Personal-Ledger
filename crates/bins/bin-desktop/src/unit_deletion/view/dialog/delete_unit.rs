//! The Delete unit Dialog (16d) in the red treatment, an Entity in the Dialog host
//! (`shell::view::dialog_entity`), with the sections that apply: the Unit that takes over as base
//! when it is the base, and a Usual unit select for each Institution that names it. A Unit
//! something else names opens as the Deactivate unit Dialog instead. Every Dialog command and typed
//! key reaches the `lib-unit-deletion` Presenter as a `lib-viper` `DialogInput`, and every click as
//! a Presenter event; this draws the view model, the focused select and its list included, and
//! nothing else.
//!
//! Keys are the Presenter's: the typed-back code always holds the typing, so the selects are
//! driven by `Up`/`Down` (`Enter` commits an open list) on the select `Tab`/`Shift-Tab` move to,
//! and by the mouse. Confirm and Cancel go to the Presenter, which closes the Dialog through the
//! host's Router; a confirmed outcome is handed to the Units page, and the host closes on
//! [`DialogClosed`].

use std::rc::Rc;

use gpui::{AnyElement, App, Window, div, prelude::*, px};
use lib_core::UnitId;
use lib_shell::Hint;
use lib_unit_deletion::presenter::{
    DeleteUnitEvent, DeleteUnitPresenter, DeleteUnitViewModel, UsualUnitSelect,
};
use lib_units::presenter::NewBaseView;
use lib_viper::view::SelectList;

use crate::dialog;
use crate::form::fields::text_field;
use crate::form::select::SelectState;
use crate::form::select_field::{self, SelectFieldProps};
use crate::form::viper::{chosen_label, drawn, labels};
use crate::shell::view::dialog_entity::{DialogView, ModuleDialogClicks};
use crate::theme::color;
use crate::units::{help, refusal};

/// The open Delete Dialog's clicks.
type Clicks = ModuleDialogClicks<DeleteUnitPresenter>;

/// A select the Dialog draws, found by its position: the New base unit first when the Unit is the
/// base, then the Usual unit of each Institution naming it, in the Ledger's order.
enum Select<'a> {
    NewBase(&'a NewBaseView),
    /// The `n`th Usual unit select.
    UsualUnit(usize, &'a UsualUnitSelect),
}

/// The select at `index`, top to bottom.
fn select(view: &DeleteUnitViewModel, index: usize) -> Option<Select<'_>> {
    let new_base = view.unit.new_base.as_ref();
    let usual_units = view
        .usual_units
        .as_ref()
        .map(|usual_units| &usual_units.selects[..]);
    match (new_base, index) {
        (Some(new_base), 0) => Some(Select::NewBase(new_base)),
        (new_base, _) => {
            let n = index - usize::from(new_base.is_some());
            Some(Select::UsualUnit(n, usual_units?.get(n)?))
        }
    }
}

/// How many selects `view` draws.
fn select_count(view: &DeleteUnitViewModel) -> usize {
    let usual_units = view
        .usual_units
        .as_ref()
        .map_or(0, |usual_units| usual_units.selects.len());
    usize::from(view.unit.new_base.is_some()) + usual_units
}

/// The event for a new Usual unit on the Institution `named` selects for.
fn usual_unit_chosen(named: &UsualUnitSelect, unit: Option<UnitId>) -> DeleteUnitEvent {
    DeleteUnitEvent::UsualUnit {
        institution: named.institution,
        unit,
    }
}

impl Select<'_> {
    fn label(&self) -> &str {
        match self {
            Self::NewBase(new_base) => &new_base.select.label,
            Self::UsualUnit(_, named) => &named.select.label,
        }
    }

    fn options(&self) -> Vec<String> {
        match self {
            Self::NewBase(new_base) => labels(&new_base.select),
            Self::UsualUnit(_, named) => labels(&named.select),
        }
    }

    /// The list as the shared dropdown draws it: open on `list` when the select is focused.
    fn drawn(&self, list: SelectList, scroll: &mut usize) -> SelectState {
        match self {
            Self::NewBase(new_base) => drawn(&new_base.select, list, scroll),
            Self::UsualUnit(_, named) => drawn(&named.select, list, scroll),
        }
    }

    /// The event a click on row `option` of the open list sends.
    fn choose(&self, option: usize) -> Option<DeleteUnitEvent> {
        match self {
            Self::NewBase(new_base) => new_base
                .select
                .options
                .get(option)
                .map(|choice| DeleteUnitEvent::NewBase(choice.value)),
            Self::UsualUnit(_, named) => {
                let unit = named.select.options.get(option)?.value;
                Some(usual_unit_chosen(named, unit))
            }
        }
    }
}

/// The `lib-unit-deletion` Presenter's [`DialogView`], hosted in a `ModuleDialog` (ADR-0056). Its
/// widget state is each select's open-list scroll window by position, a position in [`select`].
impl DialogView for DeleteUnitPresenter {
    type Widgets = Vec<usize>;

    fn kind(_view: &DeleteUnitViewModel) -> &'static str {
        "DeleteUnit"
    }

    fn render(
        view: &DeleteUnitViewModel,
        scrolls: &mut Vec<usize>,
        clicks: &Clicks,
        cx: &App,
    ) -> AnyElement {
        let count = select_count(view);
        if scrolls.len() < count {
            scrolls.resize(count, 0);
        }
        render(view, scrolls, clicks, cx)
    }

    /// The open Dialog's title, the fields it draws and whether it would confirm, for a test.
    fn readback(view: &DeleteUnitViewModel) -> Vec<(&'static str, String)> {
        readback(view)
    }

    fn hints(view: &DeleteUnitViewModel) -> Vec<Hint> {
        view.hints.clone()
    }
}

fn readback(view: &DeleteUnitViewModel) -> Vec<(&'static str, String)> {
    let unit = &view.unit;
    let usual_units = view.usual_units.as_ref();
    let focused = select(view, view.focus).map(|select| select.label().to_string());
    vec![
        ("title", unit.frame.title.clone()),
        (
            "confirm",
            unit.confirm
                .as_ref()
                .map(|confirm| confirm.text.clone())
                .unwrap_or_default(),
        ),
        ("in_use", unit.in_use.is_some().to_string()),
        (
            "in_use_text",
            unit.in_use
                .as_ref()
                .map(|in_use| in_use.text.clone())
                .unwrap_or_default(),
        ),
        (
            "new_base",
            unit.new_base
                .as_ref()
                .and_then(|new_base| chosen_label(&new_base.select))
                .unwrap_or_default(),
        ),
        (
            "new_base_options",
            unit.new_base
                .as_ref()
                .map(|new_base| labels(&new_base.select).join("|"))
                .unwrap_or_default(),
        ),
        (
            "usual_units_help",
            usual_units
                .map(|usual_units| usual_units.help.clone())
                .unwrap_or_default(),
        ),
        // `Coinspot=None|Kraken=vas — Vanguard Aus Shares`, in the Ledger's order.
        (
            "usual_units",
            usual_units
                .map(|usual_units| {
                    usual_units
                        .selects
                        .iter()
                        .map(|named| {
                            let chosen = chosen_label(&named.select).unwrap_or_default();
                            format!("{}={chosen}", named.select.label)
                        })
                        .collect::<Vec<_>>()
                        .join("|")
                })
                .unwrap_or_default(),
        ),
        (
            "usual_unit_options",
            usual_units
                .and_then(|usual_units| usual_units.selects.first())
                .map(|named| labels(&named.select).join("|"))
                .unwrap_or_default(),
        ),
        ("focused", focused.unwrap_or_default()),
        ("select_open", view.list.is_open().to_string()),
        ("refusal", unit.refusal.clone().unwrap_or_default()),
        ("can_confirm", unit.frame.can_confirm.to_string()),
    ]
}

fn render(
    view: &DeleteUnitViewModel,
    scrolls: &mut [usize],
    actions: &Clicks,
    cx: &App,
) -> AnyElement {
    let unit = &view.unit;
    let note = |text: String| {
        div()
            .text_size(px(11.5))
            .text_color(color::muted(cx))
            .child(text)
            .into_any_element()
    };
    let mut body = Vec::new();
    if let Some(in_use) = &unit.in_use {
        body.push(
            div()
                .debug_selector(|| "delete-unit-in-use".to_string())
                .text_size(px(13.0))
                .child(in_use.text.clone())
                .into_any_element(),
        );
        body.push(note(in_use.help.clone()));
        body.extend(in_use.base.clone().map(|base| {
            div()
                .debug_selector(|| "delete-unit-base-note".to_string())
                .text_size(px(11.5))
                .text_color(color::muted(cx))
                .child(base)
                .into_any_element()
        }));
    }
    // The focus ring shows which select the arrow keys drive, needed only when there are two.
    let ring = select_count(view) > 1;
    let count = select_count(view);
    for (index, scroll) in scrolls.iter_mut().enumerate().take(count) {
        let Some(select) = select(view, index) else {
            continue;
        };
        let on_field_click = actions.send(DeleteUnitEvent::ToggleSelect(index));
        // The event each row of the open list sends, read off the view as it is drawn.
        let choices: Vec<Option<DeleteUnitEvent>> = (0..select.options().len())
            .map(|row| select.choose(row))
            .collect();
        let on_option_click =
            actions.option(move |row| choices.get(row).cloned().flatten().into_iter().collect());
        let focused = ring && index == view.focus;
        let options = select.options();
        let list = if index == view.focus {
            view.list
        } else {
            SelectList::default()
        };
        let state = select.drawn(list, scroll);
        let (id, help_line) = match &select {
            Select::NewBase(new_base) => (
                "delete-unit-new-base".to_string(),
                Some(help(new_base.help.clone(), cx)),
            ),
            Select::UsualUnit(n, _) => (format!("delete-unit-usual-unit-{n}"), None),
        };
        // The Usual units' help sits once above the first of their selects.
        if let (Select::UsualUnit(0, _), Some(usual_units)) = (&select, &view.usual_units) {
            body.push(
                div()
                    .debug_selector(|| "delete-unit-usual-units-help".to_string())
                    .text_size(px(11.5))
                    .text_color(color::muted(cx))
                    .child(usual_units.help.clone())
                    .into_any_element(),
            );
        }
        let field = select_field::render(
            SelectFieldProps {
                id: id.into(),
                label: select.label().to_string().into(),
                options: &options,
                state: &state,
                focused,
                accent: false,
                read_only: None,
                on_field_click,
                on_option_click,
            },
            cx,
        );
        body.push(match help_line {
            Some(help_line) => div().child(field).child(help_line).into_any_element(),
            None => field,
        });
    }
    if let Some(confirm) = &unit.confirm {
        body.push(
            text_field(
                "delete-unit-confirm-input",
                confirm.label.clone(),
                &confirm.text,
                &confirm.placeholder,
                confirm.focused,
                Rc::new(|_window: &mut Window, _cx: &mut App| {}),
                cx,
            )
            .into_any_element(),
        );
    }
    body.extend(refusal(unit.refusal.clone(), cx));

    let destructive = unit.frame.destructive;
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(unit.frame.title.clone(), destructive, cx))
        .child(dialog::body(body))
        .child(dialog::action_row(
            [
                dialog::cancel_button("delete-unit-cancel", actions.cancel.clone(), cx)
                    .into_any_element(),
                dialog::confirm_button(
                    "delete-unit-confirm",
                    unit.frame.submit.clone(),
                    unit.frame.can_confirm,
                    destructive,
                    actions.confirm.clone(),
                    cx,
                )
                .into_any_element(),
            ],
            cx,
        ));
    dialog::overlay(dialog::WIDTH, destructive, card, cx)
}
