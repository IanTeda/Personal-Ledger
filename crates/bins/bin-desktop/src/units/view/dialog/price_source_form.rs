//! Add and Edit price source for a Custom source (16z·custom, 620px) and Edit price source for a
//! built-in (16z·edit, 480px): Name and Updates, then a Custom source's endpoint and key fields,
//! or a built-in's endpoint as read-only facts. Updates and Authentication are segmented controls
//! that `Left`/`Right` step while focused. The Presenter reads every key.
//!
//! The typed API key is the Presenter's, held as a secret, and the form draws it masked, with no
//! way to reveal it: a Custom source's under Authentication, and CoinGecko's optional Demo key on
//! its Edit Dialog. A saved key shows only as the field's placeholder, and Remove key clears it on
//! Save. The Ledger keeps it encrypted (ADR-0043).
//!
//! Under the fields, with the Ledger's price feed, a Custom source's Symbol and Test with symbol,
//! or a built-in's Test price source, then the last test's result and a Custom source's
//! collapsible Response (#1393). The footer says a test writes nothing.
//!
//! A Custom source's History section sits under the price request (#1395): its optional request,
//! a segmented Series control (a list of points or keyed by date) and the series, date and close
//! paths. A test of a source with one adds its history line under the result, and the Response
//! its history part. The body scrolls under a fixed header and footer, following the focused
//! field, since a Custom source's form is taller than a Large window.

use gpui::{AnyElement, App, ScrollHandle, SharedString, div, prelude::*, px};
use lib_shell::Hint;
use lib_units::Updates;
use lib_units::presenter::{
    ApiKeyView, AuthChoice, HistoryFields, HistoryShape, PriceSourceField, PriceSourceFormEvent,
    PriceSourceFormPresenter, PriceSourceFormViewModel, TestState, TestView,
};
use lib_viper::view::{Choice, SelectView, TextFieldView};

use super::{help, refusal};
use crate::shell::view::dialog_entity::{DialogView, ModuleDialogClicks, framed};
use crate::{
    dialog,
    form::fields::{field_label, segmented_labelled, text_field},
    theme::color,
};

/// `width:620px` (16z·custom).
const CUSTOM_WIDTH: gpui::Pixels = px(620.0);
/// `width:480px` (16z·edit).
const BUILT_IN_WIDTH: gpui::Pixels = px(480.0);
const MONOSPACE: &str = "monospace";
/// The body's scroll cap, under which the header and footer stay put.
const BODY_MAX_HEIGHT: gpui::Pixels = px(560.0);

/// The body's scroll, and the field it last followed, so focus moving down the form scrolls that
/// field's group into view and a click on one already shown doesn't jump.
#[derive(Default)]
pub struct PriceSourceWidgets {
    scroll: ScrollHandle,
    followed: Option<PriceSourceField>,
    opened: bool,
}

/// The open form's clicks.
type Clicks = ModuleDialogClicks<PriceSourceFormPresenter>;

/// The `lib-units` price source form Presenter's [`DialogView`], hosted in a `ModuleDialog`
/// (ADR-0056).
impl DialogView for PriceSourceFormPresenter {
    type Widgets = PriceSourceWidgets;

    fn kind(view: &PriceSourceFormViewModel) -> &'static str {
        match (&view.endpoint, &view.usage) {
            (None, _) => "EditBuiltInPriceSource",
            (Some(_), None) => "AddPriceSource",
            (Some(_), Some(_)) => "EditPriceSource",
        }
    }

    /// Scrolls the focused field's group into view before drawing, once focus moves.
    fn render(
        view: &PriceSourceFormViewModel,
        widgets: &mut PriceSourceWidgets,
        clicks: &Clicks,
        cx: &App,
    ) -> AnyElement {
        let field = focused(view);
        // The form opens at its top; only a move of focus scrolls.
        if widgets.opened
            && field != widgets.followed
            && let Some(group) = field.and_then(|field| group_of(view, field))
        {
            widgets.scroll.scroll_to_item(group);
        }
        widgets.opened = true;
        widgets.followed = field;
        render(view, &widgets.scroll, clicks, cx)
    }

    fn readback(view: &PriceSourceFormViewModel) -> Vec<(&'static str, String)> {
        framed(&view.frame, readback(view))
    }

    fn hints(view: &PriceSourceFormViewModel) -> Vec<Hint> {
        view.hints.clone()
    }
}

