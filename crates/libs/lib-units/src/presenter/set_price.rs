//! The Set price Dialog: a Manual entry Unit's close for one week, by hand. Week is a date, read
//! as the Locale's short form or ISO and snapped to its ISO week's Monday, which the Dialog shows
//! under it; it starts on today, so the current week is the default. Close is in the quote Unit and
//! is padded to its Precision when set. Focus starts on Close, since Week is already filled.

use chrono::{Local, NaiveDate};
use lib_core::{Money, UnitId};
use lib_locale::format::{
    AmountStyle, DateInputOptions, date_error_message, format_amount, format_date,
    format_date_input, parse_date_with, parse_positive_amount,
};
use lib_viper::view::{DialogFrame, TextFieldView};

use super::{Clock, UnitsOutcome, cycle};
use crate::interactor::SetPrice;
use crate::protocols::{SharedPorts, UnitsRouter};
use crate::{Error, MANUAL_ENTRY, Precision, WeeklyPrice, msg};

/// The form's fields, in `Tab` order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetPriceField {
    Week,
    Close,
}

impl SetPriceField {
    const ORDER: [Self; 2] = [Self::Week, Self::Close];
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetPriceEvent {
    /// A text field's whole new text, as its View's input holds it.
    Week(String),
    Close(String),
    /// `Tab`/`Shift-Tab`, wrapping.
    NextField,
    PrevField,
    /// A click on a field.
    Focus(SetPriceField),
    Confirm,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetPriceViewModel {
    pub frame: DialogFrame,
    /// Any day of the week, as typed.
    pub week: TextFieldView,
    /// The week it snaps to, `Week of 26 Oct 2026`, or why the date can't be read.
    pub week_help: String,
    pub close: TextFieldView,
    /// The quote Unit and its places: `In aud, to 2 places.`
    pub close_help: String,
    /// Why the last confirm was refused.
    pub refusal: Option<String>,
}

pub struct SetPricePresenter {
    ports: SharedPorts,
    clock: Clock,
    id: UnitId,
    code: String,
    /// The quote Unit's code and Precision, read when the Dialog opens; `None` when it is gone,
    /// which leaves the close as typed.
    quote: Option<(String, Precision)>,
    week: String,
    close: String,
    focused: SetPriceField,
    refusal: Option<String>,
}

impl SetPricePresenter {
    /// The Dialog for Unit `id`, or `None` when it is gone or not on Manual entry, which takes no
    /// price by hand.
    pub fn new(ports: SharedPorts, clock: Clock, id: UnitId) -> Option<Self> {
        let (code, quote) = {
            let ports = ports.borrow();
            let unit = ports.units().unit(id)?;
            if unit.link.source_id != MANUAL_ENTRY {
                return None;
            }
            let quote = ports
                .units()
                .unit(unit.link.quote)
                .map(|quote| (quote.code, quote.precision));
            (unit.code, quote)
        };
        let today = today(&clock);
        Some(Self {
            ports,
            clock,
            id,
            code,
            quote,
            week: format_date_input(today, None),
            close: String::new(),
            focused: SetPriceField::Close,
            refusal: None,
        })
    }

    pub fn handle(&mut self, event: SetPriceEvent, router: &mut dyn UnitsRouter) {
        match event {
            SetPriceEvent::Week(text) => self.week = text,
            SetPriceEvent::Close(text) => self.close = text,
            SetPriceEvent::NextField | SetPriceEvent::PrevField => {
                let backward = event == SetPriceEvent::PrevField;
                self.focused = cycle(&SetPriceField::ORDER, self.focused, backward, |_| true);
            }
            SetPriceEvent::Focus(field) => self.focused = field,
            SetPriceEvent::Confirm => self.confirm(router),
            SetPriceEvent::Cancel => router.close_dialog(None),
        }
    }

    pub fn view_model(&self) -> SetPriceViewModel {
        let text = |label, text: &str, placeholder, field| TextFieldView {
            label,
            text: text.to_string(),
            placeholder,
            focused: self.focused == field,
        };
        let week_help = match self.day() {
            Ok(day) => msg::units_set_price_week_of(&format_date(WeeklyPrice::week_of(day), None)),
            Err(error) => error,
        };
        let zero = Money(0.into());
        let (placeholder, close_help) = match &self.quote {
            Some((code, precision)) => (
                format_amount(&zero, AmountStyle::fixed(precision.places())).text,
                msg::units_set_price_close_help(code, i64::from(precision.get())),
            ),
            None => (String::new(), String::new()),
        };
        SetPriceViewModel {
            frame: DialogFrame {
                title: msg::units_set_price_title(&self.code),
                submit: msg::units_set_price_submit(),
                can_confirm: self.day().is_ok() && self.price().is_some(),
                destructive: false,
                hints: vec![super::next_field_hint(), super::cancel_hint()],
            },
            week: text(
                msg::units_set_price_field_week(),
                &self.week,
                format_date_input(today(&self.clock), None),
                SetPriceField::Week,
            ),
            week_help,
            close: text(
                msg::units_set_price_field_close(),
                &self.close,
                placeholder,
                SetPriceField::Close,
            ),
            close_help,
            refusal: self.refusal.clone(),
        }
    }

    /// The typed day, or the Message saying why it can't be read. A year is required, as on any
    /// entry form.
    fn day(&self) -> Result<NaiveDate, String> {
        let today = today(&self.clock);
        let options = DateInputOptions {
            style: None,
            allow_yearless: false,
        };
        parse_date_with(&self.week, today, &options)
            .map_err(|error| date_error_message(&error, today, &options))
    }

    /// The typed close, padded to the quote Unit's Precision; never rounded, so places finer than
    /// it are kept as typed. `None` until it is a positive amount.
    fn price(&self) -> Option<Money> {
        let mut close = parse_positive_amount(&self.close)?;
        if let Some((_, precision)) = &self.quote
            && close.0.fractional_digit_count() < i64::from(precision.get())
        {
            close = Money(close.0.with_scale(i64::from(precision.get())));
        }
        Some(close)
    }

    fn confirm(&mut self, router: &mut dyn UnitsRouter) {
        let (Ok(day), Some(close)) = (self.day(), self.price()) else {
            return;
        };
        let now = (self.clock)();
        let set = {
            let mut ports = self.ports.borrow_mut();
            SetPrice {
                id: self.id,
                day,
                close,
            }
            .run(ports.split_mut().units, today(&self.clock), now)
        };
        let week = WeeklyPrice::week_of(day);
        match set {
            Ok(unit) => {
                self.refusal = None;
                router.close_dialog(Some(UnitsOutcome::PriceSet {
                    code: unit.code,
                    week,
                }));
            }
            Err(refusal) => self.refusal = Some(self.refusal_text(refusal, week)),
        }
    }

    fn refusal_text(&self, refusal: Error, week: NaiveDate) -> String {
        match refusal {
            Error::FutureWeek => msg::units_refusal_future_week(&format_date(week, None)),
            Error::PricedBySource => msg::units_refusal_priced_by_source(&self.code),
            Error::UnitGone => msg::units_refusal_unit_gone(),
            refusal => {
                debug_assert!(false, "SetPrice doesn't return {refusal:?}");
                msg::units_refusal_unit_gone()
            }
        }
    }
}

/// The Clock's day where the person is, which decides the current week.
fn today(clock: &Clock) -> NaiveDate {
    clock().with_timezone(&Local).date_naive()
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{clock, hints, now, pairs, stub};
    use super::*;
    use crate::fake::{RecordingRouter, RouterCall};
    use crate::protocols::UnitRepository as _;

    fn send(dialog: &mut SetPricePresenter, events: Vec<SetPriceEvent>) -> Vec<RouterCall> {
        let mut router = RecordingRouter::default();
        for event in events {
            dialog.handle(event, &mut router);
        }
        router.calls
    }

    fn vas(shared: SharedPorts) -> SetPricePresenter {
        SetPricePresenter::new(shared, clock(), UnitId::SEED_VAS).expect("vas is on Manual entry")
    }

    fn day(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).expect("a real date")
    }

    #[test]
    fn it_opens_on_today_with_its_week_and_the_close_in_the_quote_units_places() {
        let (_, shared) = stub();
        let view = vas(shared).view_model();

        assert_eq!(view.frame.title, "Set price — vas");
        assert_eq!(view.frame.submit, "Set price");
        assert_eq!(view.week.text, format_date_input(today(&clock()), None));
        // The tests' clock reads Wednesday 21 October 2026, in the week starting Monday 19th.
        assert_eq!(view.week_help, "Week of Oct 19, 2026");
        assert_eq!(view.close.placeholder, "0.00");
        assert_eq!(view.close_help, "In aud, to 2 places.");
        assert!(view.close.focused && !view.week.focused);
        assert!(!view.frame.can_confirm, "no close yet");
    }

    #[test]
    fn a_unit_on_a_fetching_source_or_gone_opens_no_dialog() {
        let (_, shared) = stub();

        assert!(SetPricePresenter::new(shared.clone(), clock(), UnitId::SEED_BTC).is_none());
        assert!(SetPricePresenter::new(shared, clock(), UnitId::new()).is_none());
    }

    #[test]
    fn confirming_writes_this_weeks_price_padded_and_closes_with_the_week() {
        let (ports, shared) = stub();
        let mut dialog = vas(shared);

        let calls = send(
            &mut dialog,
            vec![SetPriceEvent::Close("101.2".into()), SetPriceEvent::Confirm],
        );

        assert_eq!(
            calls,
            [RouterCall::CloseDialog(Some(UnitsOutcome::PriceSet {
                code: "vas".into(),
                week: day(2026, 10, 19),
            }))]
        );
        let unit = ports.borrow().units.unit(UnitId::SEED_VAS).unwrap();
        let last = unit.last_price().expect("a price was set");
        assert_eq!(last.week, day(2026, 10, 19));
        assert_eq!(
            last.close.0.to_string(),
            "101.20",
            "padded to aud's 2 places"
        );
        assert_eq!(last.written_at, now());
    }

    #[test]
    fn a_past_week_typed_in_iso_snaps_to_its_monday() {
        let (ports, shared) = stub();
        let mut dialog = vas(shared);

        send(
            &mut dialog,
            vec![
                SetPriceEvent::Week("2026-09-10".into()),
                SetPriceEvent::Close("95".into()),
            ],
        );
        assert_eq!(dialog.view_model().week_help, "Week of Sep 7, 2026");
        send(&mut dialog, vec![SetPriceEvent::Confirm]);

        let prices = ports.borrow().units.unit(UnitId::SEED_VAS).unwrap().prices;
        assert_eq!(prices.len(), 1);
        assert_eq!(prices[0].week, day(2026, 9, 7));
    }

    #[test]
    fn a_future_week_is_refused_with_a_message_and_the_dialog_stays_open() {
        let (ports, shared) = stub();
        let mut dialog = vas(shared);

        let calls = send(
            &mut dialog,
            vec![
                SetPriceEvent::Week("2026-10-26".into()),
                SetPriceEvent::Close("100".into()),
                SetPriceEvent::Confirm,
            ],
        );

        assert_eq!(calls, []);
        assert_eq!(
            dialog.view_model().refusal.as_deref(),
            Some("The week of Oct 26, 2026 hasn't started, so it can't be priced yet.")
        );
        assert!(
            ports
                .borrow()
                .units
                .unit(UnitId::SEED_VAS)
                .unwrap()
                .prices
                .is_empty()
        );
    }

    #[test]
    fn an_unreadable_date_or_a_close_that_isnt_positive_cant_confirm() {
        let (_, shared) = stub();
        let mut dialog = vas(shared);

        send(
            &mut dialog,
            vec![
                SetPriceEvent::Week("someday".into()),
                SetPriceEvent::Close("100".into()),
            ],
        );
        let view = dialog.view_model();
        assert!(!view.frame.can_confirm);
        assert_ne!(view.week_help, "", "says why the date can't be read");
        assert!(!view.week_help.starts_with("Week of"));

        send(
            &mut dialog,
            vec![
                SetPriceEvent::Week("2026-10-20".into()),
                SetPriceEvent::Close("0".into()),
            ],
        );
        assert!(!dialog.view_model().frame.can_confirm);
        assert_eq!(send(&mut dialog, vec![SetPriceEvent::Confirm]), []);
    }

    #[test]
    fn a_unit_moved_to_a_fetching_source_under_the_dialog_is_refused_as_text() {
        let (ports, shared) = stub();
        let mut dialog = vas(shared);
        ports.borrow_mut().units.units[2].link.source_id = crate::YAHOO_FINANCE;

        send(
            &mut dialog,
            vec![SetPriceEvent::Close("100".into()), SetPriceEvent::Confirm],
        );

        assert_eq!(
            dialog.view_model().refusal.as_deref(),
            Some("vas is priced by its price source, so it takes no price by hand.")
        );
    }

    #[test]
    fn tab_moves_between_week_and_close_and_cancel_closes_with_nothing() {
        let (_, shared) = stub();
        let mut dialog = vas(shared);

        send(&mut dialog, vec![SetPriceEvent::NextField]);
        assert!(dialog.view_model().week.focused);
        send(&mut dialog, vec![SetPriceEvent::PrevField]);
        assert!(dialog.view_model().close.focused);
        send(&mut dialog, vec![SetPriceEvent::Focus(SetPriceField::Week)]);
        assert!(dialog.view_model().week.focused);

        assert_eq!(
            send(&mut dialog, vec![SetPriceEvent::Cancel]),
            [RouterCall::CloseDialog(None)]
        );
        assert_eq!(
            hints(&dialog.view_model().frame),
            pairs(&[("next_field", "next field"), ("back", "cancel")])
        );
    }
}
