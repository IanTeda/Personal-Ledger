//! The price-fetching concern of `Root`: the minute timer that reaches the Open Ledger's
//! `LedgerPrices` scheduled check, and polling it to land each response as one input, so the round it
//! completes is that input's one commit and the Units repaint on its `LedgerChanged` (ADR-0062).
//! The same poll pumps the calls waiting their source's call rate or `Retry-After` out. It wakes
//! even when idle, as its twin `WRITE_FAILURE_POLL` does, so stopping only this one would save no
//! wake-up (#1405). An `impl Root` block.

use std::time::Duration;

use gpui::{Context, Task};
use lib_units::presenter::Received;

use super::Root;

/// How often the Shell looks for a price response. The feed answers on a blocking channel, so the
/// Shell polls it rather than holding a thread on it. Public so the headless tests advance past
/// exactly this.
#[doc(hidden)]
pub const PRICE_RESPONSE_POLL: Duration = Duration::from_millis(250);

/// How often the Open Ledger checks for Units due a price. Public so the headless tests advance
/// past exactly this.
#[doc(hidden)]
pub use lib_units::presenter::PRICE_CHECK;

impl Root {
    /// Polls the Open Ledger's prices for as long as the returned task is held, landing each
    /// response as an input, then pumps the queued calls out. Stops once the feed has gone, the
    /// window with the Shell, or the Open Ledger holding the task.
    pub(super) fn listen_for_price_responses(cx: &mut Context<'_, Self>) -> Task<()> {
        cx.spawn(async move |this, cx| {
            loop {
                // The executor's timer, not `Timer::after`, so headless tests can advance it.
                cx.background_executor().timer(PRICE_RESPONSE_POLL).await;
                loop {
                    match this.update(cx, |shell, cx| shell.receive_price(cx)) {
                        Ok(Received::Landed) => {}
                        Ok(Received::Empty) => break,
                        Ok(Received::Gone) | Err(_) => return,
                    }
                }
                if this.update(cx, |shell, cx| shell.pump_prices(cx)).is_err() {
                    return;
                }
            }
        })
    }

    /// Runs the scheduled price check every [`PRICE_CHECK`] for as long as the returned task is
    /// held, whatever View is showing. Stops once the window has gone with the Shell, or the Open
    /// Ledger holding the task.
    pub(super) fn check_prices_every_minute(cx: &mut Context<'_, Self>) -> Task<()> {
        cx.spawn(async move |this, cx| {
            loop {
                // The executor's timer, not `Timer::after`, so headless tests can advance it.
                cx.background_executor().timer(PRICE_CHECK).await;
                if this.update(cx, |shell, _| shell.check_prices()).is_err() {
                    return;
                }
            }
        })
    }

    /// The Open Ledger's scheduled price check. It only sends requests, writing nothing, so it
    /// needs no input:
    /// each round's prices are written when its last response comes back through
    /// [`Self::receive_price`].
    fn check_prices(&mut self) {
        let Some(prices) = self.open.as_ref().and_then(|open| open.prices.clone()) else {
            return;
        };
        prices.borrow_mut().check();
    }

    /// Sends the queued price calls the Clock now allows. It writes nothing, so it needs no input:
    /// the *Waiting for…* Info Toast a wait raises as it begins is drained onto the stack, and the
    /// status line is left to the Shell (ADR-0048).
    fn pump_prices(&mut self, cx: &mut Context<'_, Self>) {
        let Some(prices) = self.open.as_ref().and_then(|open| open.prices.clone()) else {
            return;
        };
        let mut notices: Vec<lib_toast::Notice> = Vec::new();
        prices.borrow_mut().pump(&mut notices);
        if notices.is_empty() {
            return;
        }
        for notice in notices {
            lib_toast::Notify::notify(&mut crate::notices::Queue, notice);
        }
        self.drain_notices(cx);
        cx.notify();
    }

    /// Lands the next price response, if one waits, as one input; one not landed declines the
    /// input, so nothing commits. A fetched price is never undoable (#1371), so a landed input's
    /// writes are dropped from History before the input closes. With no Open Ledger prices left
    /// it answers [`Received::Gone`], so the poll stops.
    fn receive_price(&mut self, cx: &mut Context<'_, Self>) -> Received {
        let Some(open) = self.open.as_ref() else {
            return Received::Gone;
        };
        let Some(prices) = open.prices.clone() else {
            return Received::Gone;
        };
        crate::input::offer(cx, |_| {
            let received = prices.borrow_mut().land_next(&mut crate::notices::Queue);
            let landed = received == Received::Landed;
            if landed {
                let _ = open.ports.take_reversal();
            }
            (received, landed)
        })
    }
}