/// The field with focus, as the view model draws it.
fn focused(view: &PriceSourceFormViewModel) -> Option<PriceSourceField> {
    let text_focus = |field: &TextFieldView| field.focused;
    if view.name.focused {
        return Some(PriceSourceField::Name);
    }
    if view.updates.focused {
        return Some(PriceSourceField::Updates);
    }
    if let Some(api_key) = &view.api_key {
        if api_key.field.focused {
            return Some(PriceSourceField::ApiKey);
        }
        if api_key.remove_focused {
            return Some(PriceSourceField::RemoveKey);
        }
    }
    if let Some(test) = &view.test {
        if test.focused {
            return Some(PriceSourceField::Test);
        }
        if test.symbol.as_ref().is_some_and(text_focus) {
            return Some(PriceSourceField::Symbol);
        }
        let response = test
            .result
            .as_ref()
            .and_then(|result| result.response.as_ref());
        if response.is_some_and(|response| response.focused) {
            return Some(PriceSourceField::Response);
        }
    }
    let endpoint = view.endpoint.as_ref()?;
    [
        (text_focus(&endpoint.address), PriceSourceField::Address),
        (text_focus(&endpoint.request), PriceSourceField::Request),
        (
            text_focus(&endpoint.price_path),
            PriceSourceField::PricePath,
        ),
        (text_focus(&endpoint.headers), PriceSourceField::Headers),
        (endpoint.auth.focused, PriceSourceField::Auth),
        (
            endpoint.key_param.as_ref().is_some_and(text_focus),
            PriceSourceField::KeyParam,
        ),
        (
            text_focus(&endpoint.history.request),
            PriceSourceField::HistoryRequest,
        ),
        (
            endpoint.history.shape.focused,
            PriceSourceField::SeriesShape,
        ),
        (
            text_focus(&endpoint.history.series_path),
            PriceSourceField::SeriesPath,
        ),
        (
            text_focus(&endpoint.history.date_path),
            PriceSourceField::DatePath,
        ),
        (
            text_focus(&endpoint.history.price_path),
            PriceSourceField::HistoryPricePath,
        ),
    ]
    .into_iter()
    .find_map(|(is_focused, field)| is_focused.then_some(field))
}

/// Which of a Custom source's body groups, in [`render_custom`]'s order, draws `field`; `None`
/// for a built-in's form, which is short enough not to scroll.
fn group_of(view: &PriceSourceFormViewModel, field: PriceSourceField) -> Option<usize> {
    view.endpoint.as_ref()?;
    // Remove key and its note take a group of their own only while there is a key field.
    let key_group = usize::from(view.api_key.is_some());
    Some(match field {
        PriceSourceField::Name | PriceSourceField::Updates => 0,
        PriceSourceField::Address => 1,
        PriceSourceField::Request => 2,
        PriceSourceField::PricePath => 3,
        PriceSourceField::Headers => 4,
        PriceSourceField::Auth | PriceSourceField::KeyParam | PriceSourceField::ApiKey => 5,
        PriceSourceField::RemoveKey => 5 + key_group,
        PriceSourceField::HistoryRequest
        | PriceSourceField::SeriesShape
        | PriceSourceField::SeriesPath
        | PriceSourceField::DatePath
        | PriceSourceField::HistoryPricePath => 6 + key_group,
        PriceSourceField::Symbol | PriceSourceField::Test | PriceSourceField::Response => {
            7 + key_group
        }
    })
}

/// A segmented control over a select's choices, each click sending `event` of the choice.
fn segmented<T: Copy + PartialEq + 'static>(
    id_prefix: &'static str,
    select: &SelectView<T>,
    actions: &Clicks,
    event: fn(T) -> PriceSourceFormEvent,
    cx: &App,
) -> AnyElement {
    let Some(current) = select.chosen else {
        return div().into_any_element();
    };
    let options = select
        .options
        .iter()
        .map(|Choice { value, label }| (*value, SharedString::from(label.clone())))
        .collect();
    div()
        .flex_none()
        .child(field_label(select.label.clone()))
        .child(segmented_labelled(
            id_prefix,
            options,
            current,
            actions.choose(move |value: T| vec![event(value)]),
            None,
            cx,
        ))
        .into_any_element()
}

