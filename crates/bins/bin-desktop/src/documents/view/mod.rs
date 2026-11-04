//! The **Documents** destination (`docs/ux/desktop-mockups/04-documents/`): the 190px index rail, the list pane
//! (header, toolbar, count line and rows) and the 300px detail pane. The Library (4a) is drawn
//! here and the Inbox (4b) in `inbox`, each filling the list and detail panes.
//!
//! [`page::DocumentsPage`] holds the `lib-documents` page Presenter and draws its view model through
//! these functions: every click is an event sent to the Presenter, as its key is. The detail pane is
//! never focused: its actions are list keys (`enter`, `o`, `l`). The page's Dialogs are one hosted
//! Entity, `dialog`; the window's drop overlay is `drop_overlay`.

pub mod dialog;
pub mod drop_overlay;
mod inbox;
pub mod page;
pub mod snapshot;

use std::{ops::Range, rc::Rc};

use gpui::{
    AnyElement, App, Pixels, ScrollHandle, SharedString, UniformListScrollHandle, Window, div,
    prelude::*, px, uniform_list,
};
use lib_documents::presenter::library::{
    Chip, ChipKind, DetailView, Fact, LinkRow, RailRow, RowTail, RowView,
};
use lib_documents::presenter::{
    DocumentsFocus, DocumentsPageEvent, DocumentsPageViewModel, LibraryViewModel,
};
use lib_locale::format::upper;

use crate::{
    dialog::OnClick,
    documents::{DocumentsMode, LibrarySort, RailEntry},
    shell::view::page_clicks::PageClicks,
    theme::color,
};
use page::DocumentsPage;

const RAIL_WIDTH: Pixels = px(190.0);
const DETAIL_WIDTH: Pixels = px(300.0);

/// What of the page's draw state the Presenter's view model doesn't hold: the list scroll handles,
/// which stay on the page Entity, and whether the list zone has keyboard focus, which draws the
/// focused Inbox row's outline.
pub(super) struct Scrolls {
    pub library: UniformListScrollHandle,
    pub inbox: ScrollHandle,
    pub list_focused: bool,
}

/// The whole destination: rail, list pane and detail pane, side by side. `focused` is whether the
/// View zone has focus; the focus ring is drawn on the rail or the list within it.
pub(super) fn render(
    focused: bool,
    view: &DocumentsPageViewModel,
    clicks: &PageClicks<DocumentsPage>,
    mut scroll: Scrolls,
    cx: &App,
) -> AnyElement {
    let list_focused = focused && view.focus == DocumentsFocus::List;
    let index_focused = focused && view.focus == DocumentsFocus::Index;
    scroll.list_focused = list_focused;
    div()
        .id("documents")
        .flex_1()
        .min_w(px(0.0))
        .h_full()
        .flex()
        .child(index_rail(view, clicks, index_focused, cx))
        .child(list_pane(view, clicks, &scroll, list_focused, cx))
        .child(if view.mode == DocumentsMode::Library {
            detail_pane(view, clicks, cx).into_any_element()
        } else {
            inbox::detail_pane(view, clicks, cx)
        })
        .into_any_element()
}

// ---------------------------------------------------------------------------------------------
// Index rail
// ---------------------------------------------------------------------------------------------

fn index_rail(
    view: &DocumentsPageViewModel,
    clicks: &PageClicks<DocumentsPage>,
    focused: bool,
    cx: &App,
) -> impl IntoElement {
    let mut column = div().flex_1().min_h(px(0.0)).flex().flex_col().py(px(10.0));
    for row in &view.rail {
        if let Some(label) = row.section.clone() {
            column = column.child(rail_label(label, cx));
        }
        column = column.child(rail_row(row, row.entry == view.rail_selected, clicks, cx));
    }
    div()
        .w(RAIL_WIDTH)
        .flex_none()
        .h_full()
        .flex()
        .flex_col()
        .bg(color::background(cx))
        .border_r(px(2.0))
        .border_color(color::structural_rule(cx))
        // The View frame draws the focused edge at the rail's left; the rail only darkens its rule.
        .when(focused, |this| this.border_color(color::foreground(cx)))
        .text_size(px(12.5))
        .child(column)
        .child(
            div()
                .mx(px(14.0))
                .pt(px(10.0))
                .pb(px(10.0))
                .border_t(px(1.0))
                .border_color(color::border(cx))
                .text_size(px(11.0))
                .line_height(gpui::relative(1.4))
                .text_color(color::faint_text(cx))
                .child(view.stored.clone()),
        )
}

