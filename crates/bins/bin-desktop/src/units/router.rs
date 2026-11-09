//! `impl UnitsRouter` for the Desktop: where the Units page's and the Unit prices view's Presenters
//! ask to go. A Dialog opens as an Entity in the Dialog host through the page's gpui context, which
//! is why the Router is built for each `handle` call; the prices view to open or close is collected
//! for the page to apply once `handle` returns; and a Notice goes onto the bin's queue
//! (`crate::notices`), which the input's `send` drains.

use gpui::{App, Context};
use lib_toast::{Notice, Notify};
use lib_units::presenter::{
    DeletePriceSourcePresenter, PriceSourceFormPresenter, SetBaseUnitPresenter, SetPricePresenter,
    UnitFormPresenter, UnitsOutcome,
};
use lib_units::{UnitId, UnitsDialog as Which, UnitsRouter};

use super::UnitsSetup;
use super::view::page::UnitsPage;
use crate::announcer::Announcer;
use crate::notices::Queue;
use crate::shell::view::dialog_entity::ModuleDialog;

pub(super) struct PageRouter<'a, 'b> {
    pub ports: lib_units::SharedPorts,
    pub deletion: lib_unit_deletion::SharedUnitDeletion,
    /// The Dialog keys the Dialogs' status-line hints are built from.
    pub dialog_keys: crate::dialog_keys::LiveDialogKeys,
    /// The Clock the set price Dialog reads now from.
    pub clock: lib_core::Clock,
    pub cx: &'a mut Context<'b, UnitsPage>,
    /// Set once a Presenter asked to show a Unit's prices view (`Some`) or close it (`None`).
    pub prices: Option<Option<UnitId>>,
}

impl UnitsRouter for PageRouter<'_, '_> {
    /// Builds `dialog`'s Presenter on the shared ports, handed the Dialog keys, and hosts it as a
    /// `ModuleDialog`; its confirmed outcome comes back to the page through
    /// [`UnitsPage::dialog_landed`]. Delete unit is the Unit Deletion module's Dialog, which writes
    /// the Institutions that name the Unit too.
    fn open_dialog(&mut self, dialog: Which) -> bool {
        let page = self.cx.weak_entity();
        let landed = move |outcome: UnitsOutcome, cx: &mut App| {
            // A page that has gone has nothing to land on.
            let _ = page.update(cx, |page, cx| page.dialog_landed(outcome, cx));
        };
        let (ports, keys) = (self.ports.clone(), self.dialog_keys.get());
        match dialog {
            Which::AddUnit => {
                ModuleDialog::open(self.cx, UnitFormPresenter::add(ports, keys), landed);
            }
            Which::EditUnit(id) => {
                let Some(form) = UnitFormPresenter::edit(ports, id, keys) else {
                    return false;
                };
                ModuleDialog::open(self.cx, form, landed);
            }
            Which::DeleteUnit(id) => {
                return crate::unit_deletion::router::open_delete(
                    self.cx,
                    &ports,
                    &self.deletion,
                    id,
                    keys,
                    landed,
                );
            }
            Which::SetBaseUnit => {
                ModuleDialog::open(self.cx, SetBaseUnitPresenter::new(ports, keys), landed);
            }
            Which::SetPrice(id) => {
                let now = super::view::page::now_of(&self.clock);
                let Some(set) = SetPricePresenter::new(ports, now, id, keys) else {
                    return false;
                };
                ModuleDialog::open(self.cx, set, landed);
            }
            Which::AddPriceSource => {
                ModuleDialog::open(self.cx, PriceSourceFormPresenter::add(ports, keys), landed);
            }
            Which::EditPriceSource(id) => {
                let Some(form) = PriceSourceFormPresenter::edit(ports, id, keys) else {
                    return false;
                };
                ModuleDialog::open(self.cx, form, landed);
            }
            Which::DeletePriceSource(id) => {
                let Some(delete) = DeletePriceSourcePresenter::new(ports, id, keys) else {
                    return false;
                };
                ModuleDialog::open(self.cx, delete, landed);
            }
        }
        true
    }

    fn open_prices(&mut self, unit: UnitId) {
        self.prices = Some(Some(unit));
    }

    fn close_prices(&mut self) {
        self.prices = Some(None);
    }
}

impl Notify for PageRouter<'_, '_> {
    /// Queues `notice`, for the input's `send` to drain into the Shell.
    fn notify(&mut self, notice: Notice) {
        Queue.notify(notice);
    }
}

/// The Units module's entry point: its Settings page, built on `setup`'s ports and dated by
/// `clock`, which the Settings View hosts as any other page.
pub fn units_page(
    setup: UnitsSetup,
    announcer: gpui::Entity<Announcer>,
    clock: lib_core::Clock,
    cx: &mut App,
) -> gpui::Entity<UnitsPage> {
    use gpui::AppContext as _;
    cx.new(|cx| UnitsPage::new(setup, announcer, clock, cx))
}
