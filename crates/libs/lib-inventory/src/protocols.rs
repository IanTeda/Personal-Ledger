//! The port Inventory reaches storage through. It is synchronous (ADR-0046): an adapter serves
//! reads from memory and queues each write, so a write never fails here; a failed save is the
//! adapter's to report. `fake` implements it in memory.
//!
//! There is no delete for a Location: removing one also drops the Document Links to its Items, so
//! `lib-ledger` runs it and the Client's forwarding shim deletes the row (Q18). It is the reassign
//! port ([`InventoryReassign`], Q6) the Remove location Dialog's Presenter confirms through, which a
//! Client answers through its Ledger until `lib-reassign` (#813); a Presenter's confirm is lent it
//! rather than the Presenter holding it, since the Desktop writes its Ledger only with the gpui
//! context that confirm runs in. The reads of other domains a Dialog quotes or lists
//! ([`InventoryUnits`], [`LocationUsage`]) are lent the same way, when the Dialog opens. The
//! Presenters ask for navigation through [`InventoryRouter`], which each Client implements.

use std::cell::RefCell;
use std::rc::Rc;

use crate::{Inventory, Item, Location, LocationError};
use lib_core::{AreaId, ItemId, LocationId, UnitId};
use lib_toast::Notify;

/// The Locations and Items on the Ledger.
pub trait InventoryRepository {
    /// Every Location, in the order they were added, each with its cover and its Areas in tab
    /// order.
    fn locations(&self) -> Vec<Location>;

    /// Every Item, in the order they were added.
    fn items(&self) -> Vec<Item>;

    fn location(&self, id: LocationId) -> Option<Location> {
        self.locations().into_iter().find(|l| l.id == id)
    }

    fn item(&self, id: ItemId) -> Option<Item> {
        self.items().into_iter().find(|i| i.id == id)
    }

    /// Both lists as the one value the rules run over.
    fn inventory(&self) -> Inventory {
        Inventory {
            locations: self.locations(),
            items: self.items(),
        }
    }

    /// Writes `location` over the row with its id, or as a new row after every row already there.
    /// Its cover and its Areas go with it, in order: an Area the row had and `location` no longer
    /// lists is deleted, so the Items in it are moved first.
    fn upsert_location(&mut self, location: Location);

    /// Writes `item` over the row with its id, or as a new row after every row already there.
    fn upsert_item(&mut self, item: Item);

    fn delete_item(&mut self, id: ItemId);
}

/// The repository as the page and its Dialogs hold it: one set, shared, so an Area a Dialog adds is
/// on the page as soon as the Dialog closes. Single-threaded, as both Clients' UI threads are.
pub type SharedInventory = Rc<RefCell<dyn InventoryRepository>>;

/// A fiat Unit a Location can be kept in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitChoice {
    pub id: UnitId,
    /// Lower case, as Units hold it.
    pub code: String,
    pub name: String,
}

/// The reads of Units the Location form makes, so `lib-inventory` needn't depend on the Units
/// crate. The form copies them in when it opens, as it is modal.
pub trait InventoryUnits {
    /// The fiat Units, in Settings order: a Location's Unit is chosen from them when it is added.
    fn currencies(&self) -> Vec<UnitChoice>;

    /// Unit `id`'s code, lower case, or `None` when it has none.
    fn unit_code(&self, id: UnitId) -> Option<String>;
}

/// What names a Location in another domain.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LocationUse {
    /// The Documents that Link to one of the Location's Items, which lose that Link when it goes.
    pub documents: usize,
}

/// The read the Remove location Dialog quotes its Documents through, since Documents depend on
/// Inventory and never the reverse.
pub trait LocationUsage {
    fn location_usage(&self, id: LocationId) -> LocationUse;
}

/// What a Location remove did, for its Toast.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocationRemoved {
    /// The Location's name, read before it went.
    pub name: String,
}

/// Location remove (Q6): it takes the Location's Areas and Items with it and drops the Document
/// Links to those Items, in the one commit of the input, so it is no `lib-inventory` Interactor.
/// The Remove location Dialog's Presenter confirms through it, the pattern Categories set
/// (`CategoryReassign`), and the port `lib-reassign` (#813) answers.
pub trait InventoryReassign {
    /// Removes Location `id` with its Areas and Items, used or not.
    fn remove_location(&mut self, id: LocationId) -> Result<LocationRemoved, LocationError>;
}

/// A row of the page: a Location, or an Area of an open Location.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InventoryRow {
    Location(LocationId),
    Area(AreaId),
}

/// An Inventory Dialog, as the page asks the Router to open it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InventoryDialog {
    /// `unit` is the Unit to pre-select, when the Location it is chosen from is a fiat one the
    /// form offers.
    AddLocation {
        unit: Option<UnitId>,
    },
    EditLocation(LocationId),
    RemoveLocation(LocationId),
    AddArea(LocationId),
    EditArea(AreaId),
    RemoveArea(AreaId),
    /// The notice a Location's last Area gets in place of a Remove while it holds Items.
    AreaBlocked(AreaId),
}

/// What a Dialog did to the Inventory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InventoryChange {
    LocationAdded,
    LocationSaved,
    LocationRemoved,
    AreaAdded,
    AreaSaved,
    AreaRemoved,
}

/// What a confirmed Inventory Dialog did, handed to the page when the Dialog closes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InventoryOutcome {
    pub change: InventoryChange,
    /// The Location it was done to or in.
    pub location: LocationId,
    /// The row the cursor lands on: the one saved or added, or the neighbour that took a removed
    /// row's place. `None` once the last Location has gone.
    pub selected: Option<InventoryRow>,
}

/// Navigation out of the Inventory Presenters, implemented by each Client. The Client passes it
/// into each `handle` call rather than the Presenter holding it, since the Desktop can only open a
/// Dialog with the gpui context that call runs in. A Toast or status-line message goes to the
/// Shell through [`Notify`] (ADR-0048).
pub trait InventoryRouter: Notify {
    /// Builds `dialog`'s Presenter and hosts its View. Returns whether it opened: a Dialog on a
    /// row removed since it was asked for has no Presenter to build.
    fn open_dialog(&mut self, dialog: InventoryDialog) -> bool;

    /// Closes the open Inventory Dialog. `outcome` is what its confirm did, `None` when it was
    /// cancelled; the Client hands it to the page as [`InventoryPageEvent::DialogClosed`].
    ///
    /// [`InventoryPageEvent::DialogClosed`]: crate::presenter::InventoryPageEvent::DialogClosed
    fn close_dialog(&mut self, outcome: Option<InventoryOutcome>);
}
