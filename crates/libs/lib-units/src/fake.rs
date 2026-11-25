//! In-memory ports and the sample seed, for tests and UI-first work. Each holds its rows in a plain
//! field so a test can set up or read state directly. [`RecordingRouter`] is a Router that only
//! records what it was asked. [`FakeTransport`] records each price request and answers it on cue,
//! and [`LedgerPricesFixture`] is where every price-fetching test starts.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::mpsc::{Receiver, Sender};

use chrono::{DateTime, Duration, NaiveDate, Utc};
use lib_core::{Clock, PriceSourceId, UnitId};
use lib_toast::{Notice, Notify};
use lib_viper::dialog::DialogRouter;
use secrecy::{ExposeSecret as _, SecretString};

use crate::fetching::{
    PriceRequest, PriceResponse, PriceTransport, RequestId, SharedFetching, TransportOutcome,
};
use crate::presenter::{LedgerPrices, Received};
use crate::protocols::{PortsMut, SharedPorts, UnitsDialog, UnitsOutcome, UnitsPorts, UnitsRouter};
use crate::{
    BaseUnitRepository, Error, Holding, KeyState, PriceSourceKeys, PriceSourceRecord,
    PriceSourceRepository, Result, UnitHoldings, UnitOpenings, UnitRepository, UnitRow, UnitUsage,
    UnitUse, sample_units,
};
#[cfg(test)]
use crate::{
    BuiltInOverride, FRANKFURTER, FetchFailure, LastFetch, Precision, PriceLink, UnitKind, Updates,
    interactor::{FetchNow, FetchSource},
};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InMemoryUnits {
    pub units: Vec<UnitRow>,
}

impl UnitRepository for InMemoryUnits {
    fn units(&self) -> Vec<UnitRow> {
        self.units.clone()
    }

    fn unit(&self, id: UnitId) -> Option<UnitRow> {
        self.units.iter().find(|unit| unit.id == id).cloned()
    }

    fn upsert_unit(&mut self, unit: UnitRow) {
        match self.units.iter_mut().find(|row| row.id == unit.id) {
            Some(row) => *row = unit,
            None => self.units.push(unit),
        }
    }

    fn delete_unit(&mut self, id: UnitId) {
        self.units.retain(|row| row.id != id);
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InMemoryPriceSources {
    pub records: Vec<PriceSourceRecord>,
}

impl PriceSourceRepository for InMemoryPriceSources {
    fn price_source_records(&self) -> Vec<PriceSourceRecord> {
        self.records.clone()
    }

    fn upsert_price_source(&mut self, record: PriceSourceRecord) {
        match self.records.iter_mut().find(|row| row.id() == record.id()) {
            Some(row) => *row = record,
            None => self.records.push(record),
        }
    }

    fn delete_price_source(&mut self, id: PriceSourceId) {
        self.records.retain(|row| row.id() != id);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InMemoryBaseUnit(pub UnitId);

impl BaseUnitRepository for InMemoryBaseUnit {
    fn base_unit(&self) -> UnitId {
        self.0
    }

    fn set_base_unit(&mut self, id: UnitId) {
        self.0 = id;
    }
}

/// Usage counts a test sets per Unit; a Unit not listed is unused.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InMemoryUnitUsage {
    pub uses: HashMap<UnitId, UnitUse>,
}

impl UnitUsage for InMemoryUnitUsage {
    fn unit_usage(&self, id: UnitId) -> UnitUse {
        self.uses.get(&id).copied().unwrap_or_default()
    }
}

/// Holdings a test sets per Unit; a Unit not listed is held by no Account.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InMemoryHoldings {
    pub by_unit: HashMap<UnitId, Vec<Holding>>,
}

impl UnitHoldings for InMemoryHoldings {
    fn holdings(&self, id: UnitId) -> Vec<Holding> {
        self.by_unit.get(&id).cloned().unwrap_or_default()
    }
}

/// The earliest Opened date a test sets per Unit; a Unit not listed is held by no Account.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InMemoryOpenings {
    pub by_unit: HashMap<UnitId, NaiveDate>,
}

impl UnitOpenings for InMemoryOpenings {
    fn earliest_opened(&self, id: UnitId) -> Option<NaiveDate> {
        self.by_unit.get(&id).copied()
    }
}

/// One key as [`InMemoryKeys`] holds it.
#[derive(Clone)]
pub enum InMemoryKey {
    Saved(SecretString),
    /// Saved with another secret, so it reads as no key.
    Unreadable,
}

/// The Price Sources' API keys in plain memory: the adapter's encryption is `lib-database`'s, so
/// a test sets what this device can read directly. `locked` stands for a Ledger whose keys another
/// device's secret wrote, `unsaved` for a secret that couldn't be kept.
#[derive(Clone, Default)]
pub struct InMemoryKeys {
    pub keys: HashMap<PriceSourceId, InMemoryKey>,
    pub locked: bool,
    pub unsaved: bool,
}

impl InMemoryKeys {
    /// The keys this device can read, exposed for a test's assertions.
    pub fn exposed(&self) -> HashMap<PriceSourceId, String> {
        self.keys
            .iter()
            .filter_map(|(id, key)| match key {
                InMemoryKey::Saved(key) => Some((*id, key.expose_secret().to_string())),
                InMemoryKey::Unreadable => None,
            })
            .collect()
    }
}

/// Only which sources hold a key, never a key.
impl std::fmt::Debug for InMemoryKeys {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut sources: Vec<_> = self.keys.keys().collect();
        sources.sort();
        f.debug_struct("InMemoryKeys")
            .field("sources", &sources)
            .field("locked", &self.locked)
            .finish_non_exhaustive()
    }
}