/// A label above a clickable box, accent-bordered with a trailing caret while focused, and
/// monospace for the endpoint's parts.
fn text_box(
    id: &'static str,
    field: &TextFieldView,
    which: PriceSourceField,
    mono: bool,
    actions: &Clicks,
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
    let on_click = actions.send(PriceSourceFormEvent::Focus(which));
    div()
        .child(field_label(field.label.clone()))
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
                .text_size(px(13.0))
                .when(mono, |this| this.font_family(MONOSPACE))
                .text_color(text_color)
                .on_click(move |_event, window, cx| on_click(window, cx))
                .child(text),
        )
        .into_any_element()
}

fn render(
    view: &PriceSourceFormViewModel,
    scroll: &ScrollHandle,
    actions: &Clicks,
    cx: &App,
) -> AnyElement {
    match &view.endpoint {
        Some(_) => render_custom(view, scroll, actions, cx),
        None => render_built_in(view, scroll, actions, cx),
    }
}

/// The groups of a Custom source's body, in the order [`group_of`] counts them.
fn render_custom(
    view: &PriceSourceFormViewModel,
    scroll: &ScrollHandle,
    actions: &Clicks,
    cx: &App,
) -> AnyElement {
    let prefix = if view.usage.is_some() {
        "edit-price-source"
    } else {
        "add-price-source"
    };
    let Some(endpoint) = &view.endpoint else {
        return div().into_any_element();
    };
    let text =
        |id, field: &TextFieldView, which, mono| text_box(id, field, which, mono, actions, cx);
    let mut fields = vec![
        div()
            .flex()
            .items_start()
            .gap(px(16.0))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .child(text(
                        "price-source-name",
                        &view.name,
                        PriceSourceField::Name,
                        false,
                    ))
                    .child(help(view.name_help.clone(), cx)),
            )
            .child(segmented(
                "price-source-updates",
                &view.updates,
                actions,
                PriceSourceFormEvent::Updates as fn(Updates) -> PriceSourceFormEvent,
                cx,
            ))
            .into_any_element(),
        text(
            "price-source-address",
            &endpoint.address,
            PriceSourceField::Address,
            true,
        ),
        div()
            .child(text(
                "price-source-request",
                &endpoint.request,
                PriceSourceField::Request,
                true,
            ))
            .child(help(endpoint.request_help.clone(), cx))
            .into_any_element(),
        div()
            .child(text(
                "price-source-price-path",
                &endpoint.price_path,
                PriceSourceField::PricePath,
                true,
            ))
            .child(help(endpoint.price_path_help.clone(), cx))
            .into_any_element(),
        div()
            .child(text(
                "price-source-headers",
                &endpoint.headers,
                PriceSourceField::Headers,
                true,
            ))
            .child(help(endpoint.headers_help.clone(), cx))
            .into_any_element(),
        div()
            .flex()
            .items_start()
            .gap(px(12.0))
            .child(segmented(
                "price-source-auth",
                &endpoint.auth,
                actions,
                PriceSourceFormEvent::Auth as fn(AuthChoice) -> PriceSourceFormEvent,
                cx,
            ))
            .children(endpoint.key_param.as_ref().map(|key_param| {
                div().flex_1().min_w(px(0.0)).child(text(
                    "price-source-key-param",
                    key_param,
                    PriceSourceField::KeyParam,
                    true,
                ))
            }))
            .children(view.api_key.as_ref().map(|api_key| {
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .child(key_box(api_key, actions, cx))
            }))
            .into_any_element(),
    ];
    if let Some(api_key) = &view.api_key {
        fields.push(key_remove_and_note(api_key, actions, cx));
    }
    fields.push(history_section(&endpoint.history, actions, cx));
    fields.extend(
        view.test
            .as_ref()
            .map(|test| test_section(test, actions, cx)),
    );
    fields.extend(view.usage.clone().map(|usage| {
        div()
            .debug_selector(|| "edit-price-source-usage".to_string())
            .child(dialog::info_panel(usage, cx))
            .into_any_element()
    }));
    fields.extend(refusal(view.refusal.clone(), cx));
    frame(view, prefix, CUSTOM_WIDTH, fields, scroll, actions, cx)
}

