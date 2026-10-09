//! The Display section's full-width **Colour Theme** group (`docs/colour-themes-design.md`
//! "Settings", decided on #303): a Light / Dark / System segmented control, then a wrapping grid
//! of mini-ledger preview cards, one per built-in Colour Theme, each drawn through `resolve` with
//! this Client's `[theme]` overrides so it shows what would actually appear.
//!
//! Reads and writes the `crate::theme::colours::Colours` Global directly: the Preferences live there,
//! not on `Shell`. Only the keyboard focus inside the grid is `Shell` state, since `Shell` owns
//! every keystroke; focus and hover never preview, only a click or Enter selects.

use std::rc::Rc;

use gpui::{AnyElement, App, FontWeight, Rgba, SharedString, Window, div, prelude::*, px};
use lib_colour_theme::{ColourAppearance, ColourTheme, ColourVariant, ResolvedColours};

use crate::{
    theme::color,
    theme::colours::{self, ColourChange, to_gpui},
};

use crate::view::form_fields::{UNIFORM_OPTION_WIDTH, segmented_control_sized};

/// A click on a card, curried with its Colour Theme id.
pub type OnColourThemeClick = Rc<dyn Fn(&'static str, &mut Window, &mut App)>;

pub const CARD_WIDTH: f32 = 160.0;
pub const CARD_GAP: f32 = 12.0;

const APPEARANCES: [ColourAppearance; 3] = [
    ColourAppearance::Light,
    ColourAppearance::Dark,
    ColourAppearance::System,
];

/// The Colour Theme's name, a shared Message.
pub fn colour_theme_name(id: &str) -> String {
    match id {
        "high_contrast" => lib_locale::msg::colour_theme_high_contrast(),
        "catppuccin" => lib_locale::msg::colour_theme_catppuccin(),
        "gruvbox" => lib_locale::msg::colour_theme_gruvbox(),
        "nord" => lib_locale::msg::colour_theme_nord(),
        _ => lib_locale::msg::colour_theme_modernist(),
    }
}

pub fn appearance_label(appearance: ColourAppearance) -> String {
    match appearance {
        ColourAppearance::Light => lib_locale::msg::colour_appearance_light(),
        ColourAppearance::Dark => lib_locale::msg::colour_appearance_dark(),
        ColourAppearance::System => lib_locale::msg::colour_appearance_system(),
    }
}

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
            ColourAppearance::System.variant(None),
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

/// The card focus after `key` in a grid of `len` cards, `columns` to a row; `None` when the key
/// is not a grid movement. Stops at the edges rather than wrapping.
pub fn grid_move(index: usize, len: usize, columns: usize, key: &str) -> Option<usize> {
    let last = len.checked_sub(1)?;
    let columns = columns.max(1);
    let next = match key {
        "left" | "h" => index.saturating_sub(1),
        "right" | "l" => (index + 1).min(last),
        "up" | "k" => index.checked_sub(columns).unwrap_or(index),
        "down" | "j" => {
            if index + columns <= last {
                index + columns
            } else {
                index
            }
        }
        _ => return None,
    };
    Some(next)
}

/// The index of the Colour Theme drawn now, where grid focus starts.
pub fn chosen_index(cx: &App) -> usize {
    let chosen = colours::resolved(cx).theme_id;
    ColourTheme::built_in()
        .iter()
        .position(|theme| theme.id == chosen)
        .unwrap_or(0)
}

pub fn render(focused: Option<usize>, on_theme_click: OnColourThemeClick, cx: &App) -> AnyElement {
    let state = colours::colours(cx);
    let appearance = state.colour_appearance().unwrap_or_default();
    let on_appearance_click = Rc::new(
        |appearance: ColourAppearance, _window: &mut Window, cx: &mut App| {
            ColourChange::Appearance(appearance).apply(cx);
        },
    );
    div()
        .id("display-colour-theme")
        .mt(px(28.0))
        .w_full()
        .flex()
        .flex_col()
        .child(field_label(
            crate::msg::desktop_settings_display_colour_appearance(),
            cx,
        ))
        .child(segmented_control_sized(
            "display-colour-appearance",
            &APPEARANCES,
            appearance,
            appearance_label,
            on_appearance_click,
            Some(px(UNIFORM_OPTION_WIDTH)),
            cx,
        ))
        .child(note(system_hint(state.system()), cx))
        .child(div().mt(px(18.0)).child(field_label(
            crate::msg::desktop_settings_display_colour_theme(),
            cx,
        )))
        .child(
            div().flex().flex_wrap().gap(px(CARD_GAP)).children(
                ColourTheme::built_in()
                    .iter()
                    .enumerate()
                    .map(|(index, theme)| {
                        card(
                            index,
                            theme.id,
                            focused == Some(index),
                            on_theme_click.clone(),
                            cx,
                        )
                    }),
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
    id: &'static str,
    focused: bool,
    on_click: OnColourThemeClick,
    cx: &App,
) -> impl IntoElement {
    let chosen = colours::resolved(cx).theme_id == id;
    let preview = colours::colours(cx).preview(id);
    // The focus ring sits outside the chosen border so the two never hide each other.
    div()
        .debug_selector(move || format!("display-colour-theme-{index}"))
        .id(SharedString::from(format!("display-colour-theme-{index}")))
        .cursor_pointer()
        .w(px(CARD_WIDTH + 4.0))
        .p(px(2.0))
        .border_1()
        .border_color(if focused {
            color::foreground(cx)
        } else {
            gpui::transparent_black().into()
        })
        .on_click(move |_event, window, cx| on_click(id, window, cx))
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
                .child(colour_theme_name(id)),
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
    fn grid_move_steps_within_the_row_and_stops_at_the_ends() {
        assert_eq!(grid_move(0, 5, 3, "l"), Some(1));
        assert_eq!(grid_move(0, 5, 3, "h"), Some(0));
        assert_eq!(grid_move(4, 5, 3, "right"), Some(4));
    }

    #[test]
    fn grid_move_steps_a_row_only_onto_a_card() {
        assert_eq!(grid_move(1, 5, 3, "j"), Some(4));
        assert_eq!(grid_move(2, 5, 3, "down"), Some(2), "no card below");
        assert_eq!(grid_move(4, 5, 3, "k"), Some(1));
        assert_eq!(grid_move(1, 5, 3, "up"), Some(1));
    }

    #[test]
    fn grid_move_ignores_other_keys_and_an_empty_grid() {
        assert_eq!(grid_move(0, 5, 3, "x"), None);
        assert_eq!(grid_move(0, 0, 3, "l"), None);
    }

    #[test]
    fn grid_columns_fits_cards_and_never_drops_below_one() {
        assert_eq!(grid_columns(58.0 + 160.0), 1);
        assert_eq!(grid_columns(58.0 + 3.0 * 160.0 + 2.0 * 12.0), 3);
        assert_eq!(grid_columns(100.0), 1);
    }

    #[test]
    fn every_built_in_colour_theme_has_its_own_name() {
        crate::locale::init_for_tests();
        let names: Vec<String> = ColourTheme::built_in()
            .iter()
            .map(|theme| colour_theme_name(theme.id))
            .collect();
        for (index, name) in names.iter().enumerate() {
            assert!(!names[..index].contains(name), "{name} named twice");
        }
    }

    #[test]
    fn system_hint_names_the_variant_or_the_dark_fallback() {
        crate::locale::init_for_tests();
        assert!(system_hint(Some(ColourVariant::Light)).contains("Light"));
        assert!(system_hint(None).contains("Dark"));
    }
}
