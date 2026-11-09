//! The Ledger Creation module's Desktop half (`docs/development/viper-architecture.md`, #1068):
//! the Create ledger Dialog's View, which the root Router opens when `:new`'s file explorer has
//! chosen a path. Every rule, every piece of text and the Presenter are `lib-ledger-creation`'s;
//! this folder turns gpui keys and clicks into Presenter events and draws the view model.
//!
//! Nothing is written until Create. A confirmed create lands on the root Router, which opens the
//! new Ledger (as `:open` does) and lands on the Dashboard; the Dialog is not the Shell, so it
//! cannot do that itself.

mod dialog;

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use gpui::{App, AppContext as _, Entity};
use lib_database::LedgerCreationAdapter;
use lib_ledger_creation::interactor::LedgerCreated;
use lib_ledger_creation::presenter::{CreateLedgerPresenter, SharedLedgerCreation};

use crate::shell::view::dialog_entity::ModuleDialog;

/// The Create ledger Dialog for the Ledger file at `path`, over the real creation port and
/// prefilled from the Locale's Currency. `created` runs with the Ledger once Create has written
/// it.
pub(crate) fn open(
    path: PathBuf,
    created: impl Fn(LedgerCreated, &mut App) + 'static,
    cx: &mut App,
) -> Entity<ModuleDialog<CreateLedgerPresenter>> {
    let creation: SharedLedgerCreation = Rc::new(RefCell::new(LedgerCreationAdapter::new()));
    let presenter = CreateLedgerPresenter::new(
        creation,
        path,
        lib_locale::locale().currency(),
        lib_locale::msg::institution_personal(),
    );
    cx.new(|_| ModuleDialog::new(presenter, created))
}