/// Equal when the same sources hold the same keys: a fake's, compared only by tests.
impl PartialEq for InMemoryKeys {
    fn eq(&self, other: &Self) -> bool {
        let unreadable = |keys: &Self| {
            let mut ids: Vec<_> = keys
                .keys
                .iter()
                .filter(|(_, key)| matches!(key, InMemoryKey::Unreadable))
                .map(|(id, _)| *id)
                .collect();
            ids.sort();
            ids
        };
        self.locked == other.locked
            && self.unsaved == other.unsaved
            && self.exposed() == other.exposed()
            && unreadable(self) == unreadable(other)
    }
}

impl Eq for InMemoryKeys {}

impl PriceSourceKeys for InMemoryKeys {
    fn key(&self, source: PriceSourceId) -> Option<SecretString> {
        match self.keys.get(&source)? {
            InMemoryKey::Saved(key) => Some(key.clone()),
            InMemoryKey::Unreadable => None,
        }
    }

    fn key_state(&self, source: PriceSourceId) -> KeyState {
        match self.keys.get(&source) {
            None => KeyState::NotSet,
            Some(InMemoryKey::Saved(_)) => KeyState::Saved,
            Some(InMemoryKey::Unreadable) => KeyState::Unreadable,
        }
    }

    fn can_write(&self) -> bool {
        !self.locked
    }

    fn store_key(&mut self, source: PriceSourceId, key: SecretString) -> Result<()> {
        if self.locked {
            return Err(Error::KeysLocked);
        }
        if self.unsaved {
            return Err(Error::KeyNotSaved);
        }
        self.keys.insert(source, InMemoryKey::Saved(key));
        Ok(())
    }

    fn remove_key(&mut self, source: PriceSourceId) {
        self.keys.remove(&source);
    }
}

/// Every Units port in memory, together, so a test can hand an Interactor the ones it takes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InMemoryPorts {
    pub units: InMemoryUnits,
    pub price_sources: InMemoryPriceSources,
    pub base_unit: InMemoryBaseUnit,
    pub usage: InMemoryUnitUsage,
    pub holdings: InMemoryHoldings,
    pub openings: InMemoryOpenings,
    pub keys: InMemoryKeys,
}

impl InMemoryPorts {
    /// The sample seed: the Sample Ledger's Units (see [`sample_units`]) with aud as the base, no
    /// stored Price Source rows, and nothing using any Unit. Ids match `lib-database`'s joined Sample Ledger, so its
    /// Accounts, Bill Plans and Budgets still resolve their Units.
    pub fn sample(now: DateTime<Utc>) -> Self {
        Self {
            units: InMemoryUnits {
                units: sample_units(now),
            },
            price_sources: InMemoryPriceSources::default(),
            base_unit: InMemoryBaseUnit(UnitId::SEED_AUD),
            usage: InMemoryUnitUsage::default(),
            holdings: InMemoryHoldings::default(),
            openings: InMemoryOpenings::default(),
            keys: InMemoryKeys::default(),
        }
    }
}

