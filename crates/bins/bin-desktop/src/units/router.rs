//! `impl UnitsRouter` for the Desktop: where the Units page's Presenter, its prices view included,
//! asks to go. A Dialog opens as an Entity in the Dialog host through the generic [`PageRouter`],
//! built for each `handle` call (`PageView::with_router`), which queues a Notice for the input's
//! `send` to drain.

use gpui::{AppContext as _, Entity};
use lib_units::presenter::{
    DeletePriceSourcePresenter, PriceSourceFormPresenter, SetBaseUnitPresenter, SetPricePresenter,
    UnitFormPresenter, UnitsPagePresenter,
};
use lib_units::{UnitsDialog as Which, UnitsRouter};

use super::{UnitsPage, UnitsSetup};
use crate::announcer::Announcer;
use crate::shell::view::module_page::ModulePage;
use crate::shell::view::page_router::PageRouter;

impl UnitsRouter for PageRouter<'_, '_, UnitsPagePresenter> {
    /// Builds `dialog`'s Presenter on the shared ports, handed the Dialog keys, and hosts it as a
    /// `ModuleDialog`; its confirmed outcome comes back to the page through
    /// [`ModulePage::landing`], whether the tables or the prices view opened it. Delete unit is the
    /// Unit Deletion module's Dialog, which writes the Institutions that name the Unit too.
    fn open_dialog(&mut self, dialog: Which) -> bool {
        let setup = self.setup;
        let (ports, keys) = (setup.ports.clone(), setup.dialog_keys.get());
        match dialog {
            Which::AddUnit => self.open(UnitFormPresenter::add(ports, keys)),
            Which::EditUnit(id) => {
                let Some(form) = UnitFormPresenter::edit(ports, id, keys) else {
                    return false;
                };
                self.open(form);
            }
            Which::DeleteUnit(id) => {
                let landed = self.landing();
                return crate::unit_deletion::router::open_delete(
                    self.cx,
                    &ports,
                    &setup.deletion,
                    id,
                    keys,
                    landed,
                );
            }
            Which::SetBaseUnit => self.open(SetBaseUnitPresenter::new(ports, keys)),
            Which::SetPrice(id) => {
                let Some(set) = SetPricePresenter::new(ports, setup.clock.clone(), id, keys) else {
                    return false;
                };
                self.open(set);
            }
            Which::AddPriceSource => {
                let form = PriceSourceFormPresenter::add(ports, keys);
                self.open(testing(form, setup));
            }
            Which::EditPriceSource(id) => {
                let Some(form) = PriceSourceFormPresenter::edit(ports, id, keys) else {
                    return false;
                };
                self.open(testing(form, setup));
            }
            Which::DeletePriceSource(id) => {
                let Some(delete) = DeletePriceSourcePresenter::new(ports, id, keys) else {
                    return false;
                };
                self.open(delete);
            }
        }
        true
    }
}

/// `form` testing through the Open Ledger's fetching, when it has a price feed.
fn testing(form: PriceSourceFormPresenter, setup: &UnitsSetup) -> PriceSourceFormPresenter {
    form.with_testing(setup.fetching.clone(), setup.clock.clone())
}

/// The Units module's entry point: its Settings page, `presenter` routed over `setup`, which the
/// Settings View hosts as any other page.
pub fn units_page(
    presenter: UnitsPagePresenter,
    setup: UnitsSetup,
    announcer: Entity<Announcer>,
    cx: &mut gpui::App,
) -> Entity<UnitsPage> {
    cx.new(|cx| ModulePage::new(presenter, setup, &announcer, cx))
}
