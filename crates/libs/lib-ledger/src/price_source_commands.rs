//! The Price Source writes (ADR-0038, ADR-0043): each a command value with its own refusal,
//! checked before anything is written so a refusal leaves the Ledger untouched, and recording the
//! rows it wrote (ADR-0039).
//!
//! Only a Custom source is added or deleted. A built-in is edited by storing the person's name and
//! Updates for it; its address and request stay in code. Manual entry is never written.

use lib_core::PriceSourceId;
use lib_units::{
    CustomSource, Endpoint, MANUAL_ENTRY, Updates, add_custom_source, built_in, edit_custom_source,
    move_units_to_manual, override_built_in, remove_custom_source,
};

use crate::{Ledger, LedgerChange, LedgerCommand};

/// Why an edit of a Price Source was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum EditPriceSourceRefusal {
    /// No source has the id, which only happens when it was deleted while its Dialog was open.
    #[error("price source not found")]
    Gone,
    /// Manual entry has nothing to edit.
    #[error("manual entry can't be edited")]
    ManualEntry,
    /// A built-in's address, request, price path and auth are fixed in code.
    #[error("a built-in price source's endpoint can't be edited")]
    BuiltInEndpoint,
}

/// Why a delete of a Price Source was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DeletePriceSourceRefusal {
    #[error("price source not found")]
    Gone,
    /// A built-in is always listed.
    #[error("a built-in price source can't be deleted")]
    BuiltIn,
    /// Manual entry is the fallback every Unit can sit on.
    #[error("manual entry can't be deleted")]
    ManualEntry,
}

/// What deleting a Custom source did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PriceSourceDeleted {
    pub source: CustomSource,
    /// How many Units moved to Manual entry, each keeping its last price.
    pub moved: usize,
}

impl Ledger {
    pub fn add_price_source(
        &mut self,
        name: String,
        endpoint: Endpoint,
        updates: Updates,
    ) -> PriceSourceId {
        let source = add_custom_source(&mut self.price_sources, name, endpoint, updates);
        let id = source.id;
        self.pending.push(LedgerChange::UpsertPriceSource(
            lib_units::PriceSourceRecord::Custom(source),
        ));
        id
    }

    /// Renames and retimes source `id`. A Custom source also takes a new endpoint when `endpoint`
    /// is given; a built-in refuses one.
    pub fn edit_price_source(
        &mut self,
        id: PriceSourceId,
        name: String,
        updates: Updates,
        endpoint: Option<Endpoint>,
    ) -> Result<(), EditPriceSourceRefusal> {
        if id == MANUAL_ENTRY {
            return Err(EditPriceSourceRefusal::ManualEntry);
        }
        let record = if built_in(id).is_some() {
            if endpoint.is_some() {
                return Err(EditPriceSourceRefusal::BuiltInEndpoint);
            }
            override_built_in(&mut self.price_sources, id, name, updates)
        } else {
            edit_custom_source(&mut self.price_sources, id, name, updates, endpoint)
                .ok_or(EditPriceSourceRefusal::Gone)?
        };
        self.pending.push(LedgerChange::UpsertPriceSource(record));
        Ok(())
    }

    /// Deletes Custom source `id` and moves its Units to Manual entry, keeping their last price.
    pub fn delete_price_source(
        &mut self,
        id: PriceSourceId,
    ) -> Result<PriceSourceDeleted, DeletePriceSourceRefusal> {
        if id == MANUAL_ENTRY {
            return Err(DeletePriceSourceRefusal::ManualEntry);
        }
        if built_in(id).is_some() {
            return Err(DeletePriceSourceRefusal::BuiltIn);
        }
        let source = remove_custom_source(&mut self.price_sources, id)
            .ok_or(DeletePriceSourceRefusal::Gone)?;
        let moved = move_units_to_manual(&mut self.units, id);
        let count = moved.len();
        self.pending
            .extend(moved.into_iter().map(LedgerChange::UpsertUnit));
        self.pending.push(LedgerChange::DeletePriceSource(id));
        Ok(PriceSourceDeleted {
            source,
            moved: count,
        })
    }
}

/// [`Ledger::add_price_source`] as a value: adds a Custom source; the outcome is its id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddPriceSource {
    pub name: String,
    pub endpoint: Endpoint,
    pub updates: Updates,
}

impl LedgerCommand for AddPriceSource {
    type Outcome = PriceSourceId;
    /// The form has already validated the source, so an add can't be refused.
    type Refusal = std::convert::Infallible;
    fn run(self, ledger: &mut Ledger) -> Result<PriceSourceId, Self::Refusal> {
        Ok(ledger.add_price_source(self.name, self.endpoint, self.updates))
    }
}

