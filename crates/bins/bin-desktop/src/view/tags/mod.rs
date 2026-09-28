//! The **7a** Tags page (`docs/ux/desktop/Tags/README.md`): a full-width management page -- a header
//! row (title, `N tags · K likely duplicates — merge them`, **+ Add tag**), a 2px rule, then one
//! bordered table (TAG / TRANSACTIONS / TOTAL / LAST USED / ACTIONS) and a footnote. The selected
//! row inverts, as in the handoff.
//!
//! Rows come in `tags::sorted_by_usage` order and every figure is computed live by `tags::usage`
//! (#355): all time, every Account, a total only when the Tag's Splits share one Unit. A likely
//! duplicate (only possible in data that broke the uniqueness rule, #354) is tinted and badged with
//! its suggested merge target; an inactive Tag stays listed, dimmed and tagged (#353). Every action
//! is a callback into `Shell`, so the keyboard and the mouse reach the same handlers.

use std::rc::Rc;

use chrono::NaiveDate;
use gpui::{AnyElement, App, Rgba, ScrollHandle, SharedString, Window, div, prelude::*, px};

use lib_core::{DateStyle, HexColor};
use lib_locale::format::upper;

use crate::{
    accounts::Account,
    nav::Noun,
    tags::{self, DuplicateGroup, Tag},
    theme::color,
    transaction_query::Total,
    transaction_rows::EMPTY_CELL,
    transactions::Transaction,
};

/// Called with a Tag's [`Tag::id`].
pub type OnTagClick = Rc<dyn Fn(u32, &mut Window, &mut App)>;
pub type OnPlainClick = Rc<dyn Fn(&mut Window, &mut App)>;

pub struct TagsPageProps<'a> {
    pub tags: &'a [Tag],
    pub transactions: &'a [Transaction],
    pub accounts: &'a [Account],
    /// The base Unit's code: a TOTAL in it shows no Unit code.
    pub base_unit: Option<&'a str>,
    pub date_style: Option<DateStyle>,
    /// Index into `tags::sorted_by_usage`'s order of the selected row.
    pub selected: Option<usize>,
    pub on_add_click: OnPlainClick,
    /// The subline's "merge them" link.
    pub on_merge_link_click: OnPlainClick,
    pub on_row_click: OnTagClick,
    /// A row's "looks like a duplicate of" badge, with that row's id.
    pub on_duplicate_click: OnTagClick,
    pub on_edit_click: OnTagClick,
    pub on_remove_click: OnTagClick,
}

/// The TAG column's floor, so the fixed columns never squeeze it to nothing.
const TAG_MIN_WIDTH: gpui::Pixels = px(140.0);
const TRANSACTIONS_WIDTH: gpui::Pixels = px(110.0);
const TOTAL_WIDTH: gpui::Pixels = px(120.0);
const LAST_USED_WIDTH: gpui::Pixels = px(110.0);
const ACTIONS_WIDTH: gpui::Pixels = px(150.0);
/// The table swatch: `10×10`, `gap 8px` before the name.
const SWATCH_SIZE: gpui::Pixels = px(10.0);

/// A Tag colour as a `gpui` colour. It is the user's data, not a Colour Theme role, so it is drawn
/// as stored under every Colour Variant (#352).
pub fn swatch_colour(colour: &HexColor) -> Rgba {
    let (r, g, b) = colour.components();
    Rgba {
        r: f32::from(r) / 255.0,
        g: f32::from(g) / 255.0,
        b: f32::from(b) / 255.0,
        a: 1.0,
    }
}

/// A `size`-square swatch, or an empty slot of the same size for a colourless Tag so names stay
/// aligned (#352).
pub fn swatch(colour: Option<&HexColor>, size: gpui::Pixels) -> impl IntoElement {
    div()
        .flex_none()
        .size(size)
        .when_some(colour, |this, colour| this.bg(swatch_colour(colour)))
}

