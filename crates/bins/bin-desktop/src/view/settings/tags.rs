//! The **Tags** page (`docs/ux/desktop/Settings/README.md`'s 2j): a kicker (A–Z) with **+ Add tag**
//! over one bordered list of colour swatch and name, with `edit · remove` per row. A likely
//! duplicate carries its warning badge and a direct **merge**; the merge flow is 7e's. Usage lives
//! in Transactions and Reports, so there are no transaction, total or last-used columns. It reuses
//! the Tags model, swatch and dialogs.
//!
//! Every action is a callback into `Shell`, so the keyboard (`n`/`e`/`x`/`m`) and the mouse reach
//! the same handlers.

use std::rc::Rc;

use gpui::{AnyElement, App, Rgba, SharedString, Window, div, prelude::*, px};
use lib_locale::format::upper;

use crate::{
    tags::{self, DuplicateGroup, Tag},
    theme::color,
    view::tags::{OnPlainClick, OnTagClick, SWATCH_SIZE, likely_duplicate_count, swatch},
};

pub struct TagsPageProps<'a> {
    pub tags: &'a [Tag],
    /// The selected row's Tag id.
    pub selected: Option<u32>,
    /// The likely-duplicate groups, which the Shell works out from the ledger's Transactions.
    pub groups: &'a [DuplicateGroup],
    pub on_add_click: OnPlainClick,
    pub on_row_click: OnTagClick,
    pub on_merge_click: OnTagClick,
    pub on_edit_click: OnTagClick,
    pub on_remove_click: OnTagClick,
}

/// A row count as the number a plural selector takes.
fn count(len: usize) -> i64 {
    i64::try_from(len).unwrap_or(i64::MAX)
}

/// The heading's meta as plain text, for the status line: `18 tags · 2 likely duplicates`.
pub fn scope_text(tags: &[Tag], groups: &[DuplicateGroup]) -> String {
    let total = crate::msg::desktop_tags_count(count(tags::active_count(tags)));
    match likely_duplicate_count(groups) {
        0 => total,
        duplicates => format!(
            "{total} \u{b7} {}",
            crate::msg::desktop_tags_status_duplicates(count(duplicates))
        ),
    }
}

/// The heading's meta: the count, then the duplicates clause with its number bold in the accent
/// text colour (`#ae1800`) and no "merge them" link -- each flagged row has its own **merge**.
pub fn scope_note(tags: &[Tag], groups: &[DuplicateGroup], cx: &App) -> AnyElement {
    let duplicates = count(likely_duplicate_count(groups));
    div()
        .flex()
        .items_baseline()
        .gap(px(4.0))
        .child(crate::msg::desktop_tags_count(count(tags::active_count(
            tags,
        ))))
        .when(duplicates > 0, |this| {
            this.child("\u{b7}").children(
                crate::msg::desktop_tags_likely_duplicates(duplicates)
                    .into_iter()
                    .map(|segment| match segment.tag.as_deref() {
                        Some("strong") => div()
                            .font_weight(gpui::FontWeight::EXTRA_BOLD)
                            .text_color(color::accent_text(cx))
                            .child(segment.text)
                            .into_any_element(),
                        // The message ends in the dash that led into the Tags page's link.
                        _ => div()
                            .child(segment.text.trim_end_matches(" \u{2014}").to_string())
                            .into_any_element(),
                    }),
            )
        })
        .into_any_element()
}

pub fn render(props: &TagsPageProps<'_>, focused: bool, cx: &App) -> AnyElement {
    let sorted = tags::sorted_by_name(props.tags);
    let last = sorted.len().saturating_sub(1);
    div()
        .flex()
        .flex_col()
        .child(kicker_row(props.on_add_click.clone(), cx))
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
                    .mb(px(18.0))
                    .child(table_header(cx))
                    .children(sorted.iter().enumerate().map(|(position, tag)| {
                        let duplicate_of = tags::duplicate_of(props.groups, tag.id)
                            .and_then(|target| tags::get(props.tags, target));
                        row(
                            tag,
                            duplicate_of,
                            position == last,
                            props.selected == Some(tag.id),
                            focused,
                            props,
                            cx,
                        )
                    })),
            )
        })
        .child(
            div()
                .max_w(px(620.0))
                .text_size(px(11.5))
                .text_color(color::muted(cx))
                .child(crate::msg::desktop_tags_footnote()),
        )
        .into_any_element()
}

fn kicker_row(on_add_click: OnPlainClick, cx: &App) -> impl IntoElement {
    let hover = color::muted(cx);
    div()
        .flex()
        .items_center()
        .justify_between()
        .mb(px(10.0))
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(10.0))
                .text_color(color::faint_text(cx))
                .child(upper(&crate::msg::desktop_settings_tags_kicker())),
        )
        // `btn btn-primary` at `height:32px`.
        .child(
            div()
                .debug_selector(|| "settings-tags-add".to_string())
                .id("settings-tags-add")
                .cursor_pointer()
                .flex_none()
                .py(px(8.0))
                .px(px(14.0))
                .bg(color::foreground(cx))
                .text_color(color::selection_text(cx))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .whitespace_nowrap()
                .hover(move |style| style.bg(hover))
                .on_click(move |_event, window, cx| on_add_click(window, cx))
                .child(crate::msg::desktop_tags_add_button("+")),
        )
}

/// `padding:6px 14px; background:#eae9e9`: TAG and ACTIONS only.
fn table_header(cx: &App) -> impl IntoElement {
    div()
        .flex()
        .px(px(14.0))
        .py(px(6.0))
        .bg(color::chrome(cx))
        .border_b(px(1.0))
        .border_color(color::border(cx))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(10.0))
        .text_color(color::muted(cx))
        .child(
            div()
                .flex_1()
                .child(upper(&crate::msg::desktop_tags_column_tag())),
        )
        .child(div().child(upper(&lib_locale::msg::column_actions())))
}