fn rail_label(label: String, cx: &App) -> impl IntoElement {
    div()
        .pt(px(14.0))
        .px(px(14.0))
        .pb(px(6.0))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(10.0))
        .text_color(color::faint_text(cx))
        .child(upper(&label))
}

fn rail_row(
    row: &RailRow,
    active: bool,
    clicks: &PageClicks<DocumentsPage>,
    cx: &App,
) -> impl IntoElement {
    let entry = row.entry;
    let badge = entry == RailEntry::Inbox && row.count > 0;
    div()
        .id(SharedString::from(format!("documents-rail-{entry:?}")))
        .debug_selector(|| format!("documents-rail-{entry:?}"))
        .cursor_pointer()
        .flex()
        .items_center()
        .gap(px(6.0))
        .py(px(6.0))
        .px(px(14.0))
        .when(active, |this| {
            this.bg(color::selection_background(cx))
                .text_color(color::selection_text(cx))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
        })
        .on_click(clicks.send(DocumentsPageEvent::ClickRail(entry)))
        .child(div().flex_1().truncate().child(row.label.clone()))
        .child(if badge {
            // The Inbox badge stays red even on the active row.
            div()
                .px(px(5.0))
                .py(px(1.0))
                .bg(color::accent(cx))
                .text_color(color::background(cx))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(10.0))
                .child(row.count.to_string())
        } else {
            div()
                .text_size(px(11.0))
                .text_color(if active {
                    color::selection_muted(cx)
                } else {
                    color::faint_text(cx)
                })
                .child(row.count.to_string())
        })
}

// ---------------------------------------------------------------------------------------------
// List pane
// ---------------------------------------------------------------------------------------------

fn list_pane(
    view: &DocumentsPageViewModel,
    clicks: &PageClicks<DocumentsPage>,
    scroll: &Scrolls,
    focused: bool,
    cx: &App,
) -> impl IntoElement {
    div()
        .flex_1()
        .min_w(px(0.0))
        .h_full()
        .flex()
        .flex_col()
        .overflow_hidden()
        .when(focused, |this| {
            this.border_l(px(2.0)).border_color(color::foreground(cx))
        })
        .child(list_header(view, clicks, cx))
        .when_some(view.library.as_ref(), |this, library| {
            this.child(toolbar(view, clicks, library, cx))
                .child(
                    div()
                        .px(px(14.0))
                        .pt(px(8.0))
                        .pb(px(4.0))
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .text_size(px(10.0))
                        .text_color(color::faint_text(cx))
                        .child(library.count_line.clone()),
                )
                .child(rows(
                    view,
                    clicks,
                    scroll,
                    library,
                    Rc::new(library.rows.clone()),
                    cx,
                ))
        })
        .when_some(view.inbox.as_ref(), |this, inbox| {
            this.child(inbox::list_body(view, clicks, scroll, inbox, cx))
        })
}

fn list_header(
    view: &DocumentsPageViewModel,
    clicks: &PageClicks<DocumentsPage>,
    cx: &App,
) -> impl IntoElement {
    div()
        .flex_none()
        .flex()
        .items_center()
        .gap(px(12.0))
        .px(px(20.0))
        .pt(px(16.0))
        .pb(px(12.0))
        .border_b(px(2.0))
        .border_color(color::structural_rule(cx))
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .flex_col()
                .child(
                    div()
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .text_size(px(28.0))
                        .line_height(gpui::relative(1.1))
                        .mb(px(4.0))
                        .child(view.title.clone()),
                )
                .child(
                    div()
                        .text_size(px(11.5))
                        .text_color(color::faint_text(cx))
                        .child(view.subline.clone()),
                ),
        )
        .children(match view.inbox.as_ref() {
            None => vec![
                secondary_button(
                    "documents-import",
                    view.labels.import.clone(),
                    clicks.click(DocumentsPageEvent::Import),
                    cx,
                )
                .into_any_element(),
                primary_button(
                    "documents-add",
                    view.labels.add.clone(),
                    clicks.click(DocumentsPageEvent::Add),
                    cx,
                )
                .into_any_element(),
            ],
            Some(inbox) => inbox::header_buttons(view, clicks, inbox, cx).into(),
        })
}