impl InMemoryPorts {
    /// These ports as the Presenters share them, with the handle a test keeps to read them back.
    pub fn shared(self) -> (Rc<RefCell<Self>>, SharedPorts) {
        let ports = Rc::new(RefCell::new(self));
        let shared: SharedPorts = ports.clone();
        (ports, shared)
    }
}

impl UnitsPorts for InMemoryPorts {
    fn units(&self) -> &dyn UnitRepository {
        &self.units
    }

    fn price_sources(&self) -> &dyn PriceSourceRepository {
        &self.price_sources
    }

    fn base_unit(&self) -> &dyn BaseUnitRepository {
        &self.base_unit
    }

    fn usage(&self) -> &dyn UnitUsage {
        &self.usage
    }

    fn holdings(&self) -> &dyn UnitHoldings {
        &self.holdings
    }

    fn openings(&self) -> &dyn UnitOpenings {
        &self.openings
    }

    fn keys(&self) -> &dyn PriceSourceKeys {
        &self.keys
    }

    fn keys_mut(&mut self) -> &mut dyn PriceSourceKeys {
        &mut self.keys
    }

    fn split_mut(&mut self) -> PortsMut<'_> {
        PortsMut {
            units: &mut self.units,
            price_sources: &mut self.price_sources,
            base_unit: &mut self.base_unit,
            usage: &self.usage,
        }
    }
}

/// One request a Presenter made of its Router.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouterCall {
    OpenDialog(UnitsDialog),
    CloseDialog(Option<UnitsOutcome>),
    Notice(Notice),
}

/// A Router that does nothing but record each call, in order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecordingRouter {
    pub calls: Vec<RouterCall>,
}

impl RecordingRouter {
    /// The calls made since the last `take`.
    pub fn take(&mut self) -> Vec<RouterCall> {
        std::mem::take(&mut self.calls)
    }
}

impl UnitsRouter for RecordingRouter {
    /// Always opens: the recording Router builds no Presenter that could fail.
    fn open_dialog(&mut self, dialog: UnitsDialog) -> bool {
        self.calls.push(RouterCall::OpenDialog(dialog));
        true
    }
}

impl DialogRouter<UnitsOutcome> for RecordingRouter {
    fn close(&mut self, outcome: Option<UnitsOutcome>) {
        self.calls.push(RouterCall::CloseDialog(outcome));
    }
}

impl Notify for RecordingRouter {
    fn notify(&mut self, notice: Notice) {
        self.calls.push(RouterCall::Notice(notice));
    }
}

/// A price transport that sends nothing: it records each request and answers one when a test says,
/// returning the response and, when built by [`FakeTransport::channel`], sending it on as a
/// Client's adapter would. Clones share the record.
#[derive(Debug, Clone, Default)]
pub struct FakeTransport {
    sent: Rc<RefCell<Vec<(RequestId, PriceRequest)>>>,
    responses: Option<Sender<PriceResponse>>,
}

impl FakeTransport {
    /// A transport whose answers also arrive on the returned channel, as `lib-price-feed`'s do.
    pub fn channel() -> (Self, Receiver<PriceResponse>) {
        let (sender, receiver) = std::sync::mpsc::channel();
        let transport = Self {
            sent: Rc::default(),
            responses: Some(sender),
        };
        (transport, receiver)
    }

    /// Every request sent, oldest first.
    pub fn requests(&self) -> Vec<(RequestId, PriceRequest)> {
        self.sent.borrow().clone()
    }

    /// Every request's URL, oldest first.
    pub fn urls(&self) -> Vec<String> {
        self.sent
            .borrow()
            .iter()
            .map(|(_, request)| request.url.clone())
            .collect()
    }

