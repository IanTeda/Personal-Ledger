//! The Add unit and Edit unit Dialogs (16c·src): Code and Type, Name, then the PRICE SOURCE
//! group, whose Symbol, Quoted in and Test price hide on Manual entry.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{AnyElement, App, div, prelude::*, px};
use lib_units::presenter::{UnitField, UnitFormEvent, UnitFormPresenter, UnitFormViewModel};
use lib_viper::view::{SelectView, TextFieldView};

use super::{
    Shared, UnitsDialog, View, choose_option, enter, help, refusal, select_key, send, synced,
    test_not_built, toggle_select, typed,
};
use crate::{
    chrome::dialog_host::{DialogActions, DialogKey, DialogOutcome},
    dialog,
    form::{
        dialog_fields::{label, text_field, two_up},
        select::SelectState,
        select_field::{self, SelectFieldProps},
    },
    theme::color,
};

/// The Symbol box holds a source's own id, so it is drawn in the monospace face.
const MONOSPACE: &str = "monospace";

/// Whether the form edits a Unit, and the open state of its three selects.
#[derive(Debug, Clone, Default)]
pub(super) struct UnitSelects {
    editing: bool,
    kind: SelectState,
    source: SelectState,
    quote: SelectState,
}

impl UnitSelects {
    /// Closes every list. Returns whether one was open.
    pub(super) fn close(&mut self) -> bool {
        let open = self.kind.is_open() || self.source.is_open() || self.quote.is_open();
        self.kind.cancel();
        self.source.cancel();
        self.quote.cancel();
        open
    }

    pub(super) fn editing(&self) -> bool {
        self.editing
    }

    fn any_open(&self) -> bool {
        self.kind.is_open() || self.source.is_open() || self.quote.is_open()
    }
}

pub(super) fn open(presenter: UnitFormPresenter, editing: bool) -> View {
    View::UnitForm(
        Rc::new(RefCell::new(presenter)),
        UnitSelects {
            editing,
            ..UnitSelects::default()
        },
    )
}

fn handle(presenter: &Shared<UnitFormPresenter>, event: UnitFormEvent) {
    send(presenter, event, UnitFormPresenter::handle);
}

pub(super) fn key(
    presenter: &Shared<UnitFormPresenter>,
    selects: &mut UnitSelects,
    key: DialogKey,
) -> DialogOutcome {
    let view = presenter.borrow().view_model();
    let focused = view
        .fields
        .iter()
        .copied()
        .find(|field| focused(&view, *field));
    match key {
        DialogKey::Tab | DialogKey::BackTab => {
            selects.close();
            let event = if key == DialogKey::Tab {
                UnitFormEvent::NextField
            } else {
                UnitFormEvent::PrevField
            };
            handle(presenter, event);
            return DialogOutcome::Handled;
        }
        DialogKey::Enter | DialogKey::CtrlEnter if !selects.any_open() && !is_select(focused) => {
            return enter(view.frame.can_confirm);
        }
        _ => {}
    }
    match focused {
        Some(UnitField::Type) => {
            let (taken, value) = select_key(&mut selects.kind, &view.kind, key);
            if let Some(kind) = value {
                handle(presenter, UnitFormEvent::Kind(kind));
            }
            outcome(taken || matches!(key, DialogKey::Char(_)))
        }
        Some(UnitField::Source) => {
            let (taken, value) = select_key(&mut selects.source, &view.source, key);
            if let Some(id) = value {
                handle(presenter, UnitFormEvent::Source(id));
            }
            outcome(taken || matches!(key, DialogKey::Char(_)))
        }
        Some(UnitField::Quote) => match &view.quote {
            Some(quote) => {
                let (taken, value) = select_key(&mut selects.quote, quote, key);
                if let Some(id) = value {
                    handle(presenter, UnitFormEvent::Quote(id));
                }
                outcome(taken || matches!(key, DialogKey::Char(_)))
            }
            None => DialogOutcome::Ignored,
        },
        Some(field @ (UnitField::Code | UnitField::Name | UnitField::Symbol)) => {
            let text = text_of(&view, field).unwrap_or_default();
            match typed(&text, key) {
                Some(text) => {
                    let event = match field {
                        UnitField::Code => UnitFormEvent::Code(text),
                        UnitField::Name => UnitFormEvent::Name(text),
                        _ => UnitFormEvent::Symbol(text),
                    };
                    handle(presenter, event);
                    DialogOutcome::Handled
                }
                None => DialogOutcome::Ignored,
            }
        }
        None => DialogOutcome::Ignored,
    }
}

fn outcome(taken: bool) -> DialogOutcome {
    if taken {
        DialogOutcome::Handled
    } else {
        DialogOutcome::Ignored
    }
}

fn is_select(field: Option<UnitField>) -> bool {
    matches!(
        field,
        Some(UnitField::Type | UnitField::Source | UnitField::Quote)
    )
}