pub fn render(
    focused: bool,
    scroll_handle: &ScrollHandle,
    props: TagsPageProps<'_>,
    cx: &App,
) -> AnyElement {
    let sorted = tags::sorted_by_usage(props.tags, props.transactions);
    let groups = tags::duplicate_groups(props.tags, props.transactions);
    let last = sorted.len().saturating_sub(1);
    div()
        .id("tags")
        .flex_1()
        .min_w(px(0.0))
        .h_full()
        .overflow_y_scroll()
        .track_scroll(scroll_handle)
        .when(focused, |this| {
            this.border_l(px(2.0)).border_color(color::foreground(cx))
        })
        .px(px(28.0))
        .py(px(22.0))
        .child(page_header(&props, &groups, cx))
        .child(
            div()
                .h(px(2.0))
                .flex_none()
                .bg(color::structural_rule(cx))
                .mt(px(24.0))
                .mb(px(24.0)),
        )
        .when(sorted.is_empty(), |this| {
            this.child(
                div()
                    .text_color(color::muted(cx))
                    .child(crate::msg::desktop_tags_empty("n")),
            )
        })
        .when(!sorted.is_empty(), |this| {
            this.child(
                div()
                    .flex()
                    .flex_col()
                    .border_1()
                    .border_color(color::border(cx))
                    .child(table_header(cx))
                    .children(sorted.iter().enumerate().map(|(index, tag)| {
                        let duplicate_of = tags::duplicate_of(&groups, tag.id)
                            .and_then(|target| tags::get(props.tags, target));
                        row(
                            tag,
                            duplicate_of,
                            index == last,
                            props.selected == Some(index),
                            &props,
                            cx,
                        )
                    })),
            )
            .child(
                div()
                    .mt(px(10.0))
                    .text_size(px(11.0))
                    .text_color(color::faint_text(cx))
                    .child(crate::msg::desktop_tags_footnote()),
            )
        })
        .into_any_element()
}

/// How many Tags are flagged as likely duplicates: every group member but its suggested target.
pub fn likely_duplicate_count(groups: &[DuplicateGroup]) -> usize {
    groups.iter().map(|group| group.duplicates.len()).sum()
}

fn page_header(props: &TagsPageProps<'_>, groups: &[DuplicateGroup], cx: &App) -> impl IntoElement {
    div()
        .flex()
        .items_end()
        .justify_between()
        .gap(px(16.0))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(6.0))
                .child(
                    div()
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .text_size(px(28.0))
                        .text_color(color::foreground(cx))
                        .child(Noun::Tags.label()),
                )
                .child(summary_line(props, groups, cx)),
        )
        .child(add_button(props.on_add_click.clone(), cx))
}

/// `9 tags · 1 likely duplicate — merge them`, the duplicate count bold in the accent text colour
/// and "merge them" a link; the duplicates clause is left out when nothing is flagged.
fn summary_line(
    props: &TagsPageProps<'_>,
    groups: &[DuplicateGroup],
    cx: &App,
) -> impl IntoElement {
    let count = i64::try_from(tags::active_count(props.tags)).unwrap_or(i64::MAX);
    let duplicates = i64::try_from(likely_duplicate_count(groups)).unwrap_or(i64::MAX);
    let on_merge = props.on_merge_link_click.clone();
    let link_hover = color::foreground(cx);
    div()
        .flex()
        .flex_wrap()
        .items_baseline()
        .gap(px(4.0))
        .text_size(px(11.5))
        .text_color(color::faint_text(cx))
        .child(crate::msg::desktop_tags_count(count))
        .when(duplicates > 0, |this| {
            this.child("\u{b7}")
                .children(
                    crate::msg::desktop_tags_likely_duplicates(duplicates)
                        .into_iter()
                        .map(|segment| match segment.tag.as_deref() {
                            Some("strong") => div()
                                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                                .text_color(color::accent_text(cx))
                                .child(segment.text)
                                .into_any_element(),
                            _ => div().child(segment.text).into_any_element(),
                        }),
                )
                .child(
                    div()
                        .id("tags-merge-link")
                        .cursor_pointer()
                        .underline()
                        .text_color(color::muted(cx))
                        .hover(move |style| style.text_color(link_hover))
                        .on_click(move |_event, window, cx| on_merge(window, cx))
                        .child(crate::msg::desktop_tags_merge_link()),
                )
        })
}

fn add_button(on_click: OnPlainClick, cx: &App) -> impl IntoElement {
    let hover = color::muted(cx);
    div()
        .id("tags-add")
        .cursor_pointer()
        .flex_none()
        .py(px(10.0))
        .px(px(16.0))
        .bg(color::foreground(cx))
        .text_color(color::selection_text(cx))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .whitespace_nowrap()
        .hover(move |style| style.bg(hover))
        .on_click(move |_event, window, cx| on_click(window, cx))
        .child(crate::msg::desktop_tags_add_button("+"))
}

/// `padding:10px 16px; background:#eae9e9; font:800 10px; color:#605d5d`.
fn table_header(cx: &App) -> impl IntoElement {
    let right = |width, label: String| {
        div()
            .w(width)
            .flex_none()
            .text_align(gpui::TextAlign::Right)
            .child(upper(&label))
    };
    div()
        .flex()
        .items_center()
        .px(px(16.0))
        .py(px(10.0))
        .bg(color::chrome(cx))
        .border_b(px(1.0))
        .border_color(color::border(cx))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(10.0))
        .text_color(color::muted(cx))
        .child(
            div()
                .flex_1()
                .min_w(TAG_MIN_WIDTH)
                .child(upper(&crate::msg::desktop_tags_column_tag())),
        )
        .child(right(
            TRANSACTIONS_WIDTH,
            crate::msg::desktop_tags_column_transactions(),
        ))
        .child(right(TOTAL_WIDTH, crate::msg::desktop_tags_column_total()))
        .child(
            div()
                .w(LAST_USED_WIDTH)
                .flex_none()
                // The handoff runs LAST USED flush against TOTAL; a gap keeps them apart.
                .pl(px(16.0))
                .child(upper(&crate::msg::desktop_tags_column_last_used())),
        )
        .child(right(ACTIONS_WIDTH, lib_locale::msg::column_actions()))
}