fn primary_button(
    id: &'static str,
    label: String,
    on_click: OnClick,
    cx: &App,
) -> impl IntoElement {
    let hover = color::muted(cx);
    div()
        .id(id)
        .debug_selector(|| id.to_string())
        .cursor_pointer()
        .flex_none()
        .flex()
        .items_center()
        .h(px(36.0))
        .px(px(14.0))
        .bg(color::foreground(cx))
        .text_color(color::selection_text(cx))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .whitespace_nowrap()
        .hover(move |style| style.bg(hover))
        .on_click(move |_event, window, cx| on_click(window, cx))
        .child(label)
}

fn secondary_button(
    id: &'static str,
    label: String,
    on_click: OnClick,
    cx: &App,
) -> impl IntoElement {
    let hover = color::hover(cx);
    div()
        .id(id)
        .debug_selector(|| id.to_string())
        .cursor_pointer()
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .h(px(36.0))
        .px(px(14.0))
        .border_1()
        .border_color(color::border(cx))
        .text_color(color::foreground(cx))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .whitespace_nowrap()
        .hover(move |style| style.bg(hover))
        .on_click(move |_event, window, cx| on_click(window, cx))
        .child(label)
}

fn toolbar(
    view: &DocumentsPageViewModel,
    clicks: &PageClicks<DocumentsPage>,
    library: &LibraryViewModel,
    cx: &App,
) -> impl IntoElement {
    div()
        .flex_none()
        .flex()
        .items_center()
        .gap(px(8.0))
        .px(px(14.0))
        .py(px(10.0))
        .border_b(px(1.0))
        .border_color(color::border(cx))
        .child(search_box(view, clicks, library, cx))
        .child(sort_control(view, clicks, library, cx))
}

/// The search input (30px, 12.5px): the placeholder, or the text so far with a caret and an accent
/// border while search mode is active.
fn search_box(
    view: &DocumentsPageViewModel,
    clicks: &PageClicks<DocumentsPage>,
    library: &LibraryViewModel,
    cx: &App,
) -> impl IntoElement {
    let caret = if library.searching { "\u{2502}" } else { "" };
    let (text, text_color) = if library.query.is_empty() && !library.searching {
        (
            SharedString::from(view.labels.search_placeholder.clone()),
            color::faint_text(cx),
        )
    } else {
        (
            SharedString::from(format!("{}{caret}", library.query)),
            color::foreground(cx),
        )
    };
    div()
        .id("documents-search")
        .cursor_pointer()
        .flex_1()
        .min_w(px(0.0))
        .flex()
        .items_center()
        .h(px(30.0))
        .px(px(10.0))
        .border_1()
        .border_color(if library.searching {
            color::accent(cx)
        } else {
            color::border(cx)
        })
        .text_size(px(12.5))
        .text_color(text_color)
        .truncate()
        .on_click(clicks.send(DocumentsPageEvent::StartSearch))
        .child(text)
}

fn sort_control(
    view: &DocumentsPageViewModel,
    clicks: &PageClicks<DocumentsPage>,
    library: &LibraryViewModel,
    cx: &App,
) -> impl IntoElement {
    let option = |sort: LibrarySort, label: String| {
        let active = library.sort == sort;
        div()
            .id(SharedString::from(format!("documents-sort-{sort:?}")))
            .cursor_pointer()
            .px(px(10.0))
            .py(px(6.0))
            .when(active, |this| {
                this.bg(color::selection_background(cx))
                    .text_color(color::selection_text(cx))
                    .font_weight(gpui::FontWeight::EXTRA_BOLD)
            })
            .on_click(clicks.send(DocumentsPageEvent::ClickSort(sort)))
            .child(label)
    };
    div()
        .flex_none()
        .flex()
        .border_1()
        .border_color(color::border(cx))
        .text_size(px(12.0))
        .child(option(LibrarySort::Newest, view.labels.newest.clone()))
        .child(option(LibrarySort::Expiring, view.labels.expiring.clone()))
}