fn focused(view: &UnitFormViewModel, field: UnitField) -> bool {
    match field {
        UnitField::Code => view.code.focused,
        UnitField::Type => view.kind.focused,
        UnitField::Name => view.name.focused,
        UnitField::Source => view.source.focused,
        UnitField::Symbol => view.symbol.as_ref().is_some_and(|symbol| symbol.focused),
        UnitField::Quote => view.quote.as_ref().is_some_and(|quote| quote.focused),
    }
}

fn text_of(view: &UnitFormViewModel, field: UnitField) -> Option<String> {
    match field {
        UnitField::Code => Some(view.code.text.clone()),
        UnitField::Name => Some(view.name.text.clone()),
        UnitField::Symbol => view.symbol.as_ref().map(|symbol| symbol.text.clone()),
        UnitField::Type | UnitField::Source | UnitField::Quote => None,
    }
}

/// The Add or Edit form's element ids, so the two stay distinct.
struct Ids {
    prefix: &'static str,
    code: &'static str,
    kind: &'static str,
    name: &'static str,
    source: &'static str,
    symbol: &'static str,
    quote: &'static str,
    test: &'static str,
}

const ADD: Ids = Ids {
    prefix: "add-unit",
    code: "add-unit-code",
    kind: "add-unit-type",
    name: "add-unit-name",
    source: "add-unit-source",
    symbol: "add-unit-symbol",
    quote: "add-unit-quote",
    test: "add-unit-test",
};

const EDIT: Ids = Ids {
    prefix: "edit-unit",
    code: "edit-unit-code",
    kind: "edit-unit-type",
    name: "edit-unit-name",
    source: "edit-unit-source",
    symbol: "edit-unit-symbol",
    quote: "edit-unit-quote",
    test: "edit-unit-test",
};

/// Which of the three selects a click is on.
#[derive(Clone, Copy)]
enum Which {
    Kind,
    Source,
    Quote,
}

fn select_mut(selects: &mut UnitSelects, which: Which) -> &mut SelectState {
    match which {
        Which::Kind => &mut selects.kind,
        Which::Source => &mut selects.source,
        Which::Quote => &mut selects.quote,
    }
}

/// A click on a select's field: focuses it and toggles its list, closing the others.
fn click_select(dialog: &mut UnitsDialog, which: Which, field: UnitField) {
    let View::UnitForm(presenter, selects) = &mut dialog.view else {
        return;
    };
    handle(presenter, UnitFormEvent::Focus(field));
    let view = presenter.borrow().view_model();
    let state = select_mut(selects, which).clone();
    selects.close();
    *select_mut(selects, which) = state;
    match which {
        Which::Kind => toggle_select(&mut selects.kind, &view.kind),
        Which::Source => toggle_select(&mut selects.source, &view.source),
        Which::Quote => {
            if let Some(quote) = &view.quote {
                toggle_select(&mut selects.quote, quote);
            }
        }
    }
}

/// A click on row `index` of a select's open list.
fn click_option(dialog: &mut UnitsDialog, which: Which, index: usize) {
    let View::UnitForm(presenter, selects) = &mut dialog.view else {
        return;
    };
    let view = presenter.borrow().view_model();
    let event = match which {
        Which::Kind => choose_option(&mut selects.kind, &view.kind, index).map(UnitFormEvent::Kind),
        Which::Source => {
            choose_option(&mut selects.source, &view.source, index).map(UnitFormEvent::Source)
        }
        Which::Quote => view
            .quote
            .as_ref()
            .and_then(|quote| choose_option(&mut selects.quote, quote, index))
            .map(UnitFormEvent::Quote),
    };
    if let Some(event) = event {
        handle(presenter, event);
    }
}

fn click_text(dialog: &mut UnitsDialog, field: UnitField) {
    if let View::UnitForm(presenter, selects) = &mut dialog.view {
        selects.close();
        handle(presenter, UnitFormEvent::Focus(field));
    }
}

