//! The Display section's full-width **Colour Theme** group (`docs/colour-themes-design.md`
//! "Settings", decided on #303): a Light / Dark / System segmented control, then a wrapping grid
//! of mini-ledger preview cards, one per built-in Colour Theme, each drawn through `resolve` with
//! this Client's `[theme]` overrides so it shows what would actually appear.
//!
//! Which card and Appearance are chosen, and which card the keyboard is on, come from the Display
//! Presenter's view model; a click is its event through [`PageClicks`]. How a card looks, the
//! system's light or dark setting and what `[theme]` overrides make of it are toolkit colour work,
//! read from the `crate::theme::colours::Colours` Global. Focus and hover never preview, only a
//! click or Enter selects.

use std::cell::Cell;
use std::rc::Rc;

use gpui::{AnyElement, App, Context, FontWeight, Rgba, SharedString, canvas, div, prelude::*, px};
use lib_colour_theme::{ColourAppearance, ColourVariant, ResolvedColours};
use lib_core::configuration::ConfigKey;
use lib_settings::presenter::display::appearance_label;
use lib_settings::presenter::{ColourCardView, ColourThemeView, FieldView};
use lib_settings::{DisplayEvent, DisplayField};

use super::DisplayPage;
use crate::{
    form::fields::UNIFORM_OPTION_WIDTH,
    settings::controls::{segmented_choices, shadowed, shadowed_note},
    shell::view::page_clicks::PageClicks,
    theme::color,
    theme::colours::{self, to_gpui},
};

pub const CARD_WIDTH: f32 = 160.0;
pub const CARD_GAP: f32 = 12.0;

fn variant_label(variant: ColourVariant) -> String {
    appearance_label(match variant {
        ColourVariant::Light => ColourAppearance::Light,
        ColourVariant::Dark => ColourAppearance::Dark,
    })
}

/// "System (currently Dark)", or the undetected form naming the Dark fallback.
pub fn system_hint(system: Option<ColourVariant>) -> String {
    match system {
        Some(variant) => lib_locale::msg::colour_appearance_system_current(&variant_label(variant)),
        None => lib_locale::msg::colour_appearance_system_undetected(&variant_label(
            ColourVariant::for_appearance(ColourAppearance::System, None),
        )),
    }
}

/// How many cards fit on one row of a grid `grid_width` wide, for `j`/`k`: each card but the last
/// takes a gap after it.
pub fn grid_columns(grid_width: f32) -> usize {
    let fits = ((grid_width + CARD_GAP) / (CARD_WIDTH + CARD_GAP)).floor();
    if fits >= 1.0 { fits as usize } else { 1 }
}

/// The grid's own measure of how many cards fit a row: the count the page last told the Presenter,
/// so a draw at an unchanged width tells it nothing. It is the page's widget state, so a page
/// entered afresh measures again, and the Presenter declines a count it holds.
#[derive(Debug, Clone, Default)]
pub struct GridColumns(Rc<Cell<Option<usize>>>);

impl GridColumns {
    /// An invisible overlay on the grid that measures it as it paints. When the count its width
    /// fits changes, the page is told it once the draw is done, outside any input, and redraws
    /// with it, so the columns are the ones on screen before the next key (#1222).
    fn measure(&self, cx: &Context<'_, DisplayPage>) -> impl IntoElement {
        let told = self.0.clone();
        let page = cx.weak_entity();
        canvas(
            move |bounds, _window, cx| {
                let columns = grid_columns(f32::from(bounds.size.width));
                if told.replace(Some(columns)) != Some(columns) {
                    cx.defer(move |cx| {
                        // A page that has gone has no grid to lay out.
                        let _ = page.update(cx, |page, cx| {
                            page.told(DisplayEvent::SetGridColumns(columns), cx);
                        });
                    });
                }
            },
            |_bounds, _state, _window, _cx| {},
        )
        .absolute()
        .size_full()
    }
}

pub fn render(
    view: &ColourThemeView,
    appearance: &FieldView,
    clicks: &PageClicks<DisplayPage>,
    grid: &GridColumns,
    cx: &Context<'_, DisplayPage>,
) -> AnyElement {
    let state = colours::colours(cx);
    div()
        .id("display-colour-theme")
        .mt(px(28.0))
        .w_full()
        .flex()
        .flex_col()
        .child(field_label(appearance.label.clone(), cx))
        .child(segmented_choices(
            "display-colour-appearance",
            appearance,
            clicks.segment(|index| DisplayEvent::Choose {
                field: DisplayField::Appearance,
                index,
            }),
            Some(px(UNIFORM_OPTION_WIDTH)),
            cx,
        ))
        .child(note(system_hint(state.system()), cx))
        .children(shadowed(appearance, cx))
        .child(
            div()
                .mt(px(18.0))
                .child(field_label(view.label.clone(), cx)),
        )
        .children(shadowed_note(
            ConfigKey::ColourTheme,
            view.shadowed.as_deref(),
            cx,
        ))
        .child(
            div()
                .relative()
                .flex()
                .flex_wrap()
                .gap(px(CARD_GAP))
                .children(
                    view.cards
                        .iter()
                        .enumerate()
                        .map(|(index, view)| card(index, view, clicks, cx)),
                )
                .child(grid.measure(cx)),
        )
        .children(override_note(cx))
        .children(state.failures().iter().map(|failure| {
            warning(
                crate::msg::desktop_settings_display_colour_contrast(
                    failure.subject,
                    failure.surface,
                    &format!("{:.1}", failure.ratio),
                    &format!("{}", failure.required),
                ),
                cx,
            )
        }))
        .into_any_element()
}

