//! The Add unit and Edit unit Dialogs (16c·src): Code and Type, Name and Precision, then the PRICE
//! SOURCE group, whose Symbol, Quoted in and Test price hide on Manual entry. The Presenter reads
//! every key and holds the focused select's list.

use gpui::{AnyElement, App, div, prelude::*, px};
use lib_shell::Hint;
use lib_units::presenter::{UnitField, UnitFormEvent, UnitFormPresenter, UnitFormViewModel};
use lib_viper::view::{SelectList, SelectView, TextFieldView};

use super::{drawn, help, refusal, test_not_built};
use crate::shell::view::dialog_entity::{DialogView, ModuleDialogClicks, framed};
use crate::{
    dialog,
    form::{
        dialog_fields::{label, text_field, two_up},
        select_field::{self, SelectFieldProps},
    },
    theme::color,
};

/// The Symbol box holds a source's own id, so it is drawn in the monospace face.
const MONOSPACE: &str = "monospace";

/// The open form's clicks.
type Clicks = ModuleDialogClicks<UnitFormPresenter>;

/// The `lib-units` Unit form Presenter's [`DialogView`], hosted in a `ModuleDialog` (ADR-0056).
/// Its widget state is the top row of each select's list window: Type, Source, Quoted in.
impl DialogView for UnitFormPresenter {
    type Widgets = [usize; 3];

    /// Edit shows the usage notice; Add has none.
    fn kind(view: &UnitFormViewModel) -> &'static str {
        if view.usage.is_some() {
            "EditUnit"
        } else {
            "AddUnit"
        }
    }

    fn render(
        view: &UnitFormViewModel,
        scrolls: &mut [usize; 3],
        clicks: &Clicks,
        cx: &App,
    ) -> AnyElement {
        render(view, scrolls, clicks, cx)
    }

    fn readback(view: &UnitFormViewModel) -> Vec<(&'static str, String)> {
        framed(&view.frame, readback(view))
    }

    fn hints(view: &UnitFormViewModel) -> Vec<Hint> {
        view.hints.clone()
    }
}

fn focused(view: &UnitFormViewModel, field: UnitField) -> bool {
    match field {
        UnitField::Code => view.code.focused,
        UnitField::Type => view.kind.focused,
        UnitField::Name => view.name.focused,
        UnitField::Precision => view.precision.focused,
        UnitField::Source => view.source.focused,
        UnitField::Symbol => view.symbol.as_ref().is_some_and(|symbol| symbol.focused),
        UnitField::Quote => view.quote.as_ref().is_some_and(|quote| quote.focused),
    }
}

/// The Add or Edit form's element ids, so the two stay distinct.
struct Ids {
    prefix: &'static str,
    code: &'static str,
    kind: &'static str,
    name: &'static str,
    precision: &'static str,
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
    precision: "add-unit-precision",
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
    precision: "edit-unit-precision",
    source: "edit-unit-source",
    symbol: "edit-unit-symbol",
    quote: "edit-unit-quote",
    test: "edit-unit-test",
};

/// `select`'s list as the Presenter holds it: the focused select's, else closed.
fn list_of<T>(view: &UnitFormViewModel, select: &SelectView<T>) -> SelectList {
    if select.focused {
        view.list
    } else {
        SelectList::default()
    }
}

fn render(
    view: &UnitFormViewModel,
    scrolls: &mut [usize; 3],
    actions: &Clicks,
    cx: &App,
) -> AnyElement {
    let ids = if view.usage.is_some() { EDIT } else { ADD };
    let [kind_scroll, source_scroll, quote_scroll] = scrolls;
    let text = |id: &'static str, field: &TextFieldView, unit_field: UnitField| {
        text_field(
            id,
            label(field.label.clone()),
            &field.text,
            &field.placeholder,
            field.focused,
            actions.send(UnitFormEvent::Focus(unit_field)),
            cx,
        )
    };

    let mut fields = vec![
        two_up([
            text(ids.code, &view.code, UnitField::Code),
            select_field_of(
                ids.kind,
                &view.kind,
                list_of(view, &view.kind),
                kind_scroll,
                (
                    UnitField::Type,
                    view.kind
                        .options
                        .iter()
                        .map(|choice| UnitFormEvent::Kind(choice.value))
                        .collect(),
                ),
                actions,
                cx,
            ),
        ]),
        div()
            .child(two_up([
                text(ids.name, &view.name, UnitField::Name),
                text(ids.precision, &view.precision, UnitField::Precision),
            ]))
            .child(help(view.precision_help.clone(), cx))
            .into_any_element(),
        group_label(view.source_group.clone(), cx),
        div()
            .child(select_field_of(
                ids.source,
                &view.source,
                list_of(view, &view.source),
                source_scroll,
                (
                    UnitField::Source,
                    view.source
                        .options
                        .iter()
                        .map(|choice| UnitFormEvent::Source(choice.value))
                        .collect(),
                ),
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
                list_of(view, quote),
                quote_scroll,
                (
                    UnitField::Quote,
                    quote
                        .options
                        .iter()
                        .map(|choice| UnitFormEvent::Quote(choice.value))
                        .collect(),
                ),
                actions,
                cx,
            ),
        ]));
    }
    if let Some(usage) = &view.usage {
        fields.push(
            div()
                .debug_selector(|| "unit-form-usage".to_string())
                .child(help(usage.clone(), cx))
                .into_any_element(),
        );
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
    list: SelectList,
    scroll: &mut usize,
    (field, choices): (UnitField, Vec<UnitFormEvent>),
    actions: &Clicks,
    cx: &App,
) -> AnyElement {
    let options = super::labels(select);
    select_field::render(
        SelectFieldProps {
            id: id.into(),
            label: select.label.clone().into(),
            options: &options,
            state: &drawn(select, list, scroll),
            focused: select.focused,
            accent: false,
            read_only: None,
            on_field_click: actions.send(UnitFormEvent::ToggleList(field)),
            on_option_click: actions
                .option(move |row| choices.get(row).cloned().into_iter().collect()),
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
fn symbol_field(id: &'static str, field: &TextFieldView, actions: &Clicks, cx: &App) -> AnyElement {
    let caret = if field.focused { "\u{2502}" } else { "" };
    let (text, text_color) = if field.text.is_empty() {
        (
            format!("{}{caret}", field.placeholder),
            color::faint_text(cx),
        )
    } else {
        (format!("{}{caret}", field.text), color::foreground(cx))
    };
    let on_click = actions.send(UnitFormEvent::Focus(UnitField::Symbol));
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

fn readback(view: &UnitFormViewModel) -> Vec<(&'static str, String)> {
    vec![
        ("code", view.code.text.clone()),
        ("name", view.name.text.clone()),
        ("precision", view.precision.text.clone()),
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
        ("select_open", view.list.is_open().to_string()),
        ("usage", view.usage.clone().unwrap_or_default()),
        ("refusal", view.refusal.clone().unwrap_or_default()),
    ]
}
