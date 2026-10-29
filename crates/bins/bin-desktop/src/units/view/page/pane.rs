//! The detail pane docked beside the Units page at X-Large (DESKTOP-125, DESKTOP-140), drawn from
//! the page view model's [`DetailPane`]: each group a caps label over a 2px rule, its buttons
//! sending their events to the page.

use gpui::{AnyElement, App, Entity, SharedString, div, prelude::*, px};
use lib_units::presenter::{
    AccountLine, AccountsGroup, ActivityGroup, DetailPane, Fact, PaneButton, PaneGroup,
    SourceGroup, UnitGroup, ValueLine, ValuesGroup,
};

use super::{UnitsPage, render::send};
use crate::theme::color;

/// `width:400px` at the View's right edge.
const PANE_WIDTH: gpui::Pixels = px(400.0);
/// Every pane button is 112 x 34px, so three fill the 368px row with 8px gaps and a fourth wraps.
const PANE_BUTTON_WIDTH: gpui::Pixels = px(112.0);
const PANE_BUTTON_HEIGHT: gpui::Pixels = px(34.0);
const MONOSPACE: &str = "monospace";

pub fn detail_pane(pane: &DetailPane, entity: &Entity<UnitsPage>, cx: &App) -> AnyElement {
    div()
        .debug_selector(|| "units-detail-pane".to_string())
        .id("units-detail-pane")
        .w(PANE_WIDTH)
        .flex_none()
        .h_full()
        .overflow_y_scroll()
        .flex()
        .flex_col()
        .bg(color::chrome(cx))
        .border_l(px(2.0))
        .border_color(color::structural_rule(cx))
        .children(pane.groups.iter().map(|group| match group {
            PaneGroup::Unit(unit) => unit_group(unit, entity, cx),
            PaneGroup::Source(source) => source_group(source, entity, cx),
            PaneGroup::Accounts(accounts) => accounts_group(accounts, cx),
            PaneGroup::Values(values) => values_group(values, cx),
            PaneGroup::Activity(activity) => activity_group(activity, cx),
        }))
        .into_any_element()
}

/// A group: a caps label over a 2px rule, then the body.
fn pane_group(label: &str, body: impl IntoElement, slug: &'static str, cx: &App) -> AnyElement {
    div()
        .debug_selector(move || format!("units-pane-{slug}"))
        .flex()
        .flex_col()
        .child(
            div()
                .px(px(16.0))
                .pt(px(14.0))
                .pb(px(8.0))
                .border_b(px(2.0))
                .border_color(color::structural_rule(cx))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(10.0))
                .child(lib_locale::format::upper(label)),
        )
        .child(
            div()
                .px(px(16.0))
                .pt(px(12.0))
                .pb(px(16.0))
                .flex()
                .flex_col()
                .child(body),
        )
        .into_any_element()
}

/// A fact row: the label muted, the value right-aligned and free to wrap, so a long URL part
/// stays in the pane.
fn fact(fact: &Fact, cx: &App) -> impl IntoElement {
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
                .child(fact.label.clone()),
        )
        .child(
            div()
                .min_w(px(0.0))
                .text_align(gpui::TextAlign::Right)
                .when(fact.mono, |this| this.font_family(MONOSPACE))
                .child(fact.value.clone()),
        )
}

/// A group's 15px title and its 11.5px muted meta line.
fn title(title: String, meta: Option<String>, cx: &App) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .mb(px(8.0))
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(15.0))
                .line_height(gpui::relative(1.25))
                .child(title),
        )
        .children(meta.map(|meta| {
            div()
                .mt(px(2.0))
                .text_size(px(11.5))
                .text_color(color::muted(cx))
                .child(meta)
        }))
}

/// An 11px faint note under a group's rows.
fn note(text: String, cx: &App) -> impl IntoElement {
    div()
        .mt(px(6.0))
        .text_size(px(11.0))
        .text_color(color::faint_text(cx))
        .child(text)
}

fn buttons(buttons: &[PaneButton], entity: &Entity<UnitsPage>, cx: &App) -> impl IntoElement {
    div()
        .flex()
        .flex_wrap()
        .gap(px(8.0))
        .mt(px(12.0))
        .children(buttons.iter().map(|button| {
            let id = SharedString::from(format!("units-pane-{}", button.slug));
            let on_click = send(entity, button.event.clone());
            div()
                .debug_selector({
                    let id = id.clone();
                    move || id.to_string()
                })
                .id(id)
                .cursor_pointer()
                .w(PANE_BUTTON_WIDTH)
                .h(PANE_BUTTON_HEIGHT)
                .flex()
                .items_center()
                .px(px(10.0))
                .border_1()
                .border_color(color::border(cx))
                .bg(color::background(cx))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(12.0))
                .whitespace_nowrap()
                .on_click(move |_event, window, cx| on_click(window, cx))
                .child(button.label.clone())
        }))
}

