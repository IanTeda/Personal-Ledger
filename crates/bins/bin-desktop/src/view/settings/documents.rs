//! The **Documents** page (`docs/ux/desktop-mockups/16-settings/README.md`'s 16p, as amended by #479): a
//! kicker with **+ Add document type** over one bordered table of the Ledger's Document Types in
//! type-filter order, then four one-sentence notes. Other carries a `default` outline tag and no
//! **remove** button.
//!
//! Every action is a callback into `Shell`, so the keyboard (`n`/`e`/`x`/`J`/`K`) and the mouse
//! reach the same handlers.

use std::rc::Rc;

use gpui::{AnyElement, App, Rgba, SharedString, Window, div, prelude::*, px};
use lib_locale::format::upper;

use crate::{
    documents,
    documents::types::{DocumentTypeRow, RemindLead, TracksDate},
    theme::color,
};

pub type OnPlainClick = Rc<dyn Fn(&mut Window, &mut App)>;
pub type OnTypeClick = Rc<dyn Fn(u32, &mut Window, &mut App)>;

pub struct DocumentsPageProps<'a> {
    pub types: &'a [DocumentTypeRow],
    /// The selected row's type id.
    pub selected: Option<u32>,
    pub on_add_click: OnPlainClick,
    pub on_row_click: OnTypeClick,
    pub on_edit_click: OnTypeClick,
    pub on_remove_click: OnTypeClick,
}

/// A row count as the number a plural selector takes.
fn count(len: usize) -> i64 {
    i64::try_from(len).unwrap_or(i64::MAX)
}

/// The heading's meta as plain text, for the status line: `9 types · 392 of 412 files typed`.
pub fn scope_text(types: &[DocumentTypeRow]) -> String {
    crate::msg::desktop_document_types_scope(
        &crate::msg::desktop_document_types_count(count(types.len())),
        &crate::msg::desktop_document_types_files_typed(
            &documents::types::typed_files(types).to_string(),
            &documents::types::all_files(types).to_string(),
        ),
    )
}

/// The text of a Tracks date cell.
pub(super) fn tracks_label(tracks: TracksDate) -> String {
    match tracks {
        TracksDate::Renews => crate::msg::desktop_document_types_tracks_renews(),
        TracksDate::Ends => crate::msg::desktop_document_types_tracks_ends(),
        TracksDate::Expires => crate::msg::desktop_document_types_tracks_expires(),
        TracksDate::Revalue => crate::msg::desktop_document_types_tracks_revalue(),
    }
}

/// The text of a Remind cell.
fn remind_label(lead: RemindLead) -> String {
    match lead {
        RemindLead::Days(days) => crate::msg::desktop_document_types_remind_days(i64::from(days)),
        RemindLead::Months(months) => {
            crate::msg::desktop_document_types_remind_months(i64::from(months))
        }
    }
}

pub fn render(props: &DocumentsPageProps<'_>, focused: bool, cx: &App) -> AnyElement {
    let last = props.types.len().saturating_sub(1);
    div()
        .flex()
        .flex_col()
        .child(kicker_row(props.on_add_click.clone(), cx))
        .child(
            div()
                .flex()
                .flex_col()
                .border_1()
                .border_color(color::border(cx))
                .mb(px(16.0))
                .child(table_header(cx))
                .children(props.types.iter().enumerate().map(|(position, row_type)| {
                    row(
                        row_type,
                        position == last,
                        props.selected == Some(row_type.id),
                        focused,
                        props,
                        cx,
                    )
                })),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(6.0))
                .max_w(px(640.0))
                .text_size(px(12.0))
                .text_color(color::muted(cx))
                .children([
                    note(crate::msg::desktop_document_types_note_tracks_date(), cx),
                    note(crate::msg::desktop_document_types_note_financial_year(), cx),
                    note(crate::msg::desktop_document_types_note_removal(), cx),
                    note(crate::msg::desktop_document_types_note_default(), cx),
                ]),
        )
        .into_any_element()
}

/// One note: its `<strong>` lead in the foreground colour, the rest muted.
fn note(segments: Vec<lib_locale::Segment>, cx: &App) -> impl IntoElement {
    div()
        .flex()
        .flex_wrap()
        .children(segments.into_iter().map(|segment| {
            let text = div().child(segment.text);
            match segment.tag.as_deref() {
                Some("strong") => text
                    .font_weight(gpui::FontWeight::EXTRA_BOLD)
                    .text_color(color::foreground(cx)),
                _ => text,
            }
        }))
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
                .child(upper(&crate::msg::desktop_document_types_kicker())),
        )
        // `btn btn-primary` at `height:32px`.
        .child(
            div()
                .debug_selector(|| "settings-documents-add".to_string())
                .id("settings-documents-add")
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
                .child(crate::msg::desktop_document_types_add_button("+")),
        )
}

/// `padding:6px 14px; background:#eae9e9`, with the frame's column widths.
fn table_header(cx: &App) -> impl IntoElement {
    let cell = |label: String, width: f32, right: bool| {
        let cell = div().w(px(width)).flex_none().child(upper(&label));
        if right {
            cell.flex().justify_end()
        } else {
            cell
        }
    };
    div()
        .flex()
        .gap(px(14.0))
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
                .child(upper(&crate::msg::desktop_document_types_column_type())),
        )
        .child(cell(
            crate::msg::desktop_document_types_column_tracks_date(),
            150.0,
            false,
        ))
        .child(cell(
            crate::msg::desktop_document_types_column_remind(),
            130.0,
            false,
        ))
        .child(cell(
            crate::msg::desktop_document_types_column_financial_year(),
            120.0,
            false,
        ))
        .child(cell(
            crate::msg::desktop_document_types_column_files(),
            60.0,
            true,
        ))
        .child(cell(lib_locale::msg::column_actions(), 118.0, true))
}