pub(super) fn render(
    view: &UnitFormViewModel,
    selects: &UnitSelects,
    actions: &DialogActions,
    cx: &App,
) -> AnyElement {
    let ids = if selects.editing { EDIT } else { ADD };
    let text = |id: &'static str, field: &TextFieldView, unit_field: UnitField| {
        text_field(
            id,
            label(field.label.clone()),
            &field.text,
            &field.placeholder,
            field.focused,
            UnitsDialog::on_click(actions, move |dialog| click_text(dialog, unit_field)),
            cx,
        )
    };

    let mut fields = vec![
        two_up([
            text(ids.code, &view.code, UnitField::Code),
            select_field_of(
                ids.kind,
                &view.kind,
                &selects.kind,
                (Which::Kind, UnitField::Type),
                actions,
                cx,
            ),
        ]),
        text(ids.name, &view.name, UnitField::Name),
        group_label(view.source_group.clone(), cx),
        div()
            .child(select_field_of(
                ids.source,
                &view.source,
                &selects.source,
                (Which::Source, UnitField::Source),
                actions,
                cx,
            ))
            .child(help(view.source_help.clone(), cx))
            .into_any_element(),
    ];
    if let (Some(symbol), Some(quote)) = (&view.symbol, &view.quote) {
        fields.push(two_up([
            div()
                .flex_1()
                .min_w(px(0.0))
                .child(symbol_field(ids.symbol, symbol, actions, cx))
                .children(view.symbol_help.clone().map(|text| help(text, cx)))
                .into_any_element(),
            select_field_of(
                ids.quote,
                quote,
                &selects.quote,
                (Which::Quote, UnitField::Quote),
                actions,
                cx,
            ),
        ]));
    }
    if let Some(test) = &view.test_price {
        fields.push(test_not_built(
            ids.test,
            test.clone(),
            lib_units::msg::units_price_source_test_not_built(),
            cx,
        ));
    }
    fields.extend(refusal(view.refusal.clone(), cx));

    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(view.frame.title.clone(), false, cx))
        .child(dialog::body(fields))
        .child(dialog::action_row(
            [
                dialog::cancel_button(
                    gpui::SharedString::from(format!("{}-cancel", ids.prefix)),
                    actions.cancel.clone(),
                    cx,
                )
                .into_any_element(),
                dialog::confirm_button(
                    gpui::SharedString::from(format!("{}-confirm", ids.prefix)),
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
    dialog::overlay(dialog::WIDTH, false, card, cx)
}

fn select_field_of<T: PartialEq>(
    id: &'static str,
    select: &SelectView<T>,
    state: &SelectState,
    (which, field): (Which, UnitField),
    actions: &DialogActions,
    cx: &App,
) -> AnyElement {
    let options = super::labels(select);
    select_field::render(
        SelectFieldProps {
            id,
            label: select.label.clone().into(),
            options: &options,
            state: &synced(state, select),
            focused: select.focused,
            accent: false,
            read_only: None,
            on_field_click: UnitsDialog::on_click(actions, move |dialog| {
                click_select(dialog, which, field)
            }),
            on_option_click: UnitsDialog::on_option(actions, move |dialog, index| {
                click_option(dialog, which, index)
            }),
        },
        cx,
    )
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

/// The Symbol text box, in monospace: it holds the source's own id for the Unit.
fn symbol_field(
    id: &'static str,
    field: &TextFieldView,
    actions: &DialogActions,
    cx: &App,
) -> AnyElement {
    let caret = if field.focused { "\u{2502}" } else { "" };
    let (text, text_color) = if field.text.is_empty() {
        (
            format!("{}{caret}", field.placeholder),
            color::faint_text(cx),
        )
    } else {
        (format!("{}{caret}", field.text), color::foreground(cx))
    };
    let on_click = UnitsDialog::on_click(actions, |dialog| click_text(dialog, UnitField::Symbol));
    div()
        .child(label(field.label.clone()))
        .child(
            div()
                .id(id)
                .debug_selector(move || id.to_string())
                .cursor_pointer()
                .w_full()
                .py(px(8.0))
                .px(px(10.0))
                .border_1()
                .border_color(if field.focused {
                    color::accent(cx)
                } else {
                    color::border(cx)
                })
                .font_family(MONOSPACE)
                .text_size(px(13.0))
                .text_color(text_color)
                .on_click(move |_event, window, cx| on_click(window, cx))
                .child(text),
        )
        .into_any_element()
}

pub(super) fn readback(
    view: &UnitFormViewModel,
    selects: &UnitSelects,
) -> Vec<(&'static str, String)> {
    vec![
        ("code", view.code.text.clone()),
        ("name", view.name.text.clone()),
        ("kind", super::chosen_label(&view.kind).unwrap_or_default()),
        (
            "source",
            super::chosen_label(&view.source).unwrap_or_default(),
        ),
        ("sources", super::labels(&view.source).join("|")),
        (
            "symbol",
            view.symbol
                .as_ref()
                .map(|symbol| symbol.text.clone())
                .unwrap_or_default(),
        ),
        (
            "quote",
            view.quote
                .as_ref()
                .and_then(super::chosen_label)
                .unwrap_or_default(),
        ),
        (
            "field",
            view.fields
                .iter()
                .find(|field| focused(view, **field))
                .map(|field| format!("{field:?}"))
                .unwrap_or_default(),
        ),
        (
            "visible",
            view.fields
                .iter()
                .map(|field| format!("{field:?}"))
                .collect::<Vec<_>>()
                .join("|"),
        ),
        ("select_open", selects.any_open().to_string()),
        ("refusal", view.refusal.clone().unwrap_or_default()),
    ]
}