/// The row's colours: inverted when selected (the handoff's `#201e1d` fill), plain otherwise.
struct RowColours {
    text: Rgba,
    secondary: Rgba,
    date: Rgba,
    flag: Rgba,
    negative: Rgba,
    button_border: Rgba,
}

impl RowColours {
    fn new(selected: bool, cx: &App) -> Self {
        if selected {
            Self {
                text: color::selection_text(cx),
                secondary: color::selection_muted(cx),
                date: color::selection_muted(cx),
                flag: color::selection_accent_text(cx),
                negative: color::selection_negative_text(cx),
                button_border: color::selection_muted(cx),
            }
        } else {
            Self {
                text: color::foreground(cx),
                secondary: color::muted(cx),
                date: color::muted(cx),
                flag: color::accent_text(cx),
                negative: color::negative_text(cx),
                button_border: color::border(cx),
            }
        }
    }
}

/// TOTAL's text: the sum with the Unit code appended when it isn't the base Unit, `mixed units`
/// when the Splits span Units (#355), `—` for an unused Tag. The flag selects the negative token.
fn total_text(total: &Total, base_unit: Option<&str>) -> (bool, String) {
    match total {
        Total::Empty => (false, EMPTY_CELL.to_string()),
        Total::Mixed => (false, crate::msg::desktop_transactions_total_mixed()),
        Total::Single { unit, amount } => {
            let (negative, text) = crate::format::amount(amount);
            if Some(unit.as_str()) == base_unit {
                (negative, text)
            } else {
                (negative, format!("{text} {unit}"))
            }
        }
    }
}

fn last_used_text(last_used: Option<NaiveDate>, date_style: Option<DateStyle>) -> String {
    last_used.map_or_else(
        || EMPTY_CELL.to_string(),
        |date| crate::format::date(date, date_style),
    )
}

