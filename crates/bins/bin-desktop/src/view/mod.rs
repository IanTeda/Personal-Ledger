//! View interiors, one module per noun that has one built. Dashboard (issue #148), Settings
//! (issue #173), Accounts (issue #150), Categories (issue #272), Payees (issue #286), Tags (issue #357), Documents, Bills (issue #371) and Budgets (issue #402) exist so far -- the rest are
//! placeholder views, a separate ticket (#153).

pub mod active;
pub mod bills;
pub mod budgets;
pub mod dashboard;
pub mod documents;
pub mod event;
pub(crate) mod format;
pub mod help;
pub mod import;
pub mod inventory;
pub mod placeholder;
pub mod transactions;

use gpui::{ScrollHandle, point, px};

use crate::navigation::key_router::Movement;

/// A single `j`/`k`/`Down`/`Up` step in a scrolling page, in logical pixels -- there's no literal
/// "row height" for a dashboard of charts and figures, so this is a plain reading-sized
/// increment, not a computed value.
const VIEW_LINE_STEP: f32 = 40.0;

/// Scrolls a page that has no list to move through by one movement, kept within its content.
pub fn scroll_by_movement(handle: &ScrollHandle, movement: Movement) {
    let offset = handle.offset();
    let max_height = f32::from(handle.max_offset().height);
    let viewport_height = f32::from(handle.bounds().size.height);
    let current_y = f32::from(offset.y);

    let new_y = match movement {
        Movement::Next => current_y - VIEW_LINE_STEP,
        Movement::Prev => current_y + VIEW_LINE_STEP,
        Movement::First => 0.0,
        Movement::Last => -max_height,
        Movement::HalfPageDown => current_y - viewport_height / 2.0,
        Movement::HalfPageUp => current_y + viewport_height / 2.0,
        // No action is defined for `Enter` in the View zone by the handoff's "Movement" bullet
        // (only the primary and context rails are named) -- a no-op until one is.
        Movement::Enter => current_y,
    };
    let clamped_y = new_y.clamp(-max_height, 0.0);
    handle.set_offset(point(offset.x, px(clamped_y)));
}
