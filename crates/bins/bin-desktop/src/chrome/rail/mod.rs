//! The two navigation rails (`docs/ux/desktop-mockups/01-shell/README.md`'s "1a" spec):
//! the primary rail (every noun) and the context rail (scoped to the active noun).

pub mod context;
pub mod primary;
pub mod settings_index;

use crate::{chrome::window_size::WindowSize, navigation::nav::RailMode};

/// The primary rail each Window Size draws until `b` overrides it: icons at Medium, expanded at
/// Large and X-Large unless Settings › Display's "start with the rail hidden" is on.
pub fn default_rail(window_size: WindowSize, start_hidden: bool) -> RailMode {
    match window_size {
        WindowSize::Medium => RailMode::Collapsed,
        WindowSize::Large | WindowSize::XLarge if start_hidden => RailMode::Collapsed,
        WindowSize::Large | WindowSize::XLarge => RailMode::Expanded,
    }
}

/// A `b` toggle, kept with the Window Size it was made at: it holds only until the window crosses
/// a Breakpoint, which drops it back to that Window Size's [`default_rail`] (ADR-0042).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RailOverride {
    pub set_at: WindowSize,
    pub mode: RailMode,
}

/// The rail drawn at `window_size`: the override while the window is still at the size it was set
/// at, otherwise the default.
pub fn rail_mode(
    window_size: WindowSize,
    start_hidden: bool,
    rail_override: Option<RailOverride>,
) -> RailMode {
    rail_override
        .filter(|rail_override| rail_override.set_at == window_size)
        .map_or_else(
            || default_rail(window_size, start_hidden),
            |rail_override| rail_override.mode,
        )
}

/// The override `b` leaves at `window_size`: the opposite of the rail drawn there now.
pub fn toggled(
    window_size: WindowSize,
    start_hidden: bool,
    rail_override: Option<RailOverride>,
) -> RailOverride {
    let mode = match rail_mode(window_size, start_hidden, rail_override) {
        RailMode::Expanded => RailMode::Collapsed,
        RailMode::Collapsed => RailMode::Expanded,
    };
    RailOverride {
        set_at: window_size,
        mode,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn medium_draws_icons_and_the_wider_sizes_draw_the_rail_expanded() {
        assert_eq!(default_rail(WindowSize::Medium, false), RailMode::Collapsed);
        assert_eq!(default_rail(WindowSize::Large, false), RailMode::Expanded);
        assert_eq!(default_rail(WindowSize::XLarge, false), RailMode::Expanded);
    }

    #[test]
    fn start_hidden_applies_at_large_and_x_large_only() {
        assert_eq!(default_rail(WindowSize::Medium, true), RailMode::Collapsed);
        assert_eq!(default_rail(WindowSize::Large, true), RailMode::Collapsed);
        assert_eq!(default_rail(WindowSize::XLarge, true), RailMode::Collapsed);
    }

    #[test]
    fn an_override_holds_at_its_window_size_and_drops_at_another() {
        let expanded_at_medium = toggled(WindowSize::Medium, false, None);
        assert_eq!(expanded_at_medium.mode, RailMode::Expanded);
        let held = Some(expanded_at_medium);

        assert_eq!(
            rail_mode(WindowSize::Medium, false, held),
            RailMode::Expanded
        );
        assert_eq!(
            rail_mode(WindowSize::Large, true, held),
            RailMode::Collapsed
        );
    }

    #[test]
    fn toggling_twice_comes_back_to_the_default() {
        let once = toggled(WindowSize::Large, false, None);
        let twice = toggled(WindowSize::Large, false, Some(once));
        assert_eq!(twice.mode, RailMode::Expanded);
    }
}