/// Names the overridden roles; nothing when `[theme]` overrides none in the variant drawn.
fn override_note(cx: &App) -> Option<impl IntoElement> {
    let variant = colours::resolved(cx).variant;
    let roles = colours::colours(cx).overrides().roles(variant);
    if roles.is_empty() {
        return None;
    }
    let roles = roles
        .iter()
        .map(|role| role.key())
        .collect::<Vec<_>>()
        .join(", ");
    Some(note(
        crate::msg::desktop_settings_display_colour_overrides(&roles),
        cx,
    ))
}

fn card(
    index: usize,
    view: &ColourCardView,
    clicks: &PageClicks<DisplayPage>,
    cx: &App,
) -> impl IntoElement {
    let chosen = view.chosen;
    let preview = colours::colours(cx).preview(view.id);
    // The focus ring sits outside the chosen border so the two never hide each other.
    div()
        .debug_selector(move || format!("display-colour-theme-{index}"))
        .id(SharedString::from(format!("display-colour-theme-{index}")))
        .cursor_pointer()
        .w(px(CARD_WIDTH + 4.0))
        .p(px(2.0))
        .border_1()
        .border_color(if view.focused {
            color::foreground(cx)
        } else {
            gpui::transparent_black().into()
        })
        .on_click(clicks.send(DisplayEvent::ClickColourTheme(view.id)))
        .child(
            div()
                .border(px(if chosen { 2.0 } else { 1.0 }))
                .border_color(if chosen {
                    color::accent(cx)
                } else {
                    color::border(cx)
                })
                .child(mini_ledger(&preview)),
        )
        .child(
            div()
                .mt(px(6.0))
                .text_size(px(12.0))
                .when(chosen, |this| this.font_weight(FontWeight::EXTRA_BOLD))
                .child(view.name.clone()),
        )
}

/// A miniature ledger in `c`: a header bar with an accent chip, then three rows (the middle
/// one selected) carrying a positive and a negative amount.
fn mini_ledger(c: &ResolvedColours) -> impl IntoElement {
    let bg = c.roles.background;
    let rgba = |colour: lib_colour_theme::Rgba| -> Rgba { to_gpui(colour.over(bg)) };
    let bar = |width: f32, colour: Rgba| div().w(px(width)).h(px(4.0)).bg(colour);
    // The selected row's amount takes its on-selection shade, as the real ledger does.
    let row = |selected: bool, amount: Rgba| {
        let (text, amount) = if selected {
            (
                to_gpui(c.selection.text),
                to_gpui(c.selection.text_shades.negative),
            )
        } else {
            (rgba(c.roles.foreground), amount)
        };
        div()
            .flex()
            .items_center()
            .justify_between()
            .px(px(6.0))
            .h(px(14.0))
            .when(selected, |this| this.bg(to_gpui(c.selection.background)))
            .border_b(px(1.0))
            .border_color(rgba(c.hairline))
            .child(bar(44.0, text))
            .child(bar(20.0, amount))
    };
    div()
        .h(px(84.0))
        .bg(to_gpui(bg))
        .flex()
        .flex_col()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .px(px(6.0))
                .h(px(18.0))
                .bg(rgba(c.chrome))
                .child(bar(36.0, rgba(c.roles.muted)))
                .child(div().w(px(18.0)).h(px(8.0)).bg(to_gpui(c.roles.accent))),
        )
        .child(row(false, to_gpui(c.text_shades.positive)))
        .child(row(true, to_gpui(c.text_shades.negative)))
        .child(row(false, rgba(c.roles.muted)))
}

fn field_label(label: impl Into<SharedString>, cx: &App) -> impl IntoElement {
    div()
        .text_size(px(12.0))
        .mb(px(5.0))
        .text_color(color::muted(cx))
        .child(label.into())
}

fn note(text: String, cx: &App) -> impl IntoElement {
    div()
        .mt(px(6.0))
        .text_size(px(12.0))
        .text_color(color::muted(cx))
        .child(text)
}

fn warning(text: String, cx: &App) -> impl IntoElement {
    div()
        .mt(px(4.0))
        .text_size(px(12.0))
        .text_color(color::negative_text(cx))
        .child(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_columns_fits_cards_and_never_drops_below_one() {
        assert_eq!(grid_columns(160.0), 1);
        assert_eq!(grid_columns(3.0 * 160.0 + 2.0 * 12.0), 3);
        assert_eq!(grid_columns(3.0 * 160.0 + 2.0 * 12.0 - 1.0), 2);
        assert_eq!(grid_columns(100.0), 1);
    }

    #[test]
    fn system_hint_names_the_variant_or_the_dark_fallback() {
        crate::locale::init_for_tests();
        assert!(system_hint(Some(ColourVariant::Light)).contains("Light"));
        assert!(system_hint(None).contains("Dark"));
    }
}
