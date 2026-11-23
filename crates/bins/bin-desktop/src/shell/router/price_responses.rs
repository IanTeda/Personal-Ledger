//! The price-response concern of `Root`: polling the Open Ledger's price feed and sending each
//! response as one input, so the price it writes is that input's one commit and the Units repaint
//! on its `LedgerChanged` (ADR-0062). An `impl Root` block.

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

impl Root {
    /// Polls `responses` for as long as the returned task is held, sending each as an input. Stops
    /// once the feed has gone, the window with the Shell, or the Open Ledger holding the task.
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
            }
        })
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