/// The Library's rows, as a `uniform_list` so only the rows in view are built: drawing all of a
/// large Library on every keystroke made each one cost a full layout of every row. Every row is
/// the file glyph's height, which is why the chips keep to one line.
fn rows(
    view: &DocumentsPageViewModel,
    clicks: &PageClicks<DocumentsPage>,
    scroll: &Scrolls,
    library: &LibraryViewModel,
    rows: Rc<Vec<RowView>>,
    cx: &App,
) -> AnyElement {
    if rows.is_empty() {
        return div()
            .flex_1()
            .min_h(px(0.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(14.0))
                    .px(px(14.0))
                    .py(px(16.0))
                    .text_color(color::muted(cx))
                    .child(library.empty.clone())
                    .when(library.empty_offers_add, |this| {
                        this.child(secondary_button(
                            "documents-empty-add",
                            view.labels.add.clone(),
                            clicks.click(DocumentsPageEvent::Add),
                            cx,
                        ))
                    }),
            )
            .into_any_element();
    }
    let selected = library.selected;
    let not_linked = view.labels.not_linked.clone();
    let clicks = clicks.clone();
    uniform_list(
        "documents-rows",
        rows.len(),
        move |range: Range<usize>, _window: &mut Window, cx: &mut App| {
            range
                .map(|index| {
                    document_row(
                        &rows[index],
                        index == selected,
                        index,
                        &not_linked,
                        &clicks,
                        cx,
                    )
                    .into_any_element()
                })
                .collect::<Vec<_>>()
        },
    )
    .track_scroll(scroll.library.clone())
    .flex_1()
    .min_h(px(0.0))
    .into_any_element()
}

fn document_row(
    row: &RowView,
    selected: bool,
    index: usize,
    not_linked: &str,
    clicks: &PageClicks<DocumentsPage>,
    cx: &App,
) -> impl IntoElement {
    let (primary, secondary) = if selected {
        (color::selection_text(cx), color::selection_muted(cx))
    } else {
        (color::foreground(cx), color::muted(cx))
    };
    div()
        .id(("documents-row", index))
        .debug_selector(|| format!("documents-row-{index}"))
        .cursor_pointer()
        .flex()
        .items_center()
        .gap(px(12.0))
        .px(px(14.0))
        .py(px(7.0))
        .border_b(px(1.0))
        .border_color(color::hairline(cx))
        .when(selected, |this| {
            this.bg(color::selection_background(cx))
                .text_color(color::selection_text(cx))
        })
        .on_click(clicks.row(DocumentsPageEvent::ClickRow(index)))
        .child(file_glyph(row.extension, primary))
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .flex_col()
                .gap(px(3.0))
                .child(
                    div()
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .text_size(px(12.5))
                        .truncate()
                        .child(row.title.clone()),
                )
                .child(chips(&row.chips, secondary, selected, not_linked, cx)),
        )
        .child(
            div()
                .w(px(112.0))
                .flex_none()
                .flex()
                .flex_col()
                .items_end()
                .gap(px(3.0))
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(secondary)
                        .child(row.date.clone()),
                )
                .child(tail(&row.tail, selected, cx)),
        )
}

/// The 30×36 file glyph, its extension at the foot.
fn file_glyph(extension: &'static str, ink: gpui::Rgba) -> impl IntoElement {
    div()
        .flex_none()
        .w(px(30.0))
        .h(px(36.0))
        .border(px(1.5))
        .border_color(ink)
        .flex()
        .items_end()
        .justify_center()
        .pb(px(3.0))
        .text_color(ink)
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(8.0))
        .child(extension)
}

fn chips(
    chips: &[Chip],
    ink: gpui::Rgba,
    selected: bool,
    not_linked: &str,
    cx: &App,
) -> impl IntoElement {
    if chips.is_empty() {
        return div()
            .text_size(px(11.0))
            .text_color(if selected { ink } else { color::faint_text(cx) })
            .child(SharedString::from(not_linked.to_string()))
            .into_any_element();
    }
    div()
        .flex()
        .overflow_hidden()
        .whitespace_nowrap()
        .gap(px(10.0))
        .text_size(px(11.0))
        .text_color(ink)
        .children(chips.iter().map(|chip| {
            div()
                .flex()
                .items_center()
                .gap(px(4.0))
                .child(chip_icon(chip.kind, ink))
                .child(chip.label.clone())
        }))
        .into_any_element()
}

/// A 10px mark for what a chip links to. There is no Lucide set for these in the bundled assets, so
/// each is a short glyph rather than an SVG.
fn chip_icon(kind: ChipKind, ink: gpui::Rgba) -> impl IntoElement {
    let glyph = match kind {
        ChipKind::Account => "\u{25ad}",
        ChipKind::Item => "\u{25a3}",
        ChipKind::Amount => "\u{21c5}",
        ChipKind::Payee => "\u{25cf}",
        ChipKind::Bill => "\u{25a4}",
    };
    div().text_size(px(10.0)).text_color(ink).child(glyph)
}

