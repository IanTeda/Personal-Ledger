//! The ports the Units Interactors reach storage through. They are synchronous (ADR-0046): an
//! adapter serves reads from memory and queues each write, so a write never fails here; a failed
//! save is the adapter's to report. `fake` implements them in memory.
//!
//! The Presenters reach the ports together, through [`UnitsPorts`], and ask for navigation through
//! [`UnitsRouter`], which each Client implements.

use std::cell::RefCell;
use std::rc::Rc;

use lib_core::{AccountType, Money, PriceSourceId, UnitId};
use lib_toast::Notify;

use crate::{PriceSource, PriceSourceRecord, UnitRow, price_source, price_sources};

/// The Units on the Ledger.
pub trait UnitRepository {
    /// Every Unit, in no particular order.
    fn units(&self) -> Vec<UnitRow>;

    fn unit(&self, id: UnitId) -> Option<UnitRow> {
        self.units().into_iter().find(|unit| unit.id == id)
    }

    /// Writes `unit`, replacing the row with its id or adding it.
    fn upsert_unit(&mut self, unit: UnitRow);

    fn delete_unit(&mut self, id: UnitId);
}

/// The stored Price Source rows: Custom sources and a person's overrides of the built-ins. Manual
/// entry and the built-in definitions are code, never stored (ADR-0043).
pub trait PriceSourceRepository {
    fn price_source_records(&self) -> Vec<PriceSourceRecord>;

    /// Writes `record`, replacing the row with its id or adding it.
    fn upsert_price_source(&mut self, record: PriceSourceRecord);

    fn delete_price_source(&mut self, id: PriceSourceId);
}

/// Which Unit is the base: what new Accounts default to and what the net worth is figured in.
pub trait BaseUnitRepository {
    fn base_unit(&self) -> UnitId;

    fn set_base_unit(&mut self, id: UnitId);
}

/// How many rows in other domains name a Unit. A read port, so `lib-units` needn't depend on the
/// Accounts, Bills or Budgets crates to refuse deleting a Unit in use.
pub trait UnitUsage {
    fn unit_usage(&self, id: UnitId) -> UnitUse;
}

/// The Accounts held in a Unit, for the detail pane's ACCOUNTS group and its Used by count. A read
/// port, so `lib-units` needn't depend on the Accounts crate.
pub trait UnitHoldings {
    /// The Accounts whose Unit is `id`, in the Ledger's order.
    fn holdings(&self, id: UnitId) -> Vec<Holding>;
}

/// One Account held in a Unit, as the Units page shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Holding {
    pub name: String,
    pub kind: AccountType,
    pub balance: Money,
    pub transactions: u32,
}

/// How many rows name a Unit, carried by the delete refusal so a Dialog can say why and offer
/// Deactivate instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UnitUse {
    pub accounts: usize,
    pub bill_plans: usize,
    pub budgets: usize,
    pub locations: usize,
}

impl UnitUse {
    pub fn is_used(self) -> bool {
        self.accounts + self.bill_plans + self.budgets + self.locations > 0
    }
}

/// Every Units port, as one value the Presenters share. An Interactor writes through several ports
/// at once, so [`Self::split_mut`] lends them out together rather than one `&mut` at a time.
pub trait UnitsPorts {
    fn units(&self) -> &dyn UnitRepository;

    fn price_sources(&self) -> &dyn PriceSourceRepository;

    fn base_unit(&self) -> &dyn BaseUnitRepository;

    fn usage(&self) -> &dyn UnitUsage;

    fn holdings(&self) -> &dyn UnitHoldings;

    fn split_mut(&mut self) -> PortsMut<'_>;

    /// The Price Sources as the Presenters list them, read once from the stored records.
    fn source_catalogue(&self) -> SourceCatalogue {
        SourceCatalogue {
            records: self.price_sources().price_source_records(),
            manual: lib_locale::msg::price_source_manual_entry(),
        }
    }
}

/// The stored Price Source records with Manual entry's localised name, which together make the
/// sources ([`price_sources`]): owned, so the sources can borrow from it.
pub struct SourceCatalogue {
    records: Vec<PriceSourceRecord>,
    manual: String,
}

impl SourceCatalogue {
    /// Every source, in [`price_sources`]' order.
    pub fn sources(&self) -> Vec<PriceSource<'_>> {
        price_sources(&self.records, &self.manual)
    }

    pub fn source(&self, id: PriceSourceId) -> Option<PriceSource<'_>> {
        price_source(&self.records, &self.manual, id)
    }

    /// Manual entry's name in the Locale.
    pub fn manual_name(&self) -> &str {
        &self.manual
    }
}

/// The ports borrowed together for one Interactor call.
pub struct PortsMut<'a> {
    pub units: &'a mut dyn UnitRepository,
    pub price_sources: &'a mut dyn PriceSourceRepository,
    pub base_unit: &'a mut dyn BaseUnitRepository,
    pub usage: &'a dyn UnitUsage,
}

/// The ports as the page and its Dialogs hold them: one set, shared, so a Unit a Dialog adds is in
/// the page's table as soon as the Dialog closes. Single-threaded, as both Clients' UI threads are.
pub type SharedPorts = Rc<RefCell<dyn UnitsPorts>>;

/// What a Dialog did to a Unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitChange {
    /// Added or edited.
    Saved,
    Deleted,
    Deactivated,
    Reactivated,
    BaseSet,
}

/// What a confirmed Units Dialog did, handed to the page when the Dialog closes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnitsOutcome {
    /// What a Dialog did to the Unit `code`.
    Unit {
        change: UnitChange,
        code: String,
    },
    SourceSaved {
        name: String,
    },
    /// `moved` Units fell back to Manual entry.
    SourceDeleted {
        name: String,
        moved: usize,
    },
}

/// A Units Dialog, as the page asks the Router to open it and as its view model names the one
/// open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitsDialog {
    AddUnit,
    EditUnit(UnitId),
    DeleteUnit(UnitId),
    SetBaseUnit,
    AddPriceSource,
    EditPriceSource(PriceSourceId),
    DeletePriceSource(PriceSourceId),
}

/// Navigation out of the Units Presenters, implemented by each Client. The Client passes it into
/// each `handle` call rather than the Presenter holding it, since the Desktop can only open a
/// Dialog with the gpui context that call runs in. A Toast or status-line message goes to the
/// Shell through [`Notify`] (ADR-0048).
pub trait UnitsRouter: Notify {
    /// Builds `dialog`'s Presenter on the shared ports and hosts its View. Returns whether it
    /// opened: a Dialog on a row removed since it was asked for has no Presenter to build.
    fn open_dialog(&mut self, dialog: UnitsDialog) -> bool;

    /// Closes the open Units Dialog. `outcome` is what its confirm did, `None` when it was
    /// cancelled; the Client hands it to the page as [`UnitsPageEvent::DialogClosed`].
    ///
    /// [`UnitsPageEvent::DialogClosed`]: crate::presenter::UnitsPageEvent::DialogClosed
    fn close_dialog(&mut self, outcome: Option<UnitsOutcome>);
}