fn render_built_in(
    view: &PriceSourceFormViewModel,
    scroll: &ScrollHandle,
    actions: &Clicks,
    cx: &App,
) -> AnyElement {
    let mut fields = vec![
        div()
            .child(text_field(
                "edit-built-in-source-name",
                view.name.label.clone(),
                &view.name.text,
                &view.name.placeholder,
                view.name.focused,
                actions.send(PriceSourceFormEvent::Focus(PriceSourceField::Name)),
                cx,
            ))
            .child(help(view.name_help.clone(), cx))
            .into_any_element(),
        fact_block(view, cx),
        segmented(
            "edit-built-in-source-updates",
            &view.updates,
            actions,
            PriceSourceFormEvent::Updates as fn(Updates) -> PriceSourceFormEvent,
            cx,
        ),
    ];
    if let Some(api_key) = &view.api_key {
        fields.push(key_box(api_key, actions, cx));
        fields.push(key_remove_and_note(api_key, actions, cx));
    }
    fields.extend(view.usage.clone().map(|usage| {
        div()
            .debug_selector(|| "edit-built-in-source-usage".to_string())
            .child(dialog::info_panel(usage, cx))
            .into_any_element()
    }));
    fields.extend(
        view.test
            .as_ref()
            .map(|test| test_section(test, actions, cx)),
    );
    fields.extend(refusal(view.refusal.clone(), cx));
    frame(
        view,
        "edit-built-in-source",
        BUILT_IN_WIDTH,
        fields,
        scroll,
        actions,
        cx,
    )
}

/// The History section: its heading over a hairline, the request, the Series control beside the
/// series path, then the date and close paths side by side.
fn history_section(history: &HistoryFields, actions: &Clicks, cx: &App) -> AnyElement {
    let text = |id, field: &TextFieldView, which| text_box(id, field, which, true, actions, cx);
    let half = |element: AnyElement| div().flex_1().min_w(px(0.0)).child(element);
    div()
        .debug_selector(|| "price-source-history".to_string())
        .flex()
        .flex_col()
        .gap(px(12.0))
        .pt(px(12.0))
        .border_t(px(1.0))
        .border_color(color::hairline(cx))
        .child(
            div()
                .text_size(px(11.5))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_color(color::muted(cx))
                .child(history.heading.to_uppercase()),
        )
        .child(
            div()
                .child(text(
                    "price-source-history-request",
                    &history.request,
                    PriceSourceField::HistoryRequest,
                ))
                .child(help(history.request_help.clone(), cx)),
        )
        .child(
            div()
                .flex()
                .items_start()
                .gap(px(12.0))
                .child(segmented(
                    "price-source-series-shape",
                    &history.shape,
                    actions,
                    PriceSourceFormEvent::Shape as fn(HistoryShape) -> PriceSourceFormEvent,
                    cx,
                ))
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .child(text(
                            "price-source-series-path",
                            &history.series_path,
                            PriceSourceField::SeriesPath,
                        ))
                        .child(help(history.series_path_help.clone(), cx)),
                ),
        )
        .child(
            div()
                .child(
                    div()
                        .flex()
                        .gap(px(12.0))
                        .child(half(text(
                            "price-source-date-path",
                            &history.date_path,
                            PriceSourceField::DatePath,
                        )))
                        .child(half(text(
                            "price-source-history-price-path",
                            &history.price_path,
                            PriceSourceField::HistoryPricePath,
                        ))),
                )
                .child(help(history.point_paths_help.clone(), cx)),
        )
        .into_any_element()
}

/// The API key field, masked, its placeholder saying a saved key is kept. Off, it takes no click and
/// draws faint, its note saying why.
fn key_box(api_key: &ApiKeyView, actions: &Clicks, cx: &App) -> AnyElement {
    let field = text_box(
        "price-source-api-key",
        &api_key.field,
        PriceSourceField::ApiKey,
        true,
        actions,
        cx,
    );
    if api_key.enabled {
        return field;
    }
    div()
        .debug_selector(|| "price-source-api-key-off".to_string())
        .opacity(0.45)
        .occlude()
        .child(field)
        .into_any_element()
}

