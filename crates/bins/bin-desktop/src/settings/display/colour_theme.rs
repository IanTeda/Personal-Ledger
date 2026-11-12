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

use gpui::{AnyElement, App, FontWeight, Rgba, SharedString, div, prelude::*, px};
use lib_colour_theme::{ColourAppearance, ColourVariant, ResolvedColours};
use lib_settings::presenter::display::appearance_label;
use lib_settings::presenter::{ColourCardView, ColourThemeView, ControlView};
use lib_settings::{ColourChoice, DeviceKey, DisplayEvent};

use super::DisplayPage;
use crate::{
    form::fields::{UNIFORM_OPTION_WIDTH, segmented_labelled},
    settings::controls::shadowed,
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

/// How many cards fit on one grid row in a Settings body `body_width` wide, for `j`/`k`.
pub fn grid_columns(body_width: f32) -> usize {
    // The body's 28px side padding and its 2px focus border.
    let inner = body_width - 58.0;
    let fits = ((inner + CARD_GAP) / (CARD_WIDTH + CARD_GAP)).floor();
    if fits >= 1.0 { fits as usize } else { 1 }
}

pub fn render(
    view: &ColourThemeView,
    appearance: &ControlView<ColourAppearance>,
    clicks: &PageClicks<DisplayPage>,
    cx: &App,
) -> AnyElement {
    let state = colours::colours(cx);
    let select = &appearance.select;
    let options = select
        .options
        .iter()
        .map(|choice| (choice.value, SharedString::from(choice.label.clone())))
        .collect();
    div()
        .id("display-colour-theme")
        .mt(px(28.0))
        .w_full()
        .flex()
        .flex_col()
        .child(field_label(select.label.clone(), cx))
        .child(segmented_labelled(
            "display-colour-appearance",
            options,
            select.chosen.unwrap_or_default(),
            clicks.segment(|appearance| {
                DisplayEvent::ChooseColour(ColourChoice::Appearance(appearance))
            }),
            Some(px(UNIFORM_OPTION_WIDTH)),
            cx,
        ))
        .child(note(system_hint(state.system()), cx))
        .children(shadowed(
            DeviceKey::ColourAppearance,
            appearance.shadowed.as_deref(),
            cx,
        ))
        .child(
            div()
                .mt(px(18.0))
                .child(field_label(view.label.clone(), cx)),
        )
        .children(shadowed(
            DeviceKey::ColourTheme,
            view.shadowed.as_deref(),
            cx,
        ))
        .child(
            div().flex().flex_wrap().gap(px(CARD_GAP)).children(
                view.cards
                    .iter()
                    .enumerate()
                    .map(|(index, view)| card(index, view, clicks, cx)),
            ),
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
        assert_eq!(grid_columns(58.0 + 160.0), 1);
        assert_eq!(grid_columns(58.0 + 3.0 * 160.0 + 2.0 * 12.0), 3);
        assert_eq!(grid_columns(100.0), 1);
    }

    #[test]
    fn system_hint_names_the_variant_or_the_dark_fallback() {
        crate::locale::init_for_tests();
        assert!(system_hint(Some(ColourVariant::Light)).contains("Light"));
        assert!(system_hint(None).contains("Dark"));
    }
}
