//! The price-fetching concern of `Root`: the minute timer that reaches the Units' scheduled check,
//! and polling the Open Ledger's price feed and sending each response as one input, so the round it
//! completes is that input's one commit and the Units repaint on its `LedgerChanged` (ADR-0062).
//! The same poll pumps the calls waiting their source's call rate or `Retry-After` out. An
//! `impl Root` block.

use std::sync::mpsc;
use std::time::Duration;

use gpui::{Context, Task};
use lib_units::PriceResponse;

use super::Root;

/// How often the Shell looks for a price response. The feed answers on a blocking channel, so the
/// Shell polls it rather than holding a thread on it. Public so the headless tests advance past
/// exactly this.
#[doc(hidden)]
pub const PRICE_RESPONSE_POLL: Duration = Duration::from_millis(250);

/// How often the Open Ledger checks for Units due a price, after the check it makes on opening
/// (ADR-0043's "where and when fetching runs"). Public so the headless tests advance past exactly
/// this.
#[doc(hidden)]
pub const PRICE_CHECK: Duration = Duration::from_secs(60);

impl Root {
    /// Polls `responses` for as long as the returned task is held, sending each as an input, then
    /// pumps the queued calls out. Stops once the feed has gone, the window with the Shell, or the
    /// Open Ledger holding the task.
    pub(super) fn listen_for_price_responses(
        responses: mpsc::Receiver<PriceResponse>,
        cx: &mut Context<'_, Self>,
    ) -> Task<()> {
        cx.spawn(async move |this, cx| {
            loop {
                // The executor's timer, not `Timer::after`, so headless tests can advance it.
                cx.background_executor().timer(PRICE_RESPONSE_POLL).await;
                loop {
                    let response = match responses.try_recv() {
                        Ok(response) => response,
                        Err(mpsc::TryRecvError::Empty) => break,
                        Err(mpsc::TryRecvError::Disconnected) => return,
                    };
                    if this
                        .update(cx, |shell, cx| shell.receive_price(response, cx))
                        .is_err()
                    {
                        return;
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

    /// The Units' scheduled check. It only sends requests, writing nothing, so it needs no input:
    /// each round's prices are written when its last response comes back through
    /// [`Self::receive_price`].
    fn check_prices(&mut self) {
        let Some(presenter) = self
            .open
            .as_ref()
            .and_then(|open| open.price_responses.clone())
        else {
            return;
        };
        presenter.borrow_mut().fetch_due();
    }

    /// Sends the queued price calls the Clock now allows. It writes nothing, so it needs no input:
    /// only a change to the *Waiting for…* status is shown, straight onto the status line.
    fn pump_prices(&mut self, cx: &mut Context<'_, Self>) {
        let Some(presenter) = self
            .open
            .as_ref()
            .and_then(|open| open.price_responses.clone())
        else {
            return;
        };
        let mut notices: Vec<lib_toast::Notice> = Vec::new();
        presenter.borrow_mut().pump(&mut notices);
        if notices.is_empty() {
            return;
        }
        for notice in notices {
            lib_toast::Notify::notify(&mut crate::notices::Queue, notice);
        }
        self.drain_notices(cx);
        cx.notify();
    }

    /// Hands `response` to the Units' Presenter as one input. A fetched price is never undoable
    /// (#1371), so the input's writes are dropped from History before the input closes.
    fn receive_price(&mut self, response: PriceResponse, cx: &mut Context<'_, Self>) {
        let Some(open) = self.open.as_ref() else {
            return;
        };
        let Some(presenter) = open.price_responses.clone() else {
            return;
        };
        crate::input::send(cx, |_| {
            presenter
                .borrow_mut()
                .handle(response, &mut crate::notices::Queue);
            let _ = open.ports.take_reversal();
        });
    }
}