/// The selected row is inverted while the page has focus (`background:#201e1d; color:#f3f2f2`)
/// and only tinted while the index does. A likely duplicate takes the accent tint.
fn row(
    tag: &Tag,
    duplicate_of: Option<&Tag>,
    last: bool,
    selected: bool,
    focused: bool,
    props: &TagsPageProps<'_>,
    cx: &App,
) -> impl IntoElement {
    let id = tag.id;
    let inverted = selected && focused;
    let hover = color::hover(cx);
    let (secondary, flag, button_border): (Rgba, Rgba, Rgba) = if inverted {
        (
            color::selection_muted(cx),
            color::selection_accent_text(cx),
            color::selection_muted(cx),
        )
    } else {
        (color::muted(cx), color::accent_text(cx), color::border(cx))
    };
    let on_row_click = props.on_row_click.clone();
    let on_merge_click = props.on_merge_click.clone();
    let on_edit_click = props.on_edit_click.clone();
    let on_remove_click = props.on_remove_click.clone();

    div()
        .debug_selector(move || format!("settings-tags-row-{id}"))
        .id(SharedString::from(format!("settings-tags-row-{id}")))
        .cursor_pointer()
        .flex()
        .items_center()
        .px(px(14.0))
        .py(px(5.0))
        .text_size(px(12.5))
        .when(!last, |this| {
            this.border_b(px(1.0)).border_color(color::hairline(cx))
        })
        .when(!selected && duplicate_of.is_some(), |this| {
            this.bg(color::accent_tint(cx))
        })
        .when(selected && !focused, |this| this.bg(color::chrome(cx)))
        .when(inverted, |this| {
            this.bg(color::foreground(cx))
                .text_color(color::selection_text(cx))
        })
        .when(!selected, |this| this.hover(move |style| style.bg(hover)))
        // An inactive Tag stays listed, dimmed and tagged, rather than hidden.
        .when(!tag.is_active, |this| this.opacity(0.55))
        .on_click(move |_event, window, cx| on_row_click(id, window, cx))
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(swatch(tag.color.as_ref(), SWATCH_SIZE))
                .child(
                    div()
                        .truncate()
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .child(tag.name.clone()),
                )
                .when(!tag.is_active, |this| {
                    this.child(
                        div()
                            .flex_none()
                            .px(px(6.0))
                            .py(px(1.0))
                            .border_1()
                            .border_color(button_border)
                            .text_size(px(10.0))
                            .text_color(secondary)
                            .child(crate::msg::desktop_tags_inactive()),
                    )
                })
                .when_some(duplicate_of, |this, target| {
                    this.child(
                        div()
                            .flex_none()
                            .ml(px(4.0))
                            .px(px(6.0))
                            .py(px(1.0))
                            .border_1()
                            .border_color(color::accent(cx))
                            .text_size(px(10.0))
                            .text_color(flag)
                            .child(crate::msg::desktop_tags_duplicate_of(&target.name)),
                    )
                }),
        )
        .child(
            div()
                .flex()
                .gap(px(4.0))
                .when(duplicate_of.is_some(), |this| {
                    this.child(row_action_button(
                        SharedString::from(format!("settings-tags-merge-{id}")),
                        crate::msg::desktop_settings_tags_row_merge(),
                        button_border,
                        Rc::new(move |window: &mut Window, cx: &mut App| {
                            on_merge_click(id, window, cx)
                        }),
                        cx,
                    ))
                })
                .child(row_action_button(
                    SharedString::from(format!("settings-tags-edit-{id}")),
                    crate::msg::desktop_tags_row_edit(),
                    button_border,
                    Rc::new(move |window: &mut Window, cx: &mut App| on_edit_click(id, window, cx)),
                    cx,
                ))
                .child(row_action_button(
                    SharedString::from(format!("settings-tags-remove-{id}")),
                    crate::msg::desktop_tags_row_remove(),
                    button_border,
                    Rc::new(move |window: &mut Window, cx: &mut App| {
                        on_remove_click(id, window, cx)
                    }),
                    cx,
                )),
        )
}

/// `padding:2px 6px; font-size:11px; border:1px solid`. Stops the click reaching the row.
fn row_action_button(
    id: SharedString,
    label: String,
    border: Rgba,
    on_click: OnPlainClick,
    cx: &App,
) -> impl IntoElement {
    let hover = color::hover(cx);
    div()
        .debug_selector({
            let id = id.clone();
            move || id.to_string()
        })
        .id(id)
        .cursor_pointer()
        .py(px(2.0))
        .px(px(6.0))
        .border_1()
        .border_color(border)
        .text_size(px(11.0))
        .whitespace_nowrap()
        .hover(move |style| style.bg(hover))
        .on_click(move |_event, window, cx| {
            cx.stop_propagation();
            on_click(window, cx)
        })
        .child(label)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_text_reads_the_count_alone_when_nothing_is_flagged() {
        crate::locale::init_for_tests();
        let tags = tags::default_tags();
        assert_eq!(
            scope_text(&tags, &[]),
            format!("{} tags", tags::active_count(&tags))
        );
    }

    #[test]
    fn scope_text_adds_the_duplicates_clause() {
        crate::locale::init_for_tests();
        let tags = tags::default_tags();
        let groups = [DuplicateGroup {
            target: 1,
            duplicates: vec![2],
        }];
        assert_eq!(
            scope_text(&tags, &groups),
            format!(
                "{} tags \u{b7} 1 likely duplicate",
                tags::active_count(&tags)
            )
        );
    }
}