/// [`Ledger::edit_price_source`] as a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditPriceSource {
    pub id: PriceSourceId,
    pub name: String,
    pub updates: Updates,
    /// A Custom source's new endpoint, or `None` to keep it. Always `None` for a built-in.
    pub endpoint: Option<Endpoint>,
}

impl LedgerCommand for EditPriceSource {
    type Outcome = ();
    type Refusal = EditPriceSourceRefusal;
    fn run(self, ledger: &mut Ledger) -> Result<(), EditPriceSourceRefusal> {
        ledger.edit_price_source(self.id, self.name, self.updates, self.endpoint)
    }
}

/// [`Ledger::delete_price_source`] as a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeletePriceSource {
    pub id: PriceSourceId,
}

impl LedgerCommand for DeletePriceSource {
    type Outcome = PriceSourceDeleted;
    type Refusal = DeletePriceSourceRefusal;
    fn run(self, ledger: &mut Ledger) -> Result<PriceSourceDeleted, DeletePriceSourceRefusal> {
        ledger.delete_price_source(self.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lib_core::UnitId;
    use lib_units::{
        BuiltInOverride, COINGECKO, FRANKFURTER, KeyPlacement, PriceLink, PriceSourceAuth,
        PriceSourceRecord, UnitKind,
    };

    const MANUAL: &str = "Manual entry";

    fn endpoint(address: &str) -> Endpoint {
        Endpoint {
            address: address.to_string().into(),
            request: "/price?id={symbol}&in={quote}".to_string().into(),
            price_path: "/price".to_string().into(),
            auth: PriceSourceAuth::ApiKey {
                placement: KeyPlacement::Param,
                name: "key".to_string().into(),
            },
        }
    }

    fn add_custom(ledger: &mut Ledger) -> PriceSourceId {
        let Ok(id) = AddPriceSource {
            name: "Broker".into(),
            endpoint: endpoint("https://broker.example"),
            updates: Updates::Daily,
        }
        .run(ledger);
        id
    }

    #[test]
    fn adding_a_custom_source_lists_it_and_records_it() {
        let mut ledger = Ledger::stub();
        let id = add_custom(&mut ledger);

        let source = ledger
            .price_source(MANUAL, id)
            .expect("the source is listed");
        assert_eq!((source.name, source.updates), ("Broker", Updates::Daily));
        assert_eq!(source.endpoint(), Some(&endpoint("https://broker.example")));
        assert!(UnitKind::all().iter().all(|kind| source.prices(*kind)));
        assert_eq!(
            ledger.drain_changes(),
            vec![LedgerChange::UpsertPriceSource(
                ledger.price_source_records()[0].clone()
            )]
        );
    }

    #[test]
    fn editing_a_custom_source_rewrites_it_and_records_it() {
        let mut ledger = Ledger::stub();
        let id = add_custom(&mut ledger);
        ledger.drain_changes();

        EditPriceSource {
            id,
            name: "My broker".into(),
            updates: Updates::Hourly,
            endpoint: Some(endpoint("https://new.example")),
        }
        .run(&mut ledger)
        .unwrap();

        let source = ledger.price_source(MANUAL, id).unwrap();
        assert_eq!(
            (source.name, source.updates),
            ("My broker", Updates::Hourly)
        );
        assert_eq!(source.endpoint(), Some(&endpoint("https://new.example")));
        assert_eq!(ledger.drain_changes().len(), 1);
    }

    #[test]
    fn editing_a_built_in_stores_only_its_name_and_updates() {
        let mut ledger = Ledger::stub();

        EditPriceSource {
            id: COINGECKO,
            name: "Gecko".into(),
            updates: Updates::Daily,
            endpoint: None,
        }
        .run(&mut ledger)
        .unwrap();

        let stored = PriceSourceRecord::BuiltIn(BuiltInOverride {
            id: COINGECKO,
            name: "Gecko".into(),
            updates: Updates::Daily,
        });
        assert_eq!(ledger.price_source_records(), std::slice::from_ref(&stored));
        let source = ledger.price_source(MANUAL, COINGECKO).unwrap();
        assert_eq!(source.name, "Gecko");
        assert_eq!(
            source.endpoint(),
            Some(&built_in(COINGECKO).unwrap().endpoint)
        );
        assert_eq!(
            ledger.drain_changes(),
            vec![LedgerChange::UpsertPriceSource(stored)]
        );
    }

    #[test]
    fn a_built_ins_endpoint_and_manual_entry_are_not_editable() {
        let mut ledger = Ledger::stub();
        assert_eq!(
            ledger.edit_price_source(
                COINGECKO,
                "x".into(),
                Updates::Daily,
                Some(endpoint("https://x.example"))
            ),
            Err(EditPriceSourceRefusal::BuiltInEndpoint)
        );
        assert_eq!(
            ledger.edit_price_source(MANUAL_ENTRY, "x".into(), Updates::Daily, None),
            Err(EditPriceSourceRefusal::ManualEntry)
        );
        assert_eq!(
            ledger.edit_price_source(PriceSourceId::new(), "x".into(), Updates::Daily, None),
            Err(EditPriceSourceRefusal::Gone)
        );
        assert!(ledger.price_source_records().is_empty());
        assert!(ledger.drain_changes().is_empty());
    }

    #[test]
    fn deleting_a_built_in_or_manual_entry_is_refused_and_the_ledger_is_untouched() {
        let mut ledger = Ledger::stub();
        let units = ledger.units().to_vec();

        assert_eq!(
            DeletePriceSource { id: COINGECKO }.run(&mut ledger),
            Err(DeletePriceSourceRefusal::BuiltIn)
        );
        assert_eq!(
            DeletePriceSource { id: FRANKFURTER }.run(&mut ledger),
            Err(DeletePriceSourceRefusal::BuiltIn)
        );
        assert_eq!(
            DeletePriceSource { id: MANUAL_ENTRY }.run(&mut ledger),
            Err(DeletePriceSourceRefusal::ManualEntry)
        );
        assert_eq!(
            DeletePriceSource {
                id: PriceSourceId::new()
            }
            .run(&mut ledger),
            Err(DeletePriceSourceRefusal::Gone)
        );
        assert_eq!(ledger.units(), units);
        assert_eq!(ledger.price_sources(MANUAL).len(), 4);
        assert!(ledger.drain_changes().is_empty());
    }

    #[test]
    fn deleting_a_custom_source_moves_its_units_to_manual_entry_with_their_last_price() {
        let mut ledger = Ledger::stub();
        let id = add_custom(&mut ledger);
        let btc = ledger.unit(UnitId::SEED_BTC).unwrap().clone();
        ledger
            .edit_unit(
                btc.id,
                btc.code.clone(),
                btc.name.clone(),
                btc.kind,
                PriceLink {
                    source_id: id,
                    ..btc.link.clone()
                },
            )
            .unwrap();
        ledger.drain_changes();

        let deleted = DeletePriceSource { id }.run(&mut ledger).unwrap();

        assert_eq!(deleted.source.name, "Broker");
        assert_eq!(deleted.moved, 1);
        let moved = ledger.unit(UnitId::SEED_BTC).unwrap().clone();
        assert_eq!(moved.link.source_id, MANUAL_ENTRY);
        assert_eq!(moved.link.symbol, btc.link.symbol);
        assert_eq!(moved.last_price, btc.last_price);
        assert!(moved.last_price.is_some());
        assert!(ledger.price_source(MANUAL, id).is_none());
        assert_eq!(
            ledger.drain_changes(),
            vec![
                LedgerChange::UpsertUnit(moved),
                LedgerChange::DeletePriceSource(id)
            ]
        );
    }

    #[test]
    fn the_stub_links_btc_to_coingecko_fetched_five_minutes_ago() {
        let today = chrono::NaiveDate::from_ymd_opt(2026, 10, 18).unwrap();
        let ledger = Ledger::stub_on(today);
        let noon = today.and_hms_opt(12, 0, 0).unwrap().and_utc();
        assert_eq!(
            ledger.last_fetched(COINGECKO),
            Some(noon - chrono::Duration::minutes(5))
        );
        assert_eq!(ledger.last_fetched(MANUAL_ENTRY), None);
        let on_manual: Vec<_> = ledger
            .units()
            .iter()
            .filter(|unit| unit.link.source_id == MANUAL_ENTRY)
            .map(|unit| unit.code.as_str())
            .collect();
        assert_eq!(on_manual, vec!["aud", "vas"]);
    }

    #[test]
    fn the_ledger_offers_a_kind_its_matching_sources() {
        let mut ledger = Ledger::stub();
        add_custom(&mut ledger);
        let names: Vec<_> = ledger
            .sources_for_kind(MANUAL, UnitKind::Currency)
            .iter()
            .map(|source| source.name)
            .collect();
        assert_eq!(names, vec!["Broker", "Frankfurter", "Manual entry"]);
    }
}
