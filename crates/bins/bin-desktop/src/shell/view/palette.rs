//! The floating command palette (`docs/ux/desktop-mockups/01-shell/README.md`'s "1d" spec, the
//! Shell's `:`/`InputMode::Command` state), drawn from `lib-shell`'s palette Presenter's view model
//! (`lib_shell::palette`), which holds the input, matching, selection and history (#1063). What is
//! left here is gpui's: the layout and the row list's scroll handle, which `Root` keeps on the
//! selected row.

use gpui::{App, BoxShadow, div, point, prelude::*, px};
use lib_shell::{PaletteEntryRow, PaletteKeys, PaletteRow, PaletteViewModel};

use crate::{shell::view::window_size, theme::color};

/// Fixed width. Narrower than the "1d" spec's own `820px` -- another deliberate override (see
/// `crate::navigation::command`'s own module doc for the domain-grouping one): now that the binding sits
/// right next to the name instead of pushed to the row's far edge, 820px left the description
/// column far wider than the registry's own longest description ever needs, sized instead to
/// comfortably fit that longest description ("the tags every transaction can carry any number
/// of", 50 characters) plus the name/binding columns before it.
pub const WIDTH: gpui::Pixels = px(640.0);

/// Distance from the window's top edge: the "1d" spec's own `top: 96px`.
pub const TOP_OFFSET: gpui::Pixels = px(96.0);

/// Width of the `<command>` column -- fixed (rather than `flex_1`, which used to push the
/// binding off to the row's far edge) so the binding sits left-aligned right next to the name
/// it triggers, freeing the rest of the row for the description. Sized to comfortably fit the
/// registry's longest names ("transactions", "accounts delete") at this text size.
const NAME_COLUMN_WIDTH: gpui::Pixels = px(130.0);

/// Width of the `<binding>` column, immediately after the name column.
const BINDING_COLUMN_WIDTH: gpui::Pixels = px(64.0);

/// Caps the row list at roughly the height the registry's 14 commands took up before domain
/// headers existed (issue #144's domain-grouping change) -- rendering every header on top of
/// every command made the resting-state list noticeably taller than before, so it scrolls past
/// this instead of growing further. Mirrors `crate::navigation::explorer::MAX_ROWS_HEIGHT`'s own reasoning.
const MAX_ROWS_HEIGHT: gpui::Pixels = px(448.0);

/// The floating overlay: `position: absolute`, centred at [`TOP_OFFSET`] from the window's top
/// edge (the "1d" spec's `left: 50%; transform: translateX(-50%)`, achieved here with a full-width
/// absolute row that centres its one child, since `gpui` has no transform to recentre).
/// `scroll_handle` is the row list's, which `Root` keeps on the selected row.
pub fn render(
    model: &PaletteViewModel,
    scroll_handle: &gpui::ScrollHandle,
    viewport_width: gpui::Pixels,
    cx: &App,
) -> gpui::AnyElement {
    let rendered_rows = model.rows.iter().map(|row| match row {
        PaletteRow::Header(domain) => domain_header(domain, cx).into_any_element(),
        PaletteRow::Entry(entry) => result_row(entry, cx).into_any_element(),
    });

    div()
        .absolute()
        .top(TOP_OFFSET)
        .left(px(0.0))
        .right(px(0.0))
        .flex()
        .justify_center()
        .child(
            div()
                .debug_selector(|| "palette".to_string())
                .w(window_size::overlay_width(WIDTH, viewport_width))
                .flex()
                .flex_col()
                .bg(color::background(cx))
                .border_t(px(2.0))
                .border_b(px(2.0))
                .border_l(px(2.0))
                .border_r(px(2.0))
                .border_color(color::foreground(cx))
                .shadow(vec![BoxShadow {
                    color: color::palette_shadow(cx).into(),
                    offset: point(px(0.0), px(12.0)),
                    blur_radius: px(32.0),
                    spread_radius: px(0.0),
                }])
                .child(input_row(&model.input, model.match_count, model.total, cx))
                .child(div().h(px(2.0)).bg(color::foreground(cx)))
                .child(
                    div()
                        .id("palette-rows")
                        .flex()
                        .flex_col()
                        .max_h(MAX_ROWS_HEIGHT)
                        .overflow_y_scroll()
                        .track_scroll(scroll_handle)
                        .children(rendered_rows),
                )
                .child(div().h(px(1.0)).bg(color::hairline(cx)))
                .child(footer_row(&model.keys, cx)),
        )
        .into_any_element()
}

