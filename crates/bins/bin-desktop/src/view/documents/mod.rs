//! The **Documents** destination (`docs/ux/desktop/Documents/`): the 190px index rail, the list pane
//! (header, toolbar, count line and rows) and the 300px detail pane. The Library (4a) is drawn
//! here and the Inbox (4b) in `inbox`, each filling the list and detail panes.
//!
//! Every action is a callback into `Shell`, so keys and clicks share handlers. The detail pane is
//! never focused: its actions are list keys (`enter`, `o`, `l`).

pub mod dialogs;
mod inbox;
pub mod model;

use std::rc::Rc;

use gpui::{AnyElement, App, Pixels, ScrollHandle, SharedString, Window, div, prelude::*, px};
use lib_locale::format::upper;

use self::model::{
    Chip, ChipKind, DetailView, Fact, InboxDetailView, InboxRowView, LinkRow, RailRow, RowTail,
    RowView,
};
use crate::{
    documents::{DocumentLink, DocumentsMode, LibrarySort, RailEntry},
    theme::color,
};

pub type OnPlainClick = Rc<dyn Fn(&mut Window, &mut App)>;
pub type OnRailClick = Rc<dyn Fn(RailEntry, &mut Window, &mut App)>;
pub type OnRowClick = Rc<dyn Fn(usize, &mut Window, &mut App)>;
pub type OnSortClick = Rc<dyn Fn(LibrarySort, &mut Window, &mut App)>;
pub type OnLinkClick = Rc<dyn Fn(DocumentLink, &mut Window, &mut App)>;
/// A Transaction id: an Other candidate to accept instead of the Suggested Link.
pub type OnCandidateClick = Rc<dyn Fn(u32, &mut Window, &mut App)>;

const RAIL_WIDTH: Pixels = px(190.0);
const DETAIL_WIDTH: Pixels = px(300.0);

/// Whether keyboard focus is on the index rail or the list, while the View zone has it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DocumentsFocus {
    Index,
    #[default]
    List,
}

