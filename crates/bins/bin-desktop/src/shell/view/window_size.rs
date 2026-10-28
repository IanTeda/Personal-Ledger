//! The Desktop's side of the Window Size (ADR-0042, ADR-0050): `lib_shell` owns the enum and its
//! Breakpoints, and this adapter reads it from the viewport while a View draws, so nothing keeps a
//! copy that could go stale on a resize. The window-only sizes live here, since they use gpui types.

use gpui::{Pixels, Size, Window, px, size};
pub use lib_shell::WindowSize;

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

/// What gpui adds to [`WindowSize`]: reading it from a window, and the frame in gpui's types.
pub trait WindowSizeExt {
    /// The Window Size the window is drawn at right now.
    fn of(window: &Window) -> Self;

    /// The frame each Window Size's mockups are drawn at (ADR-0042's table), which the UI tests
    /// resize to.
    fn reference_frame(self) -> Size<Pixels>;
}

impl WindowSizeExt for WindowSize {
    fn of(window: &Window) -> Self {
        Self::from_logical_px(f32::from(window.viewport_size().width))
    }

    fn reference_frame(self) -> Size<Pixels> {
        let (width, height) = self.reference_frame_px();
        size(px(width), px(height))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
                WindowSize::from_logical_px(f32::from(window_size.reference_frame().width)),
                window_size
            );
        }
    }
}
