//! Draws the Document Types page's view model (`docs/ux/desktop-mockups/16-settings/README.md`'s 16p, as amended by #479): a
//! kicker with **+ Add document type** over one bordered table of the Ledger's Document Types in
//! type-filter order, then four one-sentence notes. Other carries a `default` outline tag and no
//! **remove** button.
//!
//! Every click sends the Presenter the event its key does: a row click only selects.

use gpui::{AnyElement, App, ClickEvent, Rgba, SharedString, Window, div, prelude::*, px};
use lib_documents::presenter::{DocumentTypesPageEvent, DocumentTypesPageViewModel, TypeLine};
use lib_locale::format::upper;

use crate::document_types::DocumentTypesPage;
use crate::{shell::view::page_clicks::PageClicks, theme::color};

/// The page: the kicker and **+ Add document type**, the table and the notes. The selected row is
/// inverted only while the page has `focused`.
pub(super) fn page(
    view: &DocumentTypesPageViewModel,
    clicks: &PageClicks<DocumentTypesPage>,
    focused: bool,
    cx: &App,
) -> AnyElement {
    let last = view.rows.len().saturating_sub(1);
    div()
        .flex()
        .flex_col()
        .child(kicker_row(view, clicks, cx))
        .child(
            div()
                .flex()
                .flex_col()
                .border_1()
                .border_color(color::border(cx))
                .mb(px(16.0))
                .child(table_header(view, cx))
                .children(view.rows.iter().enumerate().map(|(position, line)| {
                    row(
                        clicks,
                        line,
                        view,
                        position == last,
                        view.selected == Some(line.id),
                        focused,
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
                .children(view.notes.iter().cloned().map(|note| note_line(note, cx))),
        )
        .into_any_element()
}

/// One note: its `<strong>` lead in the foreground colour, the rest muted.
fn note_line(segments: Vec<lib_locale::Segment>, cx: &App) -> impl IntoElement {
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

fn kicker_row(
    view: &DocumentTypesPageViewModel,
    clicks: &PageClicks<DocumentTypesPage>,
    cx: &App,
) -> impl IntoElement {
    let hover = color::muted(cx);
    let on_add = clicks.row(DocumentTypesPageEvent::New);
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
                .child(upper(&view.kicker)),
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
                .on_click(on_add)
                .child(view.add_button.clone()),
        )
}

/// `padding:6px 14px; background:#eae9e9`, with the frame's column widths.
fn table_header(view: &DocumentTypesPageViewModel, cx: &App) -> impl IntoElement {
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
        .child(div().flex_1().child(upper(&view.columns.type_name)))
        .child(cell(view.columns.tracks_date.clone(), 150.0, false))
        .child(cell(view.columns.remind.clone(), 130.0, false))
        .child(cell(view.columns.financial_year.clone(), 120.0, false))
        .child(cell(view.columns.files.clone(), 60.0, true))
        .child(cell(view.columns.actions.clone(), 118.0, true))
}

/// The selected row is inverted while the page has focus (`background:#201e1d; color:#f3f2f2`)
/// and only tinted while the index does.
fn row(
    clicks: &PageClicks<DocumentTypesPage>,
    row_type: &TypeLine,
    view: &DocumentTypesPageViewModel,
    last: bool,
    selected: bool,
    focused: bool,
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
    let on_row_click = clicks.row(DocumentTypesPageEvent::Select(id));
    let on_edit_click = clicks.row(DocumentTypesPageEvent::EditRow(id));
    let on_remove_click = clicks.row(DocumentTypesPageEvent::RemoveRow(id));
    // A dash, in the muted colour, where the type tracks or reminds nothing.
    let optional = |text: Option<String>, width: f32| {
        let cell = div().w(px(width)).flex_none();
        match text {
            Some(text) => cell.child(text),
            None => cell.text_color(placeholder).child(row_type.none.clone()),
        }
    };
    let yes_no = row_type.financial_year.clone();

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
        .on_click(on_row_click)
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
                            .child(view.default_flag.clone()),
                    )
                }),
        )
        .child(optional(row_type.tracks_date.clone(), 150.0))
        // Remind is inert without a Tracks date, so it reads as a dash too.
        .child(optional(row_type.remind.clone(), 130.0))
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
                    view.row_edit.clone(),
                    button_border,
                    on_edit_click,
                    cx,
                ))
                .when(!row_type.is_default, |this| {
                    this.child(row_action_button(
                        SharedString::from(format!("settings-documents-remove-{id}")),
                        view.row_remove.clone(),
                        button_border,
                        on_remove_click,
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
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
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
        .on_click(move |event: &ClickEvent, window, cx| {
            cx.stop_propagation();
            on_click(event, window, cx);
        })
        .child(label)
}