fn tail(tail: &RowTail, selected: bool, cx: &App) -> impl IntoElement {
    match tail {
        RowTail::Type(label) => div()
            .font_weight(gpui::FontWeight::EXTRA_BOLD)
            .text_size(px(10.0))
            .text_color(if selected {
                color::selection_muted(cx)
            } else {
                color::faint_text(cx)
            })
            .child(label.clone()),
        RowTail::Flag { text, red } => div()
            .font_weight(gpui::FontWeight::EXTRA_BOLD)
            .text_size(px(10.5))
            .text_color(match (selected, red) {
                (true, true) => color::selection_negative_text(cx),
                (true, false) => color::selection_muted(cx),
                (false, true) => color::negative_text(cx),
                (false, false) => color::muted(cx),
            })
            .child(text.clone()),
    }
}

// ---------------------------------------------------------------------------------------------
// Detail pane
// ---------------------------------------------------------------------------------------------

fn detail_pane(
    view: &DocumentsPageViewModel,
    clicks: &PageClicks<DocumentsPage>,
    cx: &App,
) -> impl IntoElement {
    let pane = div()
        .w(DETAIL_WIDTH)
        .flex_none()
        .h_full()
        .flex()
        .flex_col()
        .bg(color::chrome(cx))
        .border_l(px(2.0))
        .border_color(color::structural_rule(cx));
    let Some(detail) = view
        .library
        .as_ref()
        .and_then(|library| library.detail.as_ref())
    else {
        return pane;
    };
    pane.child(preview(detail.extension, cx))
        .child(
            div()
                .id("documents-detail-body")
                .flex_1()
                .min_h(px(0.0))
                .overflow_y_scroll()
                .px(px(16.0))
                .py(px(12.0))
                .flex()
                .flex_col()
                .gap(px(10.0))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(2.0))
                        .child(
                            div()
                                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                                .text_size(px(15.0))
                                .line_height(gpui::relative(1.25))
                                .child(detail.title.clone()),
                        )
                        .child(
                            div()
                                .text_size(px(11.5))
                                .text_color(color::muted(cx))
                                .child(detail.meta.clone()),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .children(detail.facts.iter().map(|fact| fact_row(fact, cx))),
                )
                .child(linked_to(view, clicks, detail, cx)),
        )
        .child(detail_footer(view, clicks, cx))
}

/// The 200px preview well with a drawn page: real page-1 rendering is out of scope, so the page is
/// a placeholder carrying the file's extension.
fn preview(extension: &'static str, cx: &App) -> impl IntoElement {
    div()
        .flex_none()
        .h(px(200.0))
        .flex()
        .items_center()
        .justify_center()
        .bg(color::inset_track(cx))
        .border_b(px(2.0))
        .border_color(color::structural_rule(cx))
        .child(
            div()
                .w(px(128.0))
                .h(px(172.0))
                .p(px(12.0))
                .bg(color::background(cx))
                .shadow(vec![gpui::BoxShadow {
                    color: color::palette_shadow(cx).into(),
                    offset: gpui::point(px(0.0), px(2.0)),
                    blur_radius: px(6.0),
                    spread_radius: px(0.0),
                }])
                .flex()
                .flex_col()
                .justify_between()
                .child(
                    div().flex().flex_col().gap(px(6.0)).children(
                        [0.9_f32, 0.7, 0.8, 0.5, 0.75, 0.6]
                            .into_iter()
                            .map(|width| {
                                div()
                                    .h(px(4.0))
                                    .w(gpui::relative(width))
                                    .bg(color::hairline(cx))
                            }),
                    ),
                )
                .child(
                    div()
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .text_size(px(10.0))
                        .text_color(color::faint_text(cx))
                        .child(extension),
                ),
        )
}

fn fact_row(fact: &Fact, cx: &App) -> impl IntoElement {
    div()
        .flex()
        .justify_between()
        .gap(px(12.0))
        .py(px(5.0))
        .border_b(px(1.0))
        .border_color(color::hairline(cx))
        .text_size(px(12.0))
        .child(div().text_color(color::muted(cx)).child(fact.label.clone()))
        .child(
            div()
                .text_align(gpui::TextAlign::Right)
                .when(fact.red, |this| {
                    this.font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .text_color(color::negative_text(cx))
                })
                .child(fact.value.clone()),
        )
}

