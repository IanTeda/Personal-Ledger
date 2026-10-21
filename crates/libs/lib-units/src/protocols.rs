//! The ports the Units Interactors reach storage through. They are synchronous (ADR-0046): an
//! adapter serves reads from memory and queues each write, so a write never fails here; a failed
//! save is the adapter's to report. `fake` implements them in memory.
//!
//! The Presenters reach the ports together, through [`UnitsPorts`], and ask for navigation through
//! [`UnitsRouter`], which each Client implements.

use std::cell::RefCell;
use std::rc::Rc;

use lib_core::{PriceSourceId, UnitId};
use lib_toast::ToastKind;

use crate::{PriceSourceRecord, UnitRow};

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

/// How many rows name a Unit, carried by the delete refusal so a Dialog can say why and offer
/// Deactivate instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UnitUse {
    pub accounts: usize,
    pub bill_plans: usize,
    pub budgets: usize,
}

impl UnitUse {
    pub fn is_used(self) -> bool {
        self.accounts + self.bill_plans + self.budgets > 0
    }
}

/// Every Units port, as one value the Presenters share. An Interactor writes through several ports
/// at once, so [`Self::split_mut`] lends them out together rather than one `&mut` at a time.
pub trait UnitsPorts {
    fn units(&self) -> &dyn UnitRepository;

    fn price_sources(&self) -> &dyn PriceSourceRepository;

    fn base_unit(&self) -> &dyn BaseUnitRepository;

    fn usage(&self) -> &dyn UnitUsage;

    fn split_mut(&mut self) -> PortsMut<'_>;
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

/// What a confirmed Units Dialog did, handed to the page when the Dialog closes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnitsOutcome {
    /// A Unit added or edited.
    Saved {
        code: String,
    },
    Deleted {
        code: String,
    },
    Deactivated {
        code: String,
    },
    Reactivated {
        code: String,
    },
    BaseSet {
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
/// Dialog with the gpui context that call runs in.
pub trait UnitsRouter {
    /// Builds `dialog`'s Presenter on the shared ports and hosts its View. Returns whether it
    /// opened: a Dialog on a row removed since it was asked for has no Presenter to build.
    fn open_dialog(&mut self, dialog: UnitsDialog) -> bool;

    /// Closes the open Units Dialog. `outcome` is what its confirm did, `None` when it was
    /// cancelled; the Client hands it to the page as [`UnitsPageEvent::DialogClosed`].
    ///
    /// [`UnitsPageEvent::DialogClosed`]: crate::presenter::UnitsPageEvent::DialogClosed
    fn close_dialog(&mut self, outcome: Option<UnitsOutcome>);

    /// Raises a Toast in the Shell.
    fn raise_toast(&mut self, kind: ToastKind, text: String);

    /// Puts `text` on the Shell's status line.
    fn set_status(&mut self, text: String);
}
