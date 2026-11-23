//! Where a price response lands: the Client hands each one here through its input door, so the
//! prices it writes are that input's one commit and every View reading the Units repaints. It
//! draws nothing; a Fetch now that wrote a price says so in a Success Toast.

use chrono::Local;
use lib_toast::{Notice, Notify, ToastKind};

use super::Clock;
use super::units_pane::amount;
use crate::fetching::{PriceResponse, SharedFetching};
use crate::interactor::RecordFetchedPrices;
use crate::msg;
use crate::protocols::SharedPorts;

pub struct FetchedPricesPresenter {
    ports: SharedPorts,
    fetching: SharedFetching,
    clock: Clock,
}

impl FetchedPricesPresenter {
    pub fn new(ports: SharedPorts, fetching: SharedFetching, clock: Clock) -> Self {
        Self {
            ports,
            fetching,
            clock,
        }
    }

    /// Writes the price `response` carries, if its Unit still wants it, with a Toast naming it.
    /// A fetched price is never undoable, so it raises no `Notice::Changed`.
    pub fn handle(&mut self, response: PriceResponse, router: &mut dyn Notify) {
        let now = (self.clock)();
        let written = {
            let mut ports = self.ports.borrow_mut();
            RecordFetchedPrices {
                responses: vec![response],
            }
            .run(
                ports.split_mut().units,
                &mut self.fetching.borrow_mut(),
                now.with_timezone(&Local).date_naive(),
                now,
            )
        };
        let ports = self.ports.borrow();
        for price in written {
            let Some(unit) = ports.units().unit(price.unit) else {
                continue;
            };
            let quote = ports.units().unit(unit.link.quote);
            router.notify(Notice::Toast(
                ToastKind::Success,
                msg::units_toast_price_fetched(&unit.code, &amount(&price.close, quote.as_ref())),
            ));
        }
    }
}