    /// Answers request `request` with `outcome`.
    pub fn answer(&self, request: RequestId, outcome: TransportOutcome) -> PriceResponse {
        let response = PriceResponse { request, outcome };
        if let Some(responses) = &self.responses {
            // A test that dropped the receiver reads the response returned instead.
            let _ = responses.send(response.clone());
        }
        response
    }

    /// Answers the newest request with `status` and `body`.
    ///
    /// # Panics
    /// With no request sent, which is a test's mistake.
    pub fn answer_last(&self, status: u16, body: &str) -> PriceResponse {
        self.answer_last_with(status, body, None)
    }

    /// Answers the newest request with `status`, `body` and the raw `Retry-After` header
    /// `retry_after`, as a throttling provider does.
    ///
    /// # Panics
    /// With no request sent, which is a test's mistake.
    #[expect(
        clippy::panic,
        reason = "test support: answering nothing is a broken test"
    )]
    pub fn answer_last_with(
        &self,
        status: u16,
        body: &str,
        retry_after: Option<&str>,
    ) -> PriceResponse {
        let (id, _) = self
            .sent
            .borrow()
            .last()
            .cloned()
            .unwrap_or_else(|| panic!("a request to answer"));
        self.answer(
            id,
            TransportOutcome::Answered {
                status,
                body: body.to_string(),
                retry_after: retry_after.map(str::to_string),
            },
        )
    }
}

impl PriceTransport for FakeTransport {
    fn send(&self, id: RequestId, request: PriceRequest) {
        self.sent.borrow_mut().push((id, request));
    }
}

/// An Open Ledger's prices as a test drives it: the fake Units ports, a [`FakeTransport`], a clock
/// whose day and time the test sets, and the [`LedgerPrices`] every response lands through, on a
/// channel the fixture feeds, collecting the Notices each landing raises. It is built before the
/// open check, so a test arranges the Ledger first and runs [`LedgerPrices`]' open check itself.
pub struct LedgerPricesFixture {
    ports: Rc<RefCell<InMemoryPorts>>,
    shared: SharedPorts,
    transport: FakeTransport,
    /// The feed's sending end, `None` once [`Self::hang_up`] drops it.
    feed: Option<Sender<PriceResponse>>,
    at: Rc<Cell<DateTime<Utc>>>,
    day: Rc<Cell<NaiveDate>>,
    clock: Clock,
    prices: LedgerPrices,
    notices: Vec<Notice>,
}

impl LedgerPricesFixture {
    /// `ports` with fetching over a fresh [`FakeTransport`], the clock reading `now` and its day
    /// in UTC.
    pub fn new(ports: InMemoryPorts, now: DateTime<Utc>) -> Self {
        let (ports, shared) = ports.shared();
        let transport = FakeTransport::default();
        let (feed, responses) = std::sync::mpsc::channel();
        let at = Rc::new(Cell::new(now));
        let day = Rc::new(Cell::new(now.date_naive()));
        let clock = {
            let at = at.clone();
            let day = day.clone();
            Clock::with_time(move || day.get(), move || at.get())
        };
        let prices = LedgerPrices::unopened(
            shared.clone(),
            Rc::new(transport.clone()),
            responses,
            clock.clone(),
        );
        Self {
            prices,
            ports,
            shared,
            transport,
            feed: Some(feed),
            at,
            day,
            clock,
            notices: Vec::new(),
        }
    }

    /// The ports, for a test to arrange or read back.
    pub fn ports(&self) -> &Rc<RefCell<InMemoryPorts>> {
        &self.ports
    }

    /// The same ports as the Presenters share them.
    pub fn shared(&self) -> SharedPorts {
        self.shared.clone()
    }

    pub fn transport(&self) -> &FakeTransport {
        &self.transport
    }

    /// The fetching every Presenter on this Ledger shares.
    pub fn fetching(&self) -> &SharedFetching {
        self.prices.shared_fetching()
    }

    /// A clock reading what [`Self::later`] and [`Self::set_today`] last set.
    pub fn clock(&self) -> Clock {
        self.clock.clone()
    }

    pub fn now(&self) -> DateTime<Utc> {
        self.at.get()
    }

    /// The day the clock reads.
    pub fn today(&self) -> NaiveDate {
        self.day.get()
    }