fn unit_group(group: &UnitGroup, entity: &Entity<UnitsPage>, cx: &App) -> AnyElement {
    let body = div()
        .flex()
        .flex_col()
        .child(title(group.title.clone(), Some(group.meta.clone()), cx))
        .children(group.facts.iter().map(|row| fact(row, cx)))
        .child(buttons(&group.buttons, entity, cx));
    pane_group(&group.heading, body, "unit", cx)
}

fn source_group(group: &SourceGroup, entity: &Entity<UnitsPage>, cx: &App) -> AnyElement {
    let body = div()
        .flex()
        .flex_col()
        .children(
            group
                .title
                .clone()
                .map(|text| title(text, group.meta.clone(), cx)),
        )
        .children(group.facts.iter().map(|row| fact(row, cx)))
        .children(group.note.clone().map(|text| note(text, cx)))
        .when(!group.buttons.is_empty(), |this| {
            this.child(buttons(&group.buttons, entity, cx))
        });
    pane_group(&group.heading, body, "source", cx)
}

fn faint_line(text: String, cx: &App) -> AnyElement {
    div()
        .text_size(px(11.0))
        .text_color(color::faint_text(cx))
        .child(text)
        .into_any_element()
}

/// `code · name` over `symbol … · quoted in …` (the symbol monospace); the price over its age.
fn value_row(value: &ValueLine, cx: &App) -> impl IntoElement {
    let faint = color::faint_text(cx);
    let detail = div()
        .flex()
        .flex_wrap()
        .gap(px(4.0))
        .text_size(px(11.0))
        .text_color(faint)
        .when_some(value.symbol.clone(), |this, (label, symbol)| {
            this.child(label)
                .child(div().font_family(MONOSPACE).child(symbol))
                .child("\u{b7}")
        })
        .child(value.quoted_in.clone());
    div()
        .flex()
        .justify_between()
        .gap(px(12.0))
        .py(px(6.0))
        .border_b(px(1.0))
        .border_color(color::hairline(cx))
        .text_size(px(12.0))
        .child(
            div()
                .min_w(px(0.0))
                .flex()
                .flex_col()
                .child(
                    div()
                        .flex()
                        .gap(px(4.0))
                        .child(
                            div()
                                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                                .child(value.code.clone()),
                        )
                        .child("\u{b7}")
                        .child(value.name.clone()),
                )
                .child(detail),
        )
        .child(
            div()
                .flex_none()
                .flex()
                .flex_col()
                .items_end()
                .child(value.price.clone())
                .children(
                    value
                        .age
                        .clone()
                        .map(|age| div().text_size(px(11.0)).text_color(faint).child(age)),
                ),
        )
}

fn values_group(group: &ValuesGroup, cx: &App) -> AnyElement {
    let body = match &group.empty {
        Some(empty) => faint_line(empty.clone(), cx),
        None => div()
            .flex()
            .flex_col()
            .children(group.rows.iter().map(|row| value_row(row, cx)))
            .children(group.note.clone().map(|text| note(text, cx)))
            .into_any_element(),
    };
    pane_group(&group.heading, body, "values", cx)
}

/// The balance over its value in the quote Unit, right-aligned.
fn balance(line: &AccountLine, cx: &App) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .items_end()
        .child(line.balance.clone())
        .children(line.value.clone().map(|value| {
            div()
                .text_size(px(11.0))
                .text_color(color::faint_text(cx))
                .font_weight(gpui::FontWeight::NORMAL)
                .child(value)
        }))
}

/// Each Account in the Unit, name over type on the left and the balance on the right, then a
/// bold **Total**.
fn accounts_group(group: &AccountsGroup, cx: &App) -> AnyElement {
    let body = match &group.empty {
        Some(empty) => faint_line(empty.clone(), cx),
        None => div()
            .flex()
            .flex_col()
            .children(group.rows.iter().map(|line| {
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
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .font_weight(gpui::FontWeight::EXTRA_BOLD)
                                    .child(line.name.clone()),
                            )
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(color::faint_text(cx))
                                    .child(line.kind.clone()),
                            ),
                    )
                    .child(balance(line, cx))
            }))
            .children(group.total.as_ref().map(|total| {
                div()
                    .flex()
                    .justify_between()
                    .gap(px(12.0))
                    .pt(px(8.0))
                    .text_size(px(12.0))
                    .font_weight(gpui::FontWeight::EXTRA_BOLD)
                    .child(total.name.clone())
                    .child(balance(total, cx))
            }))
            .into_any_element(),
    };
    pane_group(&group.heading, body, "accounts", cx)
}

/// This session's outcomes, newest first, each a 12px row with a 38px tabular time.
fn activity_group(group: &ActivityGroup, cx: &App) -> AnyElement {
    let body = div()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .children(group.lines.iter().map(|line| {
            div()
                .flex()
                .gap(px(8.0))
                .text_size(px(12.0))
                .child(
                    div()
                        .w(px(38.0))
                        .flex_none()
                        .text_color(color::muted(cx))
                        .child(line.time.clone()),
                )
                .child(div().child(line.text.clone()))
        }))
        .child(note(group.note.clone(), cx));
    pane_group(&group.heading, body, "activity", cx)
}
