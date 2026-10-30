//! The log-capture feed of `Root`, which keeps the Settings › Tracing page current. An `impl Root`
//! block; the model is `settings::tracing::log::LogView`.

use std::sync::Arc;
use std::time::Duration;

use gpui::Context;

use super::Root;
use crate::settings::tracing::{
    TracingPage,
    log::{LogView, TracingLevel},
};

/// How long the Tracing page lets a burst of log events settle before redrawing, capping live
/// updates at about ten a second. Public so the headless tests advance past exactly this.
#[doc(hidden)]
pub const LOG_COALESCE: Duration = Duration::from_millis(100);

impl Root {
    /// Hands the Tracing page the app's live log capture, opening on `level`, and starts the
    /// feed that pulls new entries in. Called once, by `shell::router::build`.
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
        let log = LogView::new(buffer, level);
        let tracing = self.views.settings.read(cx).page::<TracingPage>();
        tracing.update(cx, |page, cx| page.replace_log(cx, log));
        cx.spawn(async move |this, cx| {
            loop {
                notify.notified().await;
                // The executor's timer, not `Timer::after`, so headless tests can advance it.
                cx.background_executor().timer(LOG_COALESCE).await;
                // Stops once the window, and with it the Shell, has gone.
                if this
                    .update(cx, |shell, cx| {
                        let tracing = shell.views.settings.read(cx).page::<TracingPage>();
                        tracing.update(cx, |page, cx| page.pull_log(cx));
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
    }
}
