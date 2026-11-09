//! The link picker in the command palette's frame: a query line, the kind filter, filing's Document
//! Type line and a capped, scrolling list of rows.
//!
//! Typing goes to the query, `tab` cycles the kind filter and, while filing, `←`/`→` step the
//! Document Type. `↑`/`↓` move the highlight; `select` or a click picks. What each key means is the Presenter's.

use std::ops::Range;

use gpui::{AnyElement, App, IntoElement, div, prelude::*, px};
use lib_documents::presenter::{
    LinkPickerEvent, LinkPickerPresenter, LinkPickerViewModel, PickerRowView, Purpose,
};
use lib_shell::Hint;

use crate::dialog;
use crate::documents::view::snapshot::PickerSnapshot;
use crate::shell::view::dialog_entity::{DialogView, ModuleDialogClicks};
use crate::theme::color;

type Clicks = ModuleDialogClicks<LinkPickerPresenter>;

/// The `lib-documents` link picker Presenter's [`DialogView`], hosted in a `ModuleDialog`
/// (ADR-0056). A row click picks it, as `select` does on the highlight.
impl DialogView for LinkPickerPresenter {
    type Widgets = ();

    fn kind(view: &LinkPickerViewModel) -> &'static str {
        match view.purpose {
            Purpose::Link(_) => "picker-link",
            Purpose::File(_) => "picker-file",
            Purpose::Follow(_) => "picker-follow",
        }
    }

    fn render(
        view: &LinkPickerViewModel,
        _widgets: &mut (),
        clicks: &Clicks,
        cx: &App,
    ) -> AnyElement {
        render(view, clicks, cx)
    }

    /// The picker draws no confirm button, so it reads back only its title.
    fn readback(view: &LinkPickerViewModel) -> Vec<(&'static str, String)> {
        vec![("title", view.title.clone())]
    }

    fn hints(view: &LinkPickerViewModel) -> Vec<Hint> {
        view.hints.clone()
    }
}

/// Rows drawn at once; the list scrolls past this, the palette's own capped-window idiom.
const VISIBLE_ROWS: usize = 9;

/// What a test reads of the open picker.
pub(crate) fn snapshot(view: &LinkPickerViewModel) -> PickerSnapshot {
    PickerSnapshot {
        kind: view.kind.clone(),
        query: view.query.clone(),
        selected: view.selected,
        rows: view
            .rows
            .iter()
            .map(|row| {
                (
                    row.kind.clone(),
                    row.text.clone(),
                    row.checked,
                    row.link.is_some(),
                )
            })
            .collect(),
        doc_type_chosen: view
            .type_line
            .as_ref()
            .is_some_and(|line| line.chosen.is_some()),
    }
}

/// The slice of `len` rows to draw so `selected` stays on screen in a `cap`-row window, scrolling
/// only as far as it must.
fn visible_window(selected: usize, len: usize, cap: usize) -> Range<usize> {
    let start = (selected + 1)
        .saturating_sub(cap)
        .min(len.saturating_sub(cap));
    start..(start + cap).min(len)
}

