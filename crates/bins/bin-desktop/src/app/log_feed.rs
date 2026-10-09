//! The log-capture feed of `Shell`, which keeps the Settings › Tracing page current. An `impl Shell`
//! block; the model is `settings::tracing_log::LogView`.

use std::sync::Arc;
use std::time::Duration;

use gpui::{Context, ListOffset, px};

use super::Shell;
use crate::settings::tracing_log::{LogChange, LogView, TracingLevel};

/// How long the Tracing page lets a burst of log events settle before redrawing, capping live
/// updates at about ten a second. Public so the headless tests advance past exactly this.
#[doc(hidden)]
pub const LOG_COALESCE: Duration = Duration::from_millis(100);

/// Rows measured beyond the Tracing log box's visible edge, so scrolling doesn't pop rows in.
pub(super) const LOG_LIST_OVERDRAW: gpui::Pixels = px(200.0);

/// One `j`/`k` step in the Tracing log box: a line of its 11px/1.6 monospace.
pub(super) const LOG_LINE_STEP: gpui::Pixels = px(17.6);

impl Shell {
    /// Hands the Tracing page the app's live log capture, opening on `level`, and starts the
    /// feed that pulls new entries in. Called once, by `build_shell`.
    pub fn set_log_capture(
        &mut self,
        buffer: lib_tracing::LogBuffer,
        level: TracingLevel,
        cx: &mut Context<'_, Self>,
    ) {
        let notify = Arc::new(tokio::sync::Notify::new());
        let waker = notify.clone();
        // Runs on whichever thread logged: only signal. One stored permit absorbs a burst.
        buffer.set_waker(move || waker.notify_one());
        self.settings_log = LogView::new(buffer, level);
        self.settings_log_list
            .reset(self.settings_log.visible().len());
        cx.spawn(async move |this, cx| {
            loop {
                notify.notified().await;
                // The executor's timer, not `Timer::after`, so headless tests can advance it.
                cx.background_executor().timer(LOG_COALESCE).await;
                // Stops once the window, and with it the Shell, has gone.
                if this
                    .update(cx, |shell, cx| {
                        let change = shell.settings_log.pull();
                        if change != LogChange::default() {
                            shell.apply_log_change(change);
                            cx.notify();
                        }
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
    }

    /// Mirrors a pull onto the list: evicted rows leave the bottom, new ones arrive at the top.
    /// A reader at the very top keeps seeing the newest; one scrolled down stays where they are.
    pub(super) fn apply_log_change(&mut self, change: LogChange) {
        let list = &self.settings_log_list;
        let top = list.logical_scroll_top();
        let at_top = top.item_ix == 0 && top.offset_in_item <= px(0.0);
        let count = list.item_count();
        let evicted = change.evicted.min(count);
        list.splice(count - evicted..count, 0);
        list.splice(0..0, change.added);
        if at_top {
            list.scroll_to(ListOffset::default());
        }
    }
}
