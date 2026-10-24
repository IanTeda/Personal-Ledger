//! The Window Size (ADR-0042): Medium, Large or X-Large, picked from the window's width alone. The
//! The Shell and each View read it from the viewport while they draw, so nothing keeps a copy that
//! could go stale on a resize.

use gpui::{Pixels, Size, Window, px, size};

/// The narrowest window Large is drawn at; anything narrower is Medium.
pub const LARGE_BREAKPOINT: Pixels = px(1200.0);

/// The narrowest window X-Large is drawn at.
pub const X_LARGE_BREAKPOINT: Pixels = px(1600.0);

/// The smallest the window can be made. Medium is the floor, so no layout is designed below it.
pub const MIN_WINDOW: Size<Pixels> = size(px(720.0), px(480.0));

/// The gap a floating overlay (the palette, a Toast) keeps from each side of a window too narrow
/// for its own width.
pub const OVERLAY_MARGIN: Pixels = px(16.0);

/// An overlay's width in a window `viewport_width` wide: its own `width`, or the window less
/// [`OVERLAY_MARGIN`] each side when that is narrower.
pub fn overlay_width(width: Pixels, viewport_width: Pixels) -> Pixels {
    width
        .min(viewport_width - OVERLAY_MARGIN * 2.0)
        .max(px(0.0))
}

/// What the window's width makes room for. Height never picks one: a short window scrolls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowSize {
    Medium,
    Large,
    XLarge,
}

impl WindowSize {
    /// The Window Size for a window this wide. Below the floor is still Medium, since the window
    /// can't be made narrower than [`MIN_WINDOW`].
    pub fn from_width(width: Pixels) -> Self {
        if width >= X_LARGE_BREAKPOINT {
            Self::XLarge
        } else if width >= LARGE_BREAKPOINT {
            Self::Large
        } else {
            Self::Medium
        }
    }

    /// The Window Size the window is drawn at right now.
    pub fn of(window: &Window) -> Self {
        Self::from_width(window.viewport_size().width)
    }

    /// The frame each Window Size's mockups are drawn at (ADR-0042's table), which the UI tests
    /// resize to.
    pub fn reference_frame(self) -> Size<Pixels> {
        match self {
            Self::Medium => size(px(960.0), px(1040.0)),
            Self::Large => size(px(1280.0), px(800.0)),
            Self::XLarge => size(px(1920.0), px(1080.0)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_width_picks_by_the_breakpoints() {
        let cases = [
            (719.0, WindowSize::Medium),
            (720.0, WindowSize::Medium),
            (1199.0, WindowSize::Medium),
            (1200.0, WindowSize::Large),
            (1599.0, WindowSize::Large),
            (1600.0, WindowSize::XLarge),
        ];
        for (width, expected) in cases {
            assert_eq!(WindowSize::from_width(px(width)), expected, "at {width}px");
        }
    }

    #[test]
    fn an_overlay_keeps_its_width_until_the_window_is_too_narrow_for_it() {
        assert_eq!(overlay_width(px(640.0), px(720.0)), px(640.0));
        assert_eq!(overlay_width(px(640.0), px(672.0)), px(640.0));
        assert_eq!(overlay_width(px(640.0), px(600.0)), px(568.0));
        assert_eq!(overlay_width(px(420.0), px(20.0)), px(0.0));
    }

    #[test]
    fn each_reference_frame_is_its_own_window_size() {
        for window_size in [WindowSize::Medium, WindowSize::Large, WindowSize::XLarge] {
            assert_eq!(
                WindowSize::from_width(window_size.reference_frame().width),
                window_size
            );
        }
    }
}