pub struct DocumentsPageProps {
    pub mode: DocumentsMode,
    pub focus: DocumentsFocus,
    pub rail: Vec<RailRow>,
    /// The rail row the scope or mode selects.
    pub rail_selected: RailEntry,
    /// The stored size footer, already formatted.
    pub rail_footer: String,
    pub subline: String,
    pub query: String,
    pub searching: bool,
    pub sort: LibrarySort,
    pub count_line: String,
    pub rows: Vec<RowView>,
    pub selected: usize,
    pub detail: Option<DetailView>,
    /// The Inbox's rows (newest first, Skipped last), the focused index and its detail.
    pub inbox_rows: Vec<InboxRowView>,
    pub inbox_selected: usize,
    pub inbox_detail: Option<InboxDetailView>,
    /// How many Documents have a Strong Suggested Link: the Accept all button's count.
    pub inbox_strong: usize,
    /// The key strip under the Inbox detail: each key and what it does.
    pub inbox_hints: Vec<(&'static str, String)>,
    /// Whether the list zone has keyboard focus, which draws the focused Inbox row's outline.
    pub list_focused: bool,
    pub scroll: ScrollHandle,
    pub on_rail_click: OnRailClick,
    pub on_row_click: OnRowClick,
    pub on_sort_click: OnSortClick,
    pub on_search_click: OnPlainClick,
    pub on_add_click: OnPlainClick,
    pub on_import_click: OnPlainClick,
    pub on_open_click: OnPlainClick,
    pub on_show_click: OnPlainClick,
    pub on_link_click: OnLinkClick,
    /// The × on a LINKED TO row: removes that Link.
    pub on_unlink_click: OnLinkClick,
    pub on_add_link_click: OnPlainClick,
    pub on_watched_click: OnPlainClick,
    pub on_accept_all_click: OnPlainClick,
    /// A row's own Accept (or File…) button.
    pub on_accept_row_click: OnRowClick,
    pub on_accept_next_click: OnPlainClick,
    pub on_accept_candidate_click: OnCandidateClick,
    pub on_link_elsewhere_click: OnPlainClick,
    pub on_skip_click: OnPlainClick,
}

/// The whole destination: rail, list pane and detail pane, side by side.
pub fn render(focused: bool, props: DocumentsPageProps, cx: &App) -> AnyElement {
    let list_focused = focused && props.focus == DocumentsFocus::List;
    let index_focused = focused && props.focus == DocumentsFocus::Index;
    let props = DocumentsPageProps {
        list_focused,
        ..props
    };
    div()
        .id("documents")
        .flex_1()
        .min_w(px(0.0))
        .h_full()
        .flex()
        .child(index_rail(&props, index_focused, cx))
        .child(list_pane(&props, list_focused, cx))
        .child(if props.mode == DocumentsMode::Library {
            detail_pane(&props, cx).into_any_element()
        } else {
            inbox::detail_pane(&props, cx)
        })
        .into_any_element()
}

// ---------------------------------------------------------------------------------------------
// Index rail
// ---------------------------------------------------------------------------------------------

fn index_rail(props: &DocumentsPageProps, focused: bool, cx: &App) -> impl IntoElement {
    let mut column = div().flex_1().min_h(px(0.0)).flex().flex_col().py(px(10.0));
    for (index, row) in props.rail.iter().enumerate() {
        // The section labels sit above the first Type and the first Financial Year.
        if let RailEntry::Scope(scope) = row.entry
            && let Some(label) = section_label(scope, index, &props.rail)
        {
            column = column.child(rail_label(label, cx));
        }
        column = column.child(rail_row(
            row,
            row.entry == props.rail_selected,
            props.on_rail_click.clone(),
            cx,
        ));
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
        .when(focused, |this| {
            this.border_l(px(2.0)).border_color(color::foreground(cx))
        })
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
                .child(props.rail_footer.clone()),
        )
}

/// The caps label that opens a section, if `scope` is its first row.
fn section_label(
    scope: crate::documents::LibraryScope,
    index: usize,
    rows: &[RailRow],
) -> Option<String> {
    use crate::documents::LibraryScope;
    let previous = index.checked_sub(1).and_then(|at| rows.get(at));
    match scope {
        LibraryScope::Type(_)
            if !matches!(
                previous.map(|row| row.entry),
                Some(RailEntry::Scope(LibraryScope::Type(_)))
            ) =>
        {
            Some(crate::msg::desktop_documents_rail_type())
        }
        LibraryScope::Year(_)
            if !matches!(
                previous.map(|row| row.entry),
                Some(RailEntry::Scope(LibraryScope::Year(_)))
            ) =>
        {
            Some(crate::msg::desktop_documents_rail_year())
        }
        _ => None,
    }
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

fn rail_row(row: &RailRow, active: bool, on_click: OnRailClick, cx: &App) -> impl IntoElement {
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
        .on_click(move |_event, window, cx| on_click(entry, window, cx))
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

fn list_pane(props: &DocumentsPageProps, focused: bool, cx: &App) -> impl IntoElement {
    let library = props.mode == DocumentsMode::Library;
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
        .child(list_header(props, cx))
        .when(library, |this| {
            this.child(toolbar(props, cx))
                .child(
                    div()
                        .px(px(14.0))
                        .pt(px(8.0))
                        .pb(px(4.0))
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .text_size(px(10.0))
                        .text_color(color::faint_text(cx))
                        .child(props.count_line.clone()),
                )
                .child(rows(props, cx))
        })
        .when(!library, |this| this.child(inbox::list_body(props, cx)))
}

fn list_header(props: &DocumentsPageProps, cx: &App) -> impl IntoElement {
    let library = props.mode == DocumentsMode::Library;
    let title = if library {
        crate::nav::Noun::Documents.label()
    } else {
        crate::msg::desktop_documents_inbox_title()
    };
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
                        .child(title),
                )
                .child(
                    div()
                        .text_size(px(11.5))
                        .text_color(color::faint_text(cx))
                        .child(props.subline.clone()),
                ),
        )
        .children(if library {
            vec![
                secondary_button(
                    "documents-import",
                    crate::msg::desktop_documents_import_button(),
                    props.on_import_click.clone(),
                    cx,
                )
                .into_any_element(),
                primary_button(
                    "documents-add",
                    crate::msg::desktop_documents_add_button(),
                    props.on_add_click.clone(),
                    cx,
                )
                .into_any_element(),
            ]
        } else {
            inbox::header_buttons(props, cx).into()
        })
}

fn primary_button(
    id: &'static str,
    label: String,
    on_click: OnPlainClick,
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
    on_click: OnPlainClick,
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

fn toolbar(props: &DocumentsPageProps, cx: &App) -> impl IntoElement {
    div()
        .flex_none()
        .flex()
        .items_center()
        .gap(px(8.0))
        .px(px(14.0))
        .py(px(10.0))
        .border_b(px(1.0))
        .border_color(color::border(cx))
        .child(search_box(props, cx))
        .child(sort_control(props, cx))
}

/// The search input (30px, 12.5px): the placeholder, or the text so far with a caret and an accent
/// border while search mode is active.
fn search_box(props: &DocumentsPageProps, cx: &App) -> impl IntoElement {
    let on_click = props.on_search_click.clone();
    let caret = if props.searching { "\u{2502}" } else { "" };
    let (text, text_color) = if props.query.is_empty() && !props.searching {
        (
            SharedString::from(crate::msg::desktop_documents_search_placeholder()),
            color::faint_text(cx),
        )
    } else {
        (
            SharedString::from(format!("{}{caret}", props.query)),
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
        .border_color(if props.searching {
            color::accent(cx)
        } else {
            color::border(cx)
        })
        .text_size(px(12.5))
        .text_color(text_color)
        .truncate()
        .on_click(move |_event, window, cx| on_click(window, cx))
        .child(text)
}

fn sort_control(props: &DocumentsPageProps, cx: &App) -> impl IntoElement {
    let option = |sort: LibrarySort, label: String| {
        let on_click = props.on_sort_click.clone();
        let active = props.sort == sort;
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
            .on_click(move |_event, window, cx| on_click(sort, window, cx))
            .child(label)
    };
    div()
        .flex_none()
        .flex()
        .border_1()
        .border_color(color::border(cx))
        .text_size(px(12.0))
        .child(option(
            LibrarySort::Newest,
            crate::msg::desktop_documents_sort_newest(),
        ))
        .child(option(
            LibrarySort::Expiring,
            crate::msg::desktop_documents_sort_expiring(),
        ))
}

fn rows(props: &DocumentsPageProps, cx: &App) -> impl IntoElement {
    let body = div()
        .id("documents-rows")
        .flex_1()
        .min_h(px(0.0))
        .overflow_y_scroll()
        .track_scroll(&props.scroll);
    if props.rows.is_empty() {
        let note = if props.query.trim().is_empty() {
            crate::msg::desktop_documents_empty_scope()
        } else {
            crate::msg::desktop_documents_empty_search()
        };
        return body.child(
            div()
                .flex()
                .items_center()
                .gap(px(14.0))
                .px(px(14.0))
                .py(px(16.0))
                .text_color(color::muted(cx))
                .child(note)
                .when(props.query.trim().is_empty(), |this| {
                    this.child(secondary_button(
                        "documents-empty-add",
                        crate::msg::desktop_documents_add_button(),
                        props.on_add_click.clone(),
                        cx,
                    ))
                }),
        );
    }
    body.children(props.rows.iter().enumerate().map(|(index, row)| {
        document_row(
            row,
            index == props.selected,
            index,
            props.on_row_click.clone(),
            cx,
        )
    }))
}

fn document_row(
    row: &RowView,
    selected: bool,
    index: usize,
    on_click: OnRowClick,
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
        .on_click(move |_event, window, cx| on_click(index, window, cx))
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
                .child(chips(&row.chips, secondary, selected, cx)),
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

fn chips(chips: &[Chip], ink: gpui::Rgba, selected: bool, cx: &App) -> impl IntoElement {
    if chips.is_empty() {
        return div()
            .text_size(px(11.0))
            .text_color(if selected { ink } else { color::faint_text(cx) })
            .child(crate::msg::desktop_documents_not_linked())
            .into_any_element();
    }
    div()
        .flex()
        .flex_wrap()
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

fn detail_pane(props: &DocumentsPageProps, cx: &App) -> impl IntoElement {
    let pane = div()
        .w(DETAIL_WIDTH)
        .flex_none()
        .h_full()
        .flex()
        .flex_col()
        .bg(color::chrome(cx))
        .border_l(px(2.0))
        .border_color(color::structural_rule(cx));
    let Some(detail) = props.detail.as_ref() else {
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
                .child(linked_to(props, detail, cx)),
        )
        .child(detail_footer(props, cx))
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

fn linked_to(props: &DocumentsPageProps, detail: &DetailView, cx: &App) -> impl IntoElement {
    let on_add_link = props.on_add_link_click.clone();
    div()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(10.0))
                .text_color(color::faint_text(cx))
                .child(upper(&crate::msg::desktop_documents_linked_to())),
        )
        .children(detail.links.iter().enumerate().map(|(index, link)| {
            link_row(
                index,
                link,
                props.on_link_click.clone(),
                props.on_unlink_click.clone(),
                cx,
            )
        }))
        .child(
            div()
                .id("documents-add-link")
                .cursor_pointer()
                .text_size(px(11.5))
                .text_color(color::muted(cx))
                .on_click(move |_event, window, cx| on_add_link(window, cx))
                .child(crate::msg::desktop_documents_link_add()),
        )
}

fn link_row(
    index: usize,
    link: &LinkRow,
    on_click: OnLinkClick,
    on_unlink: OnLinkClick,
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
        .on_click(move |_event, window, cx| on_click(target, window, cx))
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
                .on_click(move |_event, window, cx| {
                    cx.stop_propagation();
                    on_unlink(target, window, cx);
                })
                .child("\u{d7}"),
        )
}

fn detail_footer(props: &DocumentsPageProps, cx: &App) -> impl IntoElement {
    let on_open = props.on_open_click.clone();
    let on_show = props.on_show_click.clone();
    let hover = color::muted(cx);
    div()
        .flex_none()
        .flex()
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
                .on_click(move |_event, window, cx| on_open(window, cx))
                .child(crate::msg::desktop_documents_open()),
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
                .on_click(move |_event, window, cx| on_show(window, cx))
                .child(crate::msg::desktop_documents_show_in_folder()),
        )
}