/// A domain header in the resting-state list (`PaletteRow::Header`) -- mirrors `bin-tui`'s own
/// section-header styling for its command popup (dim, extra-bold, uppercase, small).
fn domain_header(domain: &str, cx: &App) -> impl IntoElement {
    div()
        .px(px(16.0))
        .pt(px(10.0))
        .pb(px(4.0))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(10.0))
        .text_color(color::faint_text(cx))
        .child(lib_locale::format::upper(domain))
}

/// The "1d" spec's input row: leading `:` (matching the key that opens it; the spec draws `>`), the live query with a block caret, right-aligned
/// match count.
fn input_row(input: &str, match_count: usize, total: usize, cx: &App) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(px(10.0))
        .px(px(16.0))
        .py(px(12.0))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(15.0))
        .text_color(color::foreground(cx))
        .child(":")
        .child(div().flex_1().child(input.to_string()))
        .child(div().w(px(8.0)).h(px(17.0)).bg(color::cursor(cx)))
        .child(
            div()
                .font_weight(gpui::FontWeight::NORMAL)
                .text_size(px(11.5))
                .text_color(color::muted(cx))
                .child(crate::msg::desktop_palette_match_count(
                    &match_count.to_string(),
                    &total.to_string(),
                )),
        )
}

/// One result row: the command name with its matched substring picked out, then the binding
/// column (an em dash when the command has none), then the description -- selected styling
/// inverts per the "1d" spec ("Selected result: ink fill, ground text, accent-on-dark
/// substring").
fn result_row(entry: &PaletteEntryRow, cx: &App) -> impl IntoElement {
    let (bg, text, matched, description_color, binding_color) = if entry.selected {
        (
            Some(color::selection_background(cx)),
            color::selection_text(cx),
            color::selection_accent_text(cx),
            color::selection_muted(cx),
            color::selection_muted(cx),
        )
    } else {
        (
            None,
            color::foreground(cx),
            color::accent_text(cx),
            color::muted(cx),
            color::faint_text(cx),
        )
    };

    div()
        .when_some(bg, |this, bg| this.bg(bg))
        .flex()
        .items_center()
        .gap(px(12.0))
        .px(px(16.0))
        .py(px(8.0))
        .child(
            div()
                .w(NAME_COLUMN_WIDTH)
                .flex_none()
                .text_color(text)
                .child(highlighted_name(entry, matched, text)),
        )
        .child(
            div()
                .w(BINDING_COLUMN_WIDTH)
                .flex_none()
                .text_size(px(11.5))
                .text_color(binding_color)
                .child(entry.key.clone()),
        )
        .child(
            div()
                .flex_1()
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis()
                .text_color(description_color)
                .child(entry.description.clone()),
        )
}

/// The "1d" spec's footer hint row, below the rule under the result list: every key the palette
/// answers to, as bound now, in the handoff's own order and wording.
fn footer_row(keys: &PaletteKeys, cx: &App) -> impl IntoElement {
    div()
        .px(px(16.0))
        .py(px(8.0))
        .text_size(px(11.5))
        .text_color(color::muted(cx))
        .child(
            [
                crate::msg::desktop_palette_hint_select(&keys.step_previous, &keys.step_next),
                crate::msg::desktop_palette_hint_complete(&keys.complete),
                crate::msg::desktop_palette_hint_run(&keys.run),
                crate::msg::desktop_palette_hint_history(&keys.history),
                crate::msg::desktop_palette_hint_close(&keys.close),
            ]
            .join(" \u{b7} "),
        )
}

/// The name with the part the query matched in `matched_color` at extra-bold weight and the rest
/// in `default_color`; at rest nothing is picked out.
fn highlighted_name(
    entry: &PaletteEntryRow,
    matched_color: gpui::Rgba,
    default_color: gpui::Rgba,
) -> gpui::AnyElement {
    let name = entry.name;
    let Some(range) = entry
        .matched
        .clone()
        .filter(|range| name.get(range.clone()).is_some())
    else {
        return div()
            .text_color(default_color)
            .child(name)
            .into_any_element();
    };

    div()
        .flex()
        .child(name[..range.start].to_string())
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_color(matched_color)
                .child(name[range.clone()].to_string()),
        )
        .child(name[range.end..].to_string())
        .into_any_element()
}
