//! The Unit Deletion module's entry point for other modules' pages ([`open_delete`]). The Delete
//! Dialog's Presenter closes it through the host's `HostRouter`, which puts a Notice onto the
//! bin's queue (`crate::notices`).

use gpui::{App, Context, EventEmitter};
use lib_core::UnitId;
use lib_unit_deletion::SharedUnitDeletion;
use lib_unit_deletion::presenter::DeleteUnitPresenter;
use lib_units::presenter::UnitsOutcome;
use lib_viper::dialog::DialogKeys;

use crate::shell::view::dialog_entity::ModuleDialog;
use crate::view::event::ViewEvent;

/// Opens the Delete unit Dialog on Unit `id` over page `P`, whose `landed` runs with the outcome
/// once a confirm lands, so the page redraws. Usage is copied from the Ledger as it opens, and its
/// hints come from `keys`. Returns whether it opened: a Unit that has gone has no Dialog.
pub fn open_delete<P: EventEmitter<ViewEvent> + 'static>(
    cx: &mut Context<'_, P>,
    ports: &lib_units::SharedPorts,
    deletion: &SharedUnitDeletion,
    id: UnitId,
    keys: DialogKeys,
    landed: impl Fn(UnitsOutcome, &mut App) + 'static,
) -> bool {
    let Some(presenter) = DeleteUnitPresenter::new(ports.clone(), deletion.clone(), id, keys)
    else {
        return false;
    };
    ModuleDialog::open(cx, presenter, landed);
    true
}