fn linked_to(
    view: &DocumentsPageViewModel,
    clicks: &PageClicks<DocumentsPage>,
    detail: &DetailView,
    cx: &App,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(10.0))
                .text_color(color::faint_text(cx))
                .child(upper(&view.labels.linked_to)),
        )
        .children(
            detail
                .links
                .iter()
                .enumerate()
                .map(|(index, link)| link_row(index, link, clicks, cx)),
        )
        .child(
            div()
                .id("documents-add-link")
                .cursor_pointer()
                .text_size(px(11.5))
                .text_color(color::muted(cx))
                .on_click(clicks.send(DocumentsPageEvent::OpenPicker))
                .child(view.labels.link_add.clone()),
        )
}

fn link_row(
    index: usize,
    link: &LinkRow,
    clicks: &PageClicks<DocumentsPage>,
    cx: &App,
) -> impl IntoElement {
    let target = link.link;
    div()
        .id(("documents-link", index))
        .debug_selector(|| format!("documents-link-{index}"))
        .cursor_pointer()
        .flex()
        .gap(px(8.0))
        .px(px(8.0))
        .py(px(6.0))
        .border_1()
        .border_color(color::border(cx))
        .bg(color::background(cx))
        .text_size(px(12.0))
        .on_click(clicks.send(DocumentsPageEvent::Reveal(target)))
        .child(
            div()
                .w(px(62.0))
                .flex_none()
                .text_color(color::faint_text(cx))
                .child(link.kind.clone()),
        )
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .child(link.name.clone()),
        )
        .child(
            div()
                .id(("documents-unlink", index))
                .debug_selector(|| format!("documents-unlink-{index}"))
                .flex_none()
                .cursor_pointer()
                .px(px(6.0))
                .text_color(color::faint_text(cx))
                .hover(|style| style.text_color(color::foreground(cx)))
                .on_click({
                    let unlink = clicks.send(DocumentsPageEvent::Unlink(target));
                    move |event, window, cx| {
                        cx.stop_propagation();
                        unlink(event, window, cx);
                    }
                })
                .child("\u{d7}"),
        )
}

fn detail_footer(
    view: &DocumentsPageViewModel,
    clicks: &PageClicks<DocumentsPage>,
    cx: &App,
) -> impl IntoElement {
    let hover = color::muted(cx);
    div()
        .flex_none()
        .flex()
        // The Delete button wraps to a row of its own where the pane is too narrow for three.
        .flex_wrap()
        .gap(px(8.0))
        .px(px(16.0))
        .py(px(12.0))
        .border_t(px(2.0))
        .border_color(color::structural_rule(cx))
        .child(
            div()
                .id("documents-open")
                .cursor_pointer()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .h(px(34.0))
                .bg(color::foreground(cx))
                .text_color(color::selection_text(cx))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .hover(move |style| style.bg(hover))
                .on_click(clicks.send(DocumentsPageEvent::Open))
                .child(view.labels.open.clone()),
        )
        .child(
            div()
                .id("documents-show")
                .cursor_pointer()
                .flex()
                .items_center()
                .justify_center()
                .h(px(34.0))
                .px(px(12.0))
                .border_1()
                .border_color(color::border(cx))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .on_click(clicks.send(DocumentsPageEvent::ShowInFolder))
                .child(view.labels.show_in_folder.clone()),
        )
        .child(delete_button(
            "documents-delete",
            view,
            clicks.click(DocumentsPageEvent::Delete),
            cx,
        ))
}

/// The footer's plain bordered Delete button, last in the row in both modes.
pub(super) fn delete_button(
    id: &'static str,
    view: &DocumentsPageViewModel,
    on_click: OnClick,
    cx: &App,
) -> impl IntoElement {
    div()
        .id(id)
        .debug_selector(|| id.to_string())
        .cursor_pointer()
        .flex()
        .items_center()
        .justify_center()
        .h(px(34.0))
        .px(px(12.0))
        .border_1()
        .border_color(color::border(cx))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .on_click(move |_event, window, cx| on_click(window, cx))
        .child(view.labels.delete.clone())
}