fn render(view: &LinkPickerViewModel, actions: &Clicks, cx: &App) -> AnyElement {
    let window = visible_window(view.selected, view.rows.len(), VISIBLE_ROWS);

    let query_line = div()
        .flex()
        .items_center()
        .gap(px(10.0))
        .px(px(16.0))
        .py(px(12.0))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(15.0))
        .text_color(color::foreground(cx))
        .child(div().text_color(color::muted(cx)).child(view.title.clone()))
        .child(div().flex_1().child(if view.query.is_empty() {
            div()
                .font_weight(gpui::FontWeight::NORMAL)
                .text_color(color::faint_text(cx))
                .child(view.placeholder.clone())
        } else {
            div().child(view.query.clone())
        }))
        .child(div().w(px(8.0)).h(px(17.0)).bg(color::cursor(cx)));

    let kinds = view.kinds.as_ref().map(|kinds| {
        div()
            .flex()
            .gap(px(14.0))
            .px(px(16.0))
            .py(px(8.0))
            .text_size(px(11.5))
            .children(kinds.iter().map(|tab| {
                div()
                    .when(tab.active, |this| {
                        this.font_weight(gpui::FontWeight::EXTRA_BOLD)
                            .text_color(color::foreground(cx))
                            .border_b_2()
                            .border_color(color::foreground(cx))
                    })
                    .when(!tab.active, |this| this.text_color(color::muted(cx)))
                    .child(tab.label.clone())
            }))
    });

    let type_line = view.type_line.as_ref().map(|line| {
        div()
            .flex()
            .gap(px(8.0))
            .px(px(16.0))
            .py(px(6.0))
            .text_size(px(12.0))
            .child(
                div()
                    .text_color(color::faint_text(cx))
                    .child(line.label.clone()),
            )
            .child(match &line.chosen {
                Some(label) => div()
                    .font_weight(gpui::FontWeight::EXTRA_BOLD)
                    .child(format!("\u{2039} {label} \u{203a}")),
                None => div().text_color(color::muted(cx)).child(line.none.clone()),
            })
    });

    let list = div()
        .flex()
        .flex_col()
        .children(
            view.rows[window.clone()]
                .iter()
                .enumerate()
                .map(|(offset, row)| {
                    let index = window.start + offset;
                    let on_pick = actions.send(LinkPickerEvent::PickRow(index));
                    picker_row(index, row, index == view.selected, on_pick, cx)
                }),
        );

    div()
        .absolute()
        .top(crate::shell::view::palette::TOP_OFFSET)
        .left(px(0.0))
        .right(px(0.0))
        .flex()
        .justify_center()
        .child(
            div()
                .w(crate::shell::view::palette::WIDTH)
                .flex()
                .flex_col()
                .bg(color::background(cx))
                .border_2()
                .border_color(color::foreground(cx))
                .child(query_line)
                .child(div().h(px(2.0)).bg(color::foreground(cx)))
                .children(kinds)
                .children(type_line)
                .when(view.rows.is_empty(), |this| {
                    this.child(
                        div()
                            .px(px(16.0))
                            .py(px(12.0))
                            .text_size(px(12.5))
                            .text_color(color::muted(cx))
                            .child(view.empty.clone()),
                    )
                })
                .child(list)
                .child(div().h(px(1.0)).bg(color::hairline(cx)))
                .child(
                    div()
                        .px(px(16.0))
                        .py(px(8.0))
                        .text_size(px(11.5))
                        .text_color(color::muted(cx))
                        .child(view.footer.clone()),
                ),
        )
        .into_any_element()
}

fn picker_row(
    index: usize,
    row: &PickerRowView,
    selected: bool,
    on_pick: dialog::OnClick,
    cx: &App,
) -> impl IntoElement {
    let (bg, text, faint) = if selected {
        (
            Some(color::selection_background(cx)),
            color::selection_text(cx),
            color::selection_muted(cx),
        )
    } else {
        (None, color::foreground(cx), color::faint_text(cx))
    };
    div()
        .id(("documents-picker-row", index))
        .debug_selector(|| format!("documents-picker-row-{index}"))
        .cursor_pointer()
        .when_some(bg, |this, bg| this.bg(bg))
        .flex()
        .items_center()
        .gap(px(12.0))
        .px(px(16.0))
        .py(px(8.0))
        .text_size(px(13.0))
        .text_color(text)
        .on_click(move |_event, window, cx| on_pick(window, cx))
        .child(
            div()
                .w(px(14.0))
                .flex_none()
                .child(if row.checked { "\u{2713}" } else { "" }),
        )
        .child(
            div()
                .w(px(78.0))
                .flex_none()
                .text_size(px(11.5))
                .text_color(faint)
                .child(row.kind.clone()),
        )
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis()
                .when(row.link.is_none(), |this| {
                    this.font_weight(gpui::FontWeight::EXTRA_BOLD)
                })
                .child(row.text.clone()),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_window_scrolls_only_as_far_as_the_highlight_needs() {
        assert_eq!(visible_window(0, 20, 9), 0..9);
        assert_eq!(visible_window(8, 20, 9), 0..9);
        assert_eq!(visible_window(9, 20, 9), 1..10);
        assert_eq!(visible_window(19, 20, 9), 11..20);
        assert_eq!(visible_window(1, 3, 9), 0..3);
        assert_eq!(visible_window(0, 0, 9), 0..0);
    }
}