fn row(
    tag: &Tag,
    duplicate_of: Option<&Tag>,
    last: bool,
    selected: bool,
    props: &TagsPageProps<'_>,
    cx: &App,
) -> impl IntoElement {
    let id = tag.id;
    let colours = RowColours::new(selected, cx);
    let hover = color::hover(cx);
    let usage = tags::usage(props.transactions, props.accounts, id);
    let mixed = usage.total == Total::Mixed;
    let (negative, total) = total_text(&usage.total, props.base_unit);

    let on_row_click = props.on_row_click.clone();
    let on_duplicate_click = props.on_duplicate_click.clone();
    let on_edit_click = props.on_edit_click.clone();
    let on_remove_click = props.on_remove_click.clone();

    div()
        .id(SharedString::from(format!("tags-row-{id}")))
        .cursor_pointer()
        .flex()
        .items_center()
        .px(px(16.0))
        .py(px(10.0))
        .text_color(colours.text)
        .when(!last, |this| {
            this.border_b(px(1.0)).border_color(color::hairline(cx))
        })
        .when(selected, |this| this.bg(color::selection_background(cx)))
        // The handoff's `#faf3ef` duplicate tint: the theme's accent tint is its nearest role.
        .when(!selected && duplicate_of.is_some(), |this| {
            this.bg(color::accent_tint(cx))
        })
        .when(!selected, |this| this.hover(move |style| style.bg(hover)))
        // #353: an inactive Tag stays listed, dimmed and tagged, rather than hidden.
        .when(!tag.is_active, |this| this.opacity(0.55))
        .on_click(move |_event, window, cx| on_row_click(id, window, cx))
        .child(
            div()
                .flex_1()
                .min_w(TAG_MIN_WIDTH)
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(swatch(tag.color.as_ref(), SWATCH_SIZE))
                .child(
                    div()
                        .truncate()
                        .when(selected, |this| {
                            this.font_weight(gpui::FontWeight::EXTRA_BOLD)
                        })
                        .child(tag.name.clone()),
                )
                .when(!tag.is_active, |this| {
                    this.child(
                        div()
                            .flex_none()
                            .px(px(6.0))
                            .py(px(1.0))
                            .border_1()
                            .border_color(colours.button_border)
                            .text_size(px(10.0))
                            .text_color(colours.secondary)
                            .child(crate::msg::desktop_tags_inactive()),
                    )
                })
                .when_some(duplicate_of, |this, target| {
                    this.child(
                        div()
                            .id(SharedString::from(format!("tags-duplicate-{id}")))
                            .cursor_pointer()
                            .flex_none()
                            .ml(px(4.0))
                            .px(px(6.0))
                            .py(px(1.0))
                            .border_1()
                            .border_color(color::accent(cx))
                            .text_size(px(10.0))
                            .text_color(colours.flag)
                            .on_click(move |_event, window, cx| {
                                cx.stop_propagation();
                                on_duplicate_click(id, window, cx);
                            })
                            .child(crate::msg::desktop_tags_duplicate_of(&target.name)),
                    )
                }),
        )
        .child(
            div()
                .w(TRANSACTIONS_WIDTH)
                .flex_none()
                .flex()
                .justify_end()
                .child(usage.transactions.to_string()),
        )
        .child(
            div()
                .w(TOTAL_WIDTH)
                .flex_none()
                .flex()
                .justify_end()
                .whitespace_nowrap()
                .when(selected && !mixed, |this| {
                    this.font_weight(gpui::FontWeight::EXTRA_BOLD)
                })
                .when(mixed, |this| this.text_color(colours.secondary))
                .when(negative, |this| this.text_color(colours.negative))
                .child(total),
        )
        .child(
            div()
                .w(LAST_USED_WIDTH)
                .flex_none()
                .pl(px(16.0))
                .whitespace_nowrap()
                .text_color(colours.date)
                .child(last_used_text(usage.last_used, props.date_style)),
        )
        .child(
            div()
                .w(ACTIONS_WIDTH)
                .flex_none()
                .flex()
                .justify_end()
                .gap(px(6.0))
                .child(row_action_button(
                    SharedString::from(format!("tags-edit-{id}")),
                    crate::msg::desktop_tags_row_edit(),
                    colours.button_border,
                    Rc::new(move |window: &mut Window, cx: &mut App| on_edit_click(id, window, cx)),
                    cx,
                ))
                .child(row_action_button(
                    SharedString::from(format!("tags-remove-{id}")),
                    crate::msg::desktop_tags_row_remove(),
                    colours.button_border,
                    Rc::new(move |window: &mut Window, cx: &mut App| {
                        on_remove_click(id, window, cx)
                    }),
                    cx,
                )),
        )
}

/// `padding:4px 8px; font-size:11px; border:1px`. Stops the click reaching the row, which would
/// otherwise also open Transactions.
fn row_action_button(
    id: SharedString,
    label: String,
    border: Rgba,
    on_click: OnPlainClick,
    cx: &App,
) -> impl IntoElement {
    let hover = color::hover(cx);
    div()
        .id(id)
        .cursor_pointer()
        .py(px(4.0))
        .px(px(8.0))
        .border_1()
        .border_color(border)
        .text_size(px(11.0))
        .hover(move |style| style.bg(hover))
        .on_click(move |_event, window, cx| {
            cx.stop_propagation();
            on_click(window, cx)
        })
        .child(label)
}

#[cfg(test)]
mod tests {
    use bigdecimal::BigDecimal;
    use lib_core::Money;

    use super::*;

    fn money(cents: i64) -> Money {
        Money(BigDecimal::new(cents.into(), 2))
    }

    #[test]
    fn total_names_the_unit_only_when_it_is_not_the_base_unit() {
        crate::locale::init_for_tests();
        let total = |unit: &str| Total::Single {
            unit: unit.to_string(),
            amount: money(-81200),
        };
        assert_eq!(
            total_text(&total("aud"), Some("aud")),
            (true, "\u{2212}812.00".to_string())
        );
        assert_eq!(
            total_text(&total("usd"), Some("aud")),
            (true, "\u{2212}812.00 usd".to_string())
        );
    }

    #[test]
    fn an_unused_tag_shows_a_dash_and_a_mixed_one_no_sum() {
        crate::locale::init_for_tests();
        assert_eq!(
            total_text(&Total::Empty, Some("aud")),
            (false, EMPTY_CELL.to_string())
        );
        assert_eq!(
            total_text(&Total::Mixed, Some("aud")),
            (false, "mixed units".to_string())
        );
        assert_eq!(last_used_text(None, None), EMPTY_CELL);
    }

    #[test]
    fn only_the_non_target_members_count_as_likely_duplicates() {
        let groups = [
            DuplicateGroup {
                target: 1,
                duplicates: vec![2, 3],
            },
            DuplicateGroup {
                target: 4,
                duplicates: vec![5],
            },
        ];
        assert_eq!(likely_duplicate_count(&groups), 3);
        assert_eq!(likely_duplicate_count(&[]), 0);
    }
}
