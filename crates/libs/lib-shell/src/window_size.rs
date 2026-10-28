//! The Window Size (ADR-0042, ADR-0050): Medium, Large or X-Large, picked from the window's width
//! alone. Both Clients read it here, the Desktop from logical pixels and the TUI from columns, so
//! the Breakpoints cannot drift. Nothing keeps a copy: a View reads it while it draws.

/// The narrowest window Large is drawn at, in logical pixels; anything narrower is Medium.
pub const LARGE_BREAKPOINT_PX: f32 = 1200.0;

/// The narrowest window X-Large is drawn at, in logical pixels.
pub const X_LARGE_BREAKPOINT_PX: f32 = 1600.0;

/// The design scale that maps a terminal column to logical pixels. It is not a measurement of the
/// user's font: it makes the TUI's 96-column floor the Desktop's 720px floor.
pub const PX_PER_COLUMN: f32 = 7.5;

/// The narrowest terminal Large is drawn at, in columns.
pub const LARGE_COLUMNS: u16 = 160;

/// The narrowest terminal X-Large is drawn at, in columns.
pub const X_LARGE_COLUMNS: u16 = 214;

/// The TUI's floor. A narrower terminal gets the "terminal too small" screen.
pub const MIN_COLUMNS: u16 = 96;

/// The TUI's floor in rows.
pub const MIN_ROWS: u16 = 24;

/// What the window's width makes room for. Height never picks one: a short window scrolls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowSize {
    Medium,
    Large,
    XLarge,
}

impl WindowSize {
    /// The Window Size for a window this many logical pixels wide. Below the floor is still
    /// Medium, since the Desktop window can't be made narrower than its minimum.
    pub fn from_logical_px(width: f32) -> Self {
        if width >= X_LARGE_BREAKPOINT_PX {
            Self::XLarge
        } else if width >= LARGE_BREAKPOINT_PX {
            Self::Large
        } else {
            Self::Medium
        }
    }

    /// The Window Size for a terminal this many columns wide. Below [`MIN_COLUMNS`] is still
    /// Medium: the Shell draws "terminal too small" there instead of a View.
    pub fn from_columns(columns: u16) -> Self {
        if columns >= X_LARGE_COLUMNS {
            Self::XLarge
        } else if columns >= LARGE_COLUMNS {
            Self::Large
        } else {
            Self::Medium
        }
    }

    /// The frame each Window Size's mockups are drawn at, as (width, height) in logical pixels
    /// (ADR-0042's table).
    pub fn reference_frame_px(self) -> (f32, f32) {
        match self {
            Self::Medium => (960.0, 1040.0),
            Self::Large => (1280.0, 800.0),
            Self::XLarge => (1920.0, 1080.0),
        }
    }

    /// The frame each Window Size's TUI mockups are drawn at, as (columns, rows) (ADR-0050's
    /// table).
    pub fn reference_frame_cells(self) -> (u16, u16) {
        match self {
            Self::Medium => (128, 69),
            Self::Large => (171, 53),
            Self::XLarge => (256, 72),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_logical_px_picks_by_the_breakpoints() {
        let cases = [
            (719.0, WindowSize::Medium),
            (720.0, WindowSize::Medium),
            (1199.0, WindowSize::Medium),
            (1200.0, WindowSize::Large),
            (1599.0, WindowSize::Large),
            (1600.0, WindowSize::XLarge),
        ];
        for (width, expected) in cases {
            assert_eq!(WindowSize::from_logical_px(width), expected, "at {width}px");
        }
    }

    #[test]
    fn from_columns_picks_by_the_breakpoints() {
        let cases = [
            (MIN_COLUMNS, WindowSize::Medium),
            (159, WindowSize::Medium),
            (160, WindowSize::Large),
            (213, WindowSize::Large),
            (214, WindowSize::XLarge),
        ];
        for (columns, expected) in cases {
            assert_eq!(
                WindowSize::from_columns(columns),
                expected,
                "at {columns} columns"
            );
        }
    }

    #[test]
    fn each_column_breakpoint_is_the_first_column_reaching_the_px_breakpoint() {
        for (columns, breakpoint) in [
            (LARGE_COLUMNS, LARGE_BREAKPOINT_PX),
            (X_LARGE_COLUMNS, X_LARGE_BREAKPOINT_PX),
        ] {
            assert!(f32::from(columns) * PX_PER_COLUMN >= breakpoint);
            assert!(f32::from(columns - 1) * PX_PER_COLUMN < breakpoint);
        }
    }

    #[test]
    fn the_floor_is_the_desktops_720px_minimum() {
        assert_eq!(f32::from(MIN_COLUMNS) * PX_PER_COLUMN, 720.0);
        assert_eq!(MIN_ROWS, 24);
    }

    #[test]
    fn each_reference_frame_is_its_own_window_size() {
        for window_size in [WindowSize::Medium, WindowSize::Large, WindowSize::XLarge] {
            assert_eq!(
                WindowSize::from_logical_px(window_size.reference_frame_px().0),
                window_size
            );
            assert_eq!(
                WindowSize::from_columns(window_size.reference_frame_cells().0),
                window_size
            );
        }
    }
}