    /// Moves the clock on `by`, its day by as many days as that moves now's UTC date.
    pub fn later(&self, by: Duration) {
        let before = self.at.get();
        let after = before + by;
        self.at.set(after);
        self.day
            .set(self.day.get() + (after.date_naive() - before.date_naive()));
    }

    /// Sets the clock's day alone, leaving now where it is, as a Client east of UTC reads a day
    /// ahead of now's UTC date.
    pub fn set_today(&self, day: NaiveDate) {
        self.day.set(day);
    }

    /// The Ledger's prices, for its scheduled check and backfill.
    pub fn prices(&mut self) -> &mut LedgerPrices {
        &mut self.prices
    }

    /// Puts `response` on the feed and lands it as a Client's input door does, returning the
    /// Notices it raised; they are collected in [`Self::notices`] too.
    ///
    /// # Panics
    /// After [`Self::hang_up`], which is a test's mistake.
    #[expect(
        clippy::panic,
        reason = "test support: landing on a hung-up feed is a broken test"
    )]
    pub fn land(&mut self, response: PriceResponse) -> Vec<Notice> {
        let Some(feed) = &self.feed else {
            panic!("the feed was hung up");
        };
        // The receiver lives in `prices`, held beside the sender.
        let _ = feed.send(response);
        let (received, raised) = self.land_next();
        assert_eq!(received, Received::Landed, "the response just sent");
        raised
    }

    /// One [`LedgerPrices::land_next`], as a Client's poll makes it, returning what it found and
    /// the Notices it raised; they are collected in [`Self::notices`] too.
    pub fn land_next(&mut self) -> (Received, Vec<Notice>) {
        let mut raised = Vec::new();
        let received = self.prices.land_next(&mut raised);
        self.notices.extend(raised.iter().cloned());
        (received, raised)
    }

    /// Drops the feed's sending end, as a Ledger's feed goes when it closes.
    pub fn hang_up(&mut self) {
        self.feed = None;
    }

    /// Releases the queued calls as a Client's response poll does, returning the Notices it
    /// raised; they are collected in [`Self::notices`] too.
    pub fn pump(&mut self) -> Vec<Notice> {
        let mut raised = Vec::new();
        self.prices.pump(&mut raised);
        self.notices.extend(raised.iter().cloned());
        raised
    }

    /// Every Notice raised since the fixture was built or last emptied.
    pub fn notices(&self) -> &[Notice] {
        &self.notices
    }

    /// Empties the collected Notices, returning them.
    pub fn take_notices(&mut self) -> Vec<Notice> {
        std::mem::take(&mut self.notices)
    }
}

/// What the price-fetching tests ask of the fixture beyond a Client's interface, kept here once
/// rather than in each test module.
#[cfg(test)]
impl LedgerPricesFixture {
    /// The sample with `units` added, the clock reading `now`.
    pub(crate) fn with_units(units: Vec<UnitRow>, now: DateTime<Utc>) -> Self {
        let mut sample = InMemoryPorts::sample(now);
        sample.units.units.extend(units);
        Self::new(sample, now)
    }

    pub(crate) fn unit(&self, code: &str) -> UnitRow {
        self.ports
            .borrow()
            .units
            .units
            .iter()
            .find(|unit| unit.code == code)
            .cloned()
            .expect("listed")
    }

    pub(crate) fn edit_unit(&self, code: &str, edit: impl FnOnce(&mut UnitRow)) {
        edit(
            self.ports
                .borrow_mut()
                .units
                .units
                .iter_mut()
                .find(|unit| unit.code == code)
                .expect("listed"),
        );
    }

    /// Accounts in the Unit `code`, the earliest opened on `on`.
    pub(crate) fn opened(&self, code: &str, on: NaiveDate) {
        let id = self.unit(code).id;
        self.ports.borrow_mut().openings.by_unit.insert(id, on);
    }

    /// The weeks `code` has a Weekly Price for.
    pub(crate) fn weeks(&self, code: &str) -> Vec<NaiveDate> {
        self.unit(code)
            .prices
            .iter()
            .map(|price| price.week)
            .collect()
    }

    pub(crate) fn urls(&self) -> Vec<String> {
        self.transport.urls()
    }