/// Remove key, while there is a saved key, then where the key is kept or why the field is off.
fn key_remove_and_note(api_key: &ApiKeyView, actions: &Clicks, cx: &App) -> AnyElement {
    let remove = api_key.remove.clone().map(|label| {
        let on_click = actions.send(PriceSourceFormEvent::RemoveKey);
        div()
            .id("price-source-remove-key")
            .debug_selector(|| "price-source-remove-key".to_string())
            .flex_none()
            .cursor_pointer()
            .py(px(5.0))
            .px(px(10.0))
            .border_1()
            .border_color(if api_key.remove_focused {
                color::accent(cx)
            } else {
                color::border(cx)
            })
            .text_size(px(12.0))
            .font_weight(gpui::FontWeight::BOLD)
            .on_click(move |_event, window, cx| on_click(window, cx))
            .child(label)
    });
    div()
        .flex()
        .items_center()
        .gap(px(10.0))
        .children(remove)
        .child(
            div()
                .debug_selector(|| "price-source-key-note".to_string())
                .text_size(px(11.5))
                .text_color(color::faint_text(cx))
                .child(api_key.note.clone()),
        )
        .into_any_element()
}

/// The test: a Custom source's Symbol, the button and what it prices or why it is off, then the
/// last result and a Custom source's Response.
fn test_section(test: &TestView, actions: &Clicks, cx: &App) -> AnyElement {
    let on_click = actions.send(PriceSourceFormEvent::Test);
    let enabled = test.enabled;
    let button = div()
        .id("price-source-test")
        .debug_selector(|| "price-source-test".to_string())
        .flex_none()
        .py(px(7.0))
        .px(px(12.0))
        .border_1()
        .border_color(if test.focused {
            color::accent(cx)
        } else {
            color::border(cx)
        })
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(12.0))
        .when(enabled, |this| {
            this.cursor_pointer()
                .on_click(move |_event, window, cx| on_click(window, cx))
        })
        .when(!enabled, |this| this.opacity(0.45))
        .child(test.button.clone());
    let row = div()
        .flex()
        .items_end()
        .gap(px(12.0))
        .children(test.symbol.as_ref().map(|symbol| {
            div().w(px(180.0)).flex_none().child(text_box(
                "price-source-symbol",
                symbol,
                PriceSourceField::Symbol,
                true,
                actions,
                cx,
            ))
        }))
        .child(button)
        .child(
            div()
                .debug_selector(|| "price-source-test-help".to_string())
                .min_w(px(0.0))
                .pb(px(8.0))
                .text_size(px(11.5))
                .text_color(color::faint_text(cx))
                .child(test.help.clone()),
        );
    let result = test.result.as_ref().map(|result| {
        let heading_color = match result.state {
            TestState::Found => color::positive_text(cx),
            TestState::Failed => color::negative_text(cx),
            TestState::Testing => color::muted(cx),
        };
        let response = result.response.as_ref().map(|response| {
            let toggle = actions.send(PriceSourceFormEvent::ToggleResponse);
            let caret = if response.open {
                "\u{25be}"
            } else {
                "\u{25b8}"
            };
            div()
                .flex()
                .flex_col()
                .gap(px(4.0))
                .mt(px(6.0))
                .child(
                    div()
                        .id("price-source-response")
                        .debug_selector(|| "price-source-response".to_string())
                        .cursor_pointer()
                        .text_size(px(12.0))
                        .text_color(if response.focused {
                            color::accent(cx)
                        } else {
                            color::muted(cx)
                        })
                        .on_click(move |_event, window, cx| toggle(window, cx))
                        .child(format!("{caret} {}", response.label)),
                )
                .when(response.open, |this| {
                    this.child(response_body(
                        "price-source-response-body",
                        &response.body,
                        cx,
                    ))
                    .children(response.history.as_ref().map(|history| {
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(color::muted(cx))
                                    .child(history.label.clone()),
                            )
                            .child(response_body(
                                "price-source-history-response-body",
                                &history.body,
                                cx,
                            ))
                    }))
                })
        });
        let history = result.history.as_ref().map(|history| {
            div()
                .debug_selector(|| "price-source-test-history".to_string())
                .mt(px(2.0))
                .text_size(px(12.0))
                .font_family(MONOSPACE)
                .text_color(match history.state {
                    TestState::Found | TestState::Testing => color::muted(cx),
                    TestState::Failed => color::negative_text(cx),
                })
                .child(history.text.clone())
        });
        div()
            .debug_selector(|| "price-source-test-result".to_string())
            .flex()
            .flex_col()
            .px(px(12.0))
            .py(px(8.0))
            .border_1()
            .border_color(color::hairline(cx))
            .child(
                div()
                    .flex()
                    .gap(px(8.0))
                    .font_weight(gpui::FontWeight::BOLD)
                    .text_color(heading_color)
                    .child(result.heading.clone())
                    .children(result.unsaved.clone().map(|tag| {
                        div()
                            .font_weight(gpui::FontWeight::NORMAL)
                            .text_color(color::muted(cx))
                            .child(tag)
                    })),
            )
            .when(!result.detail.is_empty(), |this| {
                this.child(
                    div()
                        .mt(px(2.0))
                        .text_size(px(12.0))
                        .text_color(color::muted(cx))
                        .font_family(MONOSPACE)
                        .child(result.detail.clone()),
                )
            })
            .children(history)
            .children(response)
    });
    div()
        .flex()
        .flex_col()
        .gap(px(8.0))
        .child(row)
        .children(result)
        .into_any_element()
}

