//! The caret concern of `Root`: the blink poll that toggles every text box's caret
//! (`form::caret`), redrawing only while one is on screen. An `impl Root` block.

use gpui::Context;

use super::Root;
use crate::form::caret::{self, CARET_BLINK};

impl Root {
    /// Toggles the caret every [`CARET_BLINK`] for the window's life. Stops once the window, and
    /// with it the Shell, has gone. Called once, by `shell::router::build`.
    pub fn start_caret_blink(&self, cx: &mut Context<'_, Self>) {
        caret::install(cx);
        cx.spawn(async move |this, cx| {
            loop {
                // The executor's timer, not `Timer::after`, so headless tests can advance it.
                cx.background_executor().timer(CARET_BLINK).await;
                let toggled = this.update(cx, |_, cx| {
                    if caret::tick(cx) {
                        cx.notify();
                    }
                });
                if toggled.is_err() {
                    break;
                }
            }
        })
        .detach();
    }
}
