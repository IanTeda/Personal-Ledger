//! The Ledger Creation module's Desktop half (`docs/development/viper-architecture.md`, #1068):
//! the Create ledger Dialog's View, which the root Router opens when `:new`'s file explorer has
//! chosen a path. Every rule, every piece of text and the Presenter are `lib-ledger-creation`'s;
//! this folder turns gpui keys and clicks into Presenter events and draws the view model.
//!
//! Nothing is written until Create. A confirmed create emits [`LedgerCreatedEvent`], which the
//! root Router answers by opening the new Ledger (as `:open` does) and landing on the Dashboard;
//! the Dialog is not the Shell, so it cannot do that itself.

mod dialog;

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use gpui::{App, AppContext as _, Entity};
use lib_database::LedgerCreationAdapter;
use lib_ledger_creation::presenter::{CreateLedgerPresenter, SharedLedgerCreation};

pub use dialog::{CreateLedgerDialog, LedgerCreatedEvent};

/// The Create ledger Dialog for the Ledger file at `path`, over the real creation port and
/// prefilled from the Locale's Currency.
pub(crate) fn open(path: PathBuf, cx: &mut App) -> Entity<CreateLedgerDialog> {
    let creation: SharedLedgerCreation = Rc::new(RefCell::new(LedgerCreationAdapter::new()));
    let presenter = CreateLedgerPresenter::new(
        creation,
        path,
        lib_locale::locale().currency(),
        lib_locale::msg::institution_personal(),
    );
    cx.new(|_| CreateLedgerDialog::new(presenter))
}
