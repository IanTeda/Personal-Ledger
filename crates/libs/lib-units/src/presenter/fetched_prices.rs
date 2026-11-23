//! Where a price response lands, and what the Client's minute timer reaches: the Client hands each
//! response here through its input door, so the round it completes is written as that input's one
//! commit and every View reading the Units repaints. It draws nothing; a Fetch now that wrote a
//! price says so in a Success Toast, and a scheduled round says nothing at all.

use chrono::Local;
use lib_toast::{Notice, Notify, ToastKind};

use super::Clock;
use super::units_pane::amount;
use crate::fetching::{PriceResponse, SharedFetching};
use crate::interactor::{FetchDue, RecordFetchedPrices};
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

    /// The scheduled check, on Ledger open and every minute after: starts a silent round on each
    /// source with a Unit due, leaving a source with a round in flight alone.
    pub fn fetch_due(&mut self) {
        let now = (self.clock)();
        FetchDue {
            today: now.with_timezone(&Local).date_naive(),
            now,
        }
        .run(&*self.ports.borrow(), &mut self.fetching.borrow_mut());
    }

    /// Holds `response` until the rest of its round lands, then writes every price the round
    /// carries for a Unit that still wants it. A round a person asked for raises a Toast naming
    /// each price; a scheduled one raises nothing. A fetched price is never undoable, so it raises
    /// no `Notice::Changed`.
    pub fn handle(&mut self, response: PriceResponse, router: &mut dyn Notify) {
        let Some(landed) = self.fetching.borrow_mut().land(response) else {
            return;
        };
        let now = (self.clock)();
        let written = {
            let mut ports = self.ports.borrow_mut();
            RecordFetchedPrices {
                responses: landed.responses,
            }
            .run(
                ports.split_mut().units,
                &mut self.fetching.borrow_mut(),
                now.with_timezone(&Local).date_naive(),
                now,
            )
        };
        if !landed.asked {
            return;
        }
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

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use chrono::{DateTime, Utc};
    use lib_core::{UnitId, UnitKind};
    use lib_toast::Notice;

    use super::*;
    use crate::fake::{FakeTransport, InMemoryPorts};
    use crate::fetching::PriceFetching;
    use crate::protocols::SharedPorts;
    use crate::{FRANKFURTER, Precision, PriceLink, UnitRow, WeeklyPrice};

    fn now() -> DateTime<Utc> {
        chrono::NaiveDate::from_ymd_opt(2026, 10, 21)
            .and_then(|day| day.and_hms_opt(9, 0, 0))
            .expect("a real time")
            .and_utc()
    }

    fn currency(code: &str) -> UnitRow {
        UnitRow::new(
            code.into(),
            code.into(),
            UnitKind::Currency,
            Precision::of_kind(UnitKind::Currency),
            PriceLink {
                source_id: FRANKFURTER,
                symbol: code.into(),
                quote: UnitId::SEED_AUD,
            },
        )
    }

    #[test]
    fn a_scheduled_round_is_written_once_its_last_response_lands_and_tells_no_one() {
        let mut sample = InMemoryPorts::sample(now());
        sample
            .units
            .units
            .extend([currency("EUR"), currency("USD")]);
        let ports: SharedPorts = Rc::new(RefCell::new(sample));
        let transport = FakeTransport::default();
        let fetching = PriceFetching::shared(Rc::new(transport.clone()));
        let mut presenter =
            FetchedPricesPresenter::new(ports.clone(), fetching.clone(), Rc::new(now));
        let mut notices: Vec<Notice> = Vec::new();
        let priced = |code: &str| {
            let ports = ports.borrow();
            let unit = ports
                .units()
                .units()
                .into_iter()
                .find(|unit| unit.code == code)
                .expect("listed");
            unit.prices.iter().any(|price| {
                price.week == WeeklyPrice::week_of(now().with_timezone(&Local).date_naive())
            })
        };

        presenter.fetch_due();
        let requests = transport.requests();
        assert_eq!(requests.len(), 2, "eur and usd, one Frankfurter round");

        let eur = transport.answer(
            requests[0].0,
            crate::TransportOutcome::Answered {
                status: 200,
                body: r#"{"rate":1.65}"#.into(),
            },
        );
        presenter.handle(eur, &mut notices);
        assert!(!priced("EUR"), "held until the round's last response lands");

        let usd = transport.answer_last(200, r#"{"rate":1.52}"#);
        presenter.handle(usd, &mut notices);

        assert!(
            priced("EUR") && priced("USD"),
            "the whole round is written at once"
        );
        assert!(
            notices.is_empty(),
            "a scheduled round raises no Toast and no status"
        );
        assert_eq!(fetching.borrow().in_flight(), 0);
    }
}