    pub(crate) fn sent(&self) -> usize {
        self.transport.requests().len()
    }

    /// Overrides Frankfurter's Updates with `updates`.
    pub(crate) fn retime(&self, updates: Updates) {
        self.ports
            .borrow_mut()
            .price_sources
            .records
            .push(PriceSourceRecord::BuiltIn(BuiltInOverride {
                id: FRANKFURTER,
                name: "Frankfurter".into(),
                updates,
            }));
    }

    /// The scheduled check, as the Client's minute timer runs it; returns how many requests it
    /// sent to Frankfurter (the sample's btc comes due on CoinGecko too).
    pub(crate) fn check(&mut self) -> usize {
        let frankfurter = |fixture: &Self| {
            fixture
                .urls()
                .iter()
                .filter(|url| url.contains("frankfurter"))
                .count()
        };
        let before = frankfurter(self);
        self.prices.check();
        frankfurter(self) - before
    }

    pub(crate) fn fetch_now(&self, code: &str) {
        let id = self.unit(code).id;
        FetchNow { id }
            .run(
                &*self.ports.borrow(),
                &mut self.fetching().borrow_mut(),
                self.now(),
            )
            .expect("sent");
    }

    pub(crate) fn fetch_source(&self, id: PriceSourceId) -> Result<usize> {
        FetchSource { id }.run(
            &*self.ports.borrow(),
            &mut self.fetching().borrow_mut(),
            self.now(),
        )
    }

    pub(crate) fn unit_failure(&self, code: &str) -> Option<FetchFailure> {
        let unit = self.unit(code);
        let ports = self.ports.borrow();
        let catalogue = ports.source_catalogue();
        let source = catalogue.source(unit.link.source_id)?;
        self.fetching()
            .borrow()
            .unit_fetch(&unit, &source)
            .and_then(LastFetch::failure)
    }

    pub(crate) fn source_failure(&self, id: PriceSourceId) -> Option<FetchFailure> {
        let ports = self.ports.borrow();
        let catalogue = ports.source_catalogue();
        let source = catalogue.source(id)?;
        self.fetching()
            .borrow()
            .source_fetch(&source)
            .and_then(LastFetch::failure)
    }

    /// Answers every round's request still out, oldest first, each with `outcome(url)`, without
    /// landing them.
    pub(crate) fn respond(&self, outcome: impl Fn(&str) -> TransportOutcome) -> Vec<PriceResponse> {
        self.transport
            .requests()
            .into_iter()
            .filter(|(id, _)| self.fetching().borrow().in_flight_request(*id))
            .map(|(id, request)| self.transport.answer(id, outcome(&request.url)))
            .collect()
    }

    /// Answers the newest request with `outcome`, without landing it.
    pub(crate) fn respond_last(&self, outcome: TransportOutcome) -> PriceResponse {
        let (id, _) = self.transport.requests().pop().expect("a request out");
        self.transport.answer(id, outcome)
    }

    /// Answers and lands every round's request still out, oldest first, each with `outcome(url)`.
    pub(crate) fn answer(&mut self, outcome: impl Fn(&str) -> TransportOutcome) {
        for response in self.respond(outcome) {
            self.land(response);
        }
    }

    /// Answers and lands the newest request with `outcome`.
    pub(crate) fn answer_last(&mut self, outcome: TransportOutcome) {
        let response = self.respond_last(outcome);
        self.land(response);
    }
}

