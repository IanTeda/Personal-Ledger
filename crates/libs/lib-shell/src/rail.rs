//! The primary rail's two drawn states and the rule that picks one (ADR-0042). Which is drawn
//! follows the Window Size, not `NavState`; the Presenter holds only the `b` override.

use crate::window_size::WindowSize;

/// The primary rail's own two drawn states: expanded, or collapsed to icons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RailMode {
    Expanded,
    Collapsed,
}

/// A `b` toggle, kept with the Window Size it was made at: it holds only until the window is drawn
/// at another, which drops it back to that Window Size's default (ADR-0042).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RailOverride {
    pub(crate) set_at: WindowSize,
    pub(crate) mode: RailMode,
}

/// The primary rail each Window Size draws until `b` overrides it: icons at Medium, expanded at
/// Large and X-Large unless Settings › Display's "start with the rail hidden" is on.
fn default_rail(window_size: WindowSize, start_hidden: bool) -> RailMode {
    match window_size {
        WindowSize::Medium => RailMode::Collapsed,
        WindowSize::Large | WindowSize::XLarge if start_hidden => RailMode::Collapsed,
        WindowSize::Large | WindowSize::XLarge => RailMode::Expanded,
    }
}

/// The rail drawn at `window_size`: the override while the window is still at the size it was set
/// at, otherwise the default.
pub(crate) fn rail_mode(
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
pub(crate) fn toggled(
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