/// The selected row is inverted while the page has focus (`background:#201e1d; color:#f3f2f2`)
/// and only tinted while the index does.
fn row(
    row_type: &DocumentTypeRow,
    last: bool,
    selected: bool,
    focused: bool,
    props: &DocumentsPageProps<'_>,
    cx: &App,
) -> impl IntoElement {
    let id = row_type.id;
    let inverted = selected && focused;
    let hover = color::hover(cx);
    let (placeholder, flag, button_border): (Rgba, Rgba, Rgba) = if inverted {
        (
            color::selection_muted(cx),
            color::selection_accent_text(cx),
            color::selection_muted(cx),
        )
    } else {
        (
            color::faint_text(cx),
            color::accent_text(cx),
            color::border(cx),
        )
    };
    let on_row_click = props.on_row_click.clone();
    let on_edit_click = props.on_edit_click.clone();
    let on_remove_click = props.on_remove_click.clone();
    // A dash, in the muted colour, where the type tracks or reminds nothing.
    let optional = |text: Option<String>, width: f32| {
        let cell = div().w(px(width)).flex_none();
        match text {
            Some(text) => cell.child(text),
            None => cell
                .text_color(placeholder)
                .child(crate::msg::desktop_document_types_none()),
        }
    };
    let yes_no = if row_type.financial_year {
        crate::msg::desktop_document_types_yes()
    } else {
        crate::msg::desktop_document_types_no()
    };

    div()
        .debug_selector(move || format!("settings-documents-row-{id}"))
        .id(SharedString::from(format!("settings-documents-row-{id}")))
        .cursor_pointer()
        .flex()
        .items_center()
        .gap(px(14.0))
        .px(px(14.0))
        .py(px(6.0))
        .text_size(px(12.5))
        .when(!last, |this| {
            this.border_b(px(1.0)).border_color(color::hairline(cx))
        })
        .when(selected && !focused, |this| this.bg(color::chrome(cx)))
        .when(inverted, |this| {
            this.bg(color::foreground(cx))
                .text_color(color::selection_text(cx))
        })
        .when(!selected, |this| this.hover(move |style| style.bg(hover)))
        .on_click(move |_event, window, cx| on_row_click(id, window, cx))
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(
                    div()
                        .truncate()
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .child(row_type.name.clone()),
                )
                .when(row_type.is_default, |this| {
                    this.child(
                        div()
                            .flex_none()
                            .px(px(6.0))
                            .py(px(1.0))
                            .border_1()
                            .border_color(color::accent(cx))
                            .text_size(px(10.0))
                            .text_color(flag)
                            .child(crate::msg::desktop_document_types_flag_default()),
                    )
                }),
        )
        .child(optional(row_type.tracks_date.map(tracks_label), 150.0))
        // Remind is inert without a Tracks date, so it reads as a dash too.
        .child(optional(
            row_type.tracks_date.and(row_type.remind).map(remind_label),
            130.0,
        ))
        .child(div().w(px(120.0)).flex_none().child(yes_no))
        .child(
            div()
                .w(px(60.0))
                .flex_none()
                .flex()
                .justify_end()
                .child(row_type.files.to_string()),
        )
        .child(
            div()
                .w(px(118.0))
                .flex_none()
                .flex()
                .justify_end()
                .gap(px(4.0))
                .child(row_action_button(
                    SharedString::from(format!("settings-documents-edit-{id}")),
                    crate::msg::desktop_document_types_row_edit(),
                    button_border,
                    Rc::new(move |window: &mut Window, cx: &mut App| on_edit_click(id, window, cx)),
                    cx,
                ))
                .when(!row_type.is_default, |this| {
                    this.child(row_action_button(
                        SharedString::from(format!("settings-documents-remove-{id}")),
                        crate::msg::desktop_document_types_row_remove(),
                        button_border,
                        Rc::new(move |window: &mut Window, cx: &mut App| {
                            on_remove_click(id, window, cx)
                        }),
                        cx,
                    ))
                }),
        )
}

/// `padding:4px 8px; font-size:11.5px; border:1px solid`. Stops the click reaching the row.
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
        .py(px(4.0))
        .px(px(8.0))
        .border_1()
        .border_color(border)
        .text_size(px(11.5))
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
    fn scope_text_reads_the_live_counts() {
        crate::locale::init_for_tests();
        assert_eq!(
            scope_text(&documents::types::default_types()),
            "9 types \u{b7} 392 of 412 files typed"
        );
    }

    #[test]
    fn a_single_type_reads_singular() {
        crate::locale::init_for_tests();
        let one = &documents::types::default_types()[..1];
        assert!(scope_text(one).starts_with("1 type \u{b7} "));
    }

    #[test]
    fn remind_reads_days_and_months() {
        crate::locale::init_for_tests();
        assert_eq!(remind_label(RemindLead::Days(30)), "30 days before");
        assert_eq!(remind_label(RemindLead::Months(3)), "3 months before");
        assert_eq!(remind_label(RemindLead::Days(1)), "1 day before");
    }
}