/// A currency on Frankfurter, quoted in aud.
#[cfg(test)]
pub(crate) fn currency(code: &str) -> UnitRow {
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

/// A provider's answer: `status`, `body` and the raw `Retry-After` header `retry_after`.
#[cfg(test)]
pub(crate) fn answered(status: u16, body: &str, retry_after: Option<&str>) -> TransportOutcome {
    TransportOutcome::Answered {
        status,
        body: body.into(),
        retry_after: retry_after.map(str::to_string),
    }
}

/// Offline, whatever was asked.
#[cfg(test)]
pub(crate) fn offline(_: &str) -> TransportOutcome {
    TransportOutcome::Failed(crate::TransportError::Offline)
}

/// A rate of 1.5, whatever was asked.
#[cfg(test)]
pub(crate) fn priced(_: &str) -> TransportOutcome {
    answered(200, r#"{"rate":1.5}"#, None)
}

/// A 429, waiting `retry_after` when it says.
#[cfg(test)]
pub(crate) fn throttled(retry_after: Option<&str>) -> TransportOutcome {
    answered(429, "", retry_after)
}

/// A 404 for `code`'s request, a price for every other.
#[cfg(test)]
pub(crate) fn missing(code: &'static str) -> impl Fn(&str) -> TransportOutcome {
    move |url| {
        if url.contains(&format!("/{code}/")) {
            answered(404, r#"{"message":"not found"}"#, None)
        } else {
            priced(url)
        }
    }
}

/// Frankfurter's history for EUR in AUD over three past weeks.
#[cfg(test)]
pub(crate) fn rates() -> TransportOutcome {
    answered(
        200,
        include_str!("../fixtures/history/frankfurter_rates.json"),
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::COINGECKO;

    #[test]
    fn the_sample_seed_holds_aud_btc_and_vas_with_aud_as_base() {
        let ports = InMemoryPorts::sample(Utc::now());
        let codes: Vec<_> = ports
            .units
            .units()
            .into_iter()
            .map(|unit| unit.code)
            .collect();
        assert_eq!(codes, vec!["AUD", "BTC", "VAS"]);
        assert_eq!(ports.base_unit.base_unit(), UnitId::SEED_AUD);
        assert!(ports.price_sources.price_source_records().is_empty());
        assert!(!ports.usage.unit_usage(UnitId::SEED_AUD).is_used());
    }

    #[test]
    fn upserting_a_unit_replaces_the_row_with_its_id_or_appends_it() {
        let mut units = InMemoryPorts::sample(Utc::now()).units;
        let mut btc = units.unit(UnitId::SEED_BTC).unwrap();
        btc.name = "Bitcoin core".into();
        units.upsert_unit(btc.clone());
        let eth = UnitRow::new(
            "ETH".into(),
            "Ethereum".into(),
            UnitKind::Crypto,
            Precision::of_kind(UnitKind::Crypto),
            PriceLink::manual(UnitId::SEED_AUD),
        );
        units.upsert_unit(eth.clone());

        assert_eq!(units.units().len(), 4);
        assert_eq!(units.unit(UnitId::SEED_BTC), Some(btc));
        assert_eq!(units.unit(eth.id), Some(eth));

        units.delete_unit(UnitId::SEED_BTC);
        assert_eq!(units.unit(UnitId::SEED_BTC), None);
    }

    #[test]
    fn the_prices_fixture_lands_a_fetch_now_and_collects_its_toast() {
        let mut prices = crate::presenter::test_support::prices();
        FetchNow {
            id: UnitId::SEED_BTC,
        }
        .run(
            &*prices.ports().borrow(),
            &mut prices.fetching().borrow_mut(),
            prices.now(),
        )
        .expect("sent");
        prices.later(Duration::seconds(1));

        let raised = prices.land(
            prices
                .transport()
                .answer_last(200, r#"{"bitcoin":{"aud":92568}}"#),
        );

        assert_eq!(
            raised,
            [Notice::Toast(
                lib_toast::ToastKind::Success,
                "Fetched BTC at 92,568.00 AUD".into()
            )]
        );
        assert_eq!(prices.notices(), raised.as_slice());
        assert_eq!(prices.take_notices(), raised);
        assert!(prices.notices().is_empty());
    }

    #[test]
    fn upserting_a_price_source_replaces_the_row_with_its_id_or_appends_it() {
        let mut sources = InMemoryPriceSources::default();
        let record = |name: &str| {
            PriceSourceRecord::BuiltIn(BuiltInOverride {
                id: COINGECKO,
                name: name.into(),
                updates: Updates::Daily,
            })
        };
        sources.upsert_price_source(record("Coins"));
        sources.upsert_price_source(record("Gecko"));
        assert_eq!(sources.price_source_records(), vec![record("Gecko")]);

        sources.delete_price_source(COINGECKO);
        assert!(sources.price_source_records().is_empty());
    }
}