/// A response body, one line per row, monospace on the chrome.
fn response_body(id: &'static str, body: &str, cx: &App) -> impl IntoElement {
    div()
        .debug_selector(move || id.to_string())
        .p(px(8.0))
        .bg(color::chrome(cx))
        .font_family(MONOSPACE)
        .text_size(px(11.5))
        .flex()
        .flex_col()
        .children(body.lines().map(|line| div().child(line.to_string())))
}

/// The card: title, the fields, Cancel and the submit.
fn frame(
    view: &PriceSourceFormViewModel,
    prefix: &'static str,
    width: gpui::Pixels,
    fields: Vec<AnyElement>,
    scroll: &ScrollHandle,
    actions: &Clicks,
    cx: &App,
) -> AnyElement {
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(view.frame.title.clone(), false, cx))
        .child(
            div()
                .id("price-source-body")
                .debug_selector(|| "price-source-body".to_string())
                .max_h(BODY_MAX_HEIGHT)
                .overflow_y_scroll()
                .track_scroll(scroll)
                .p(px(20.0))
                .flex()
                .flex_col()
                .gap(px(12.0))
                .children(fields),
        )
        .child(dialog::action_row(
            [
                div()
                    .debug_selector(|| "price-source-test-note".to_string())
                    .mr_auto()
                    .text_size(px(11.5))
                    .text_color(color::faint_text(cx))
                    .children(view.test.as_ref().map(|test| test.note.clone()))
                    .into_any_element(),
                dialog::cancel_button(
                    SharedString::from(format!("{prefix}-cancel")),
                    actions.cancel.clone(),
                    cx,
                )
                .into_any_element(),
                dialog::confirm_button(
                    SharedString::from(format!("{prefix}-confirm")),
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
    dialog::overlay(width, false, card, cx)
}

/// A built-in's endpoint, read-only: fact rows as the X-Large pane's, the URL parts monospace,
/// then the note on why they aren't edited here.
fn fact_block(view: &PriceSourceFormViewModel, cx: &App) -> AnyElement {
    div()
        .debug_selector(|| "edit-built-in-source-facts".to_string())
        .flex()
        .flex_col()
        .px(px(12.0))
        .py(px(8.0))
        .bg(color::chrome(cx))
        .children(
            view.built_in_facts
                .iter()
                .enumerate()
                .map(|(index, (label, value))| {
                    // The first fact names the source; the rest are its URL parts.
                    let mono = index > 0;
                    div()
                        .flex()
                        .justify_between()
                        .gap(px(12.0))
                        .py(px(5.0))
                        .border_b(px(1.0))
                        .border_color(color::hairline(cx))
                        .text_size(px(12.0))
                        .child(
                            div()
                                .flex_none()
                                .text_color(color::muted(cx))
                                .child(label.clone()),
                        )
                        .child(
                            div()
                                .min_w(px(0.0))
                                .text_align(gpui::TextAlign::Right)
                                .when(mono, |this| this.font_family(MONOSPACE))
                                .child(value.clone()),
                        )
                }),
        )
        .children(view.built_in_help.clone().map(|text| help(text, cx)))
        .into_any_element()
}

fn readback(view: &PriceSourceFormViewModel) -> Vec<(&'static str, String)> {
    let endpoint = view.endpoint.as_ref();
    let text =
        |field: Option<&TextFieldView>| field.map(|field| field.text.clone()).unwrap_or_default();
    vec![
        ("name", view.name.text.clone()),
        (
            "updates",
            super::chosen_label(&view.updates).unwrap_or_default(),
        ),
        ("address", text(endpoint.map(|e| &e.address))),
        ("request", text(endpoint.map(|e| &e.request))),
        ("price_path", text(endpoint.map(|e| &e.price_path))),
        ("headers", text(endpoint.map(|e| &e.headers))),
        (
            "auth",
            endpoint
                .and_then(|e| super::chosen_label(&e.auth))
                .unwrap_or_default(),
        ),
        (
            "key_param",
            text(endpoint.and_then(|e| e.key_param.as_ref())),
        ),
        (
            "history_request",
            text(endpoint.map(|e| &e.history.request)),
        ),
        (
            "series_shape",
            endpoint
                .and_then(|e| super::chosen_label(&e.history.shape))
                .unwrap_or_default(),
        ),
        (
            "series_path",
            text(endpoint.map(|e| &e.history.series_path)),
        ),
        ("date_path", text(endpoint.map(|e| &e.history.date_path))),
        (
            "history_price_path",
            text(endpoint.map(|e| &e.history.price_path)),
        ),
        ("api_key", text(view.api_key.as_ref().map(|key| &key.field))),
        (
            "api_key_placeholder",
            view.api_key
                .as_ref()
                .map(|key| key.field.placeholder.clone())
                .unwrap_or_default(),
        ),
        (
            "api_key_enabled",
            view.api_key
                .as_ref()
                .map(|key| key.enabled.to_string())
                .unwrap_or_default(),
        ),
        (
            "remove_key",
            view.api_key
                .as_ref()
                .and_then(|key| key.remove.clone())
                .unwrap_or_default(),
        ),
        (
            "key_note",
            view.api_key
                .as_ref()
                .map(|key| key.note.clone())
                .unwrap_or_default(),
        ),
        (
            "field",
            focused(view)
                .map(|field| format!("{field:?}"))
                .unwrap_or_default(),
        ),
        ("usage", view.usage.clone().unwrap_or_default()),
        ("refusal", view.refusal.clone().unwrap_or_default()),
    ]
    .into_iter()
    .chain(test_readback(view.test.as_ref()))
    .collect()
}

/// The test's state as labelled values, empty while the form offers no test.
fn test_readback(test: Option<&TestView>) -> Vec<(&'static str, String)> {
    let Some(test) = test else {
        return Vec::new();
    };
    let result = test.result.as_ref();
    let response = result.and_then(|result| result.response.as_ref());
    vec![
        ("test", test.button.clone()),
        ("test_enabled", test.enabled.to_string()),
        ("test_help", test.help.clone()),
        (
            "symbol",
            test.symbol
                .as_ref()
                .map(|symbol| symbol.text.clone())
                .unwrap_or_default(),
        ),
        (
            "test_heading",
            result
                .map(|result| result.heading.clone())
                .unwrap_or_default(),
        ),
        (
            "test_detail",
            result
                .map(|result| result.detail.clone())
                .unwrap_or_default(),
        ),
        (
            "test_unsaved",
            result
                .and_then(|result| result.unsaved.clone())
                .unwrap_or_default(),
        ),
        (
            "response",
            response
                .map(|response| response.label.clone())
                .unwrap_or_default(),
        ),
        (
            "response_open",
            response
                .map(|response| response.open.to_string())
                .unwrap_or_default(),
        ),
        (
            "response_body",
            response
                .map(|response| response.body.clone())
                .unwrap_or_default(),
        ),
        (
            "test_history",
            result
                .and_then(|result| result.history.as_ref())
                .map(|history| history.text.clone())
                .unwrap_or_default(),
        ),
        (
            "history_response",
            response
                .and_then(|response| response.history.as_ref())
                .map(|history| history.label.clone())
                .unwrap_or_default(),
        ),
        ("test_note", test.note.clone()),
    ]
}
