//! In-memory ports and the sample seed, for tests and UI-first work. Each holds its rows in a plain
//! field so a test can set up or read state directly. [`RecordingRouter`] is a Router that only
//! records what it was asked. [`FakeTransport`] records each price request and answers it on cue,
//! and [`LedgerPricesFixture`] is where every price-fetching test starts.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::mpsc::{Receiver, Sender};

use chrono::{DateTime, Duration, Utc};
use lib_core::{PriceSourceId, UnitId};
use lib_toast::{Notice, Notify};
use lib_viper::dialog::DialogRouter;
use secrecy::{ExposeSecret as _, SecretString};

use crate::fetching::{
    PriceFetching, PriceRequest, PriceResponse, PriceTransport, RequestId, SharedFetching,
    TransportOutcome,
};
use crate::presenter::{Clock, FetchedPricesPresenter};
use crate::protocols::{PortsMut, SharedPorts, UnitsDialog, UnitsOutcome, UnitsPorts, UnitsRouter};
use crate::{
    BaseUnitRepository, Error, Holding, KeyState, PriceSourceKeys, PriceSourceRecord,
    PriceSourceRepository, Result, UnitHoldings, UnitOpenings, UnitRepository, UnitRow, UnitUsage,
    UnitUse, sample_units,
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
    pub by_unit: HashMap<UnitId, chrono::NaiveDate>,
}

impl UnitOpenings for InMemoryOpenings {
    fn earliest_opened(&self, id: UnitId) -> Option<chrono::NaiveDate> {
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

/// An Open Ledger's price fetching as a test drives it: the fake Units ports, a [`FakeTransport`],
/// a clock the test sets, the shared fetching, and the fetched-prices Presenter every response
/// lands through, collecting the Notices each landing raises. It wraps today's
/// [`FetchedPricesPresenter`] under the name the Ledger prices module will take, so the tests that
/// build on it move with that rename rather than each rebuilding this setup.
pub struct LedgerPricesFixture {
    ports: Rc<RefCell<InMemoryPorts>>,
    shared: SharedPorts,
    transport: FakeTransport,
    fetching: SharedFetching,
    at: Rc<Cell<DateTime<Utc>>>,
    prices: FetchedPricesPresenter,
    notices: Vec<Notice>,
}

impl LedgerPricesFixture {
    /// `ports` with fetching over a fresh [`FakeTransport`], the clock reading `now`.
    pub fn new(ports: InMemoryPorts, now: DateTime<Utc>) -> Self {
        let (ports, shared) = ports.shared();
        let transport = FakeTransport::default();
        let fetching = PriceFetching::shared(Rc::new(transport.clone()));
        let at = Rc::new(Cell::new(now));
        let clock: Clock = {
            let at = at.clone();
            Rc::new(move || at.get())
        };
        Self {
            prices: FetchedPricesPresenter::new(shared.clone(), fetching.clone(), clock),
            ports,
            shared,
            transport,
            fetching,
            at,
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
        &self.fetching
    }

    /// A clock reading what [`Self::set_now`] and [`Self::later`] last set.
    pub fn clock(&self) -> Clock {
        let at = self.at.clone();
        Rc::new(move || at.get())
    }

    pub fn now(&self) -> DateTime<Utc> {
        self.at.get()
    }

    pub fn set_now(&self, now: DateTime<Utc>) {
        self.at.set(now);
    }

    /// Moves the clock on `by`.
    pub fn later(&self, by: Duration) {
        self.at.set(self.at.get() + by);
    }

    /// The fetched-prices Presenter, for its scheduled check and backfill.
    pub fn prices(&mut self) -> &mut FetchedPricesPresenter {
        &mut self.prices
    }

    /// Lands `response` as a Client's input door does, returning the Notices it raised; they are
    /// collected in [`Self::notices`] too.
    pub fn land(&mut self, response: PriceResponse) -> Vec<Notice> {
        let mut raised = Vec::new();
        self.prices.handle(response, &mut raised);
        self.notices.extend(raised.iter().cloned());
        raised
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BuiltInOverride, COINGECKO, Precision, PriceLink, UnitKind, Updates};

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
        crate::interactor::FetchNow {
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
