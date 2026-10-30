//! The Inventory end-to-end smoke test: the Desktop's real start-up on a temporary SQLite file.
//! Rename a Location holding Items, add a Location with an Area, restart, see both on the
//! Inventory page with their Areas and Items read from the Inventory adapter, and see a Document's
//! Link to a stored Item still resolve. Run with the other smoke tests at a parent ticket's close
//! and before a merge to `main` (`mise run end-to-end-ui smoke`).

use super::common;
use bin_desktop::shell::router::SettingsInventorySnapshot;

use common::{Harness, config_for};
use gpui::TestAppContext;

/// Settings on the Inventory page (nine presses down the index) with focus stepped into it.
fn in_inventory(ui: &mut Harness<'_>) {
    ui.press("g s");
    ui.press("j j j j j j j j j");
    assert_eq!(ui.settings_frame().page, "Inventory");
    ui.press("l");
}

/// Each Location's name, its Areas, and each Area's Item count, as the page shows them.
type Shown = (
    Vec<(String, String, Option<String>)>,
    Vec<(String, Vec<String>)>,
    Vec<(String, usize)>,
);

fn shown(ui: &mut Harness<'_>) -> Shown {
    let page = ui.settings::<SettingsInventorySnapshot>();
    (
        page.inventory_locations,
        page.inventory_areas,
        page.inventory_area_items,
    )
}

#[gpui::test]
fn a_location_with_items_and_an_added_one_are_there_after_a_restart(app: &mut TestAppContext) {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let config = config_for(&dir.path().join("ledger.pldb"));

    let before = {
        let storage = bin_desktop::start_on(&config, common::today()).expect("a new Ledger starts");
        let writer = storage.writer.clone();
        let mut ui = Harness::with_storage(app, storage);
        in_inventory(&mut ui);
        // 12 Elm St, the first Location, holds the stub's Items.
        ui.press("e x enter");
        assert_eq!(ui.dialog(), None, "the Edit Dialog confirmed");
        ui.press("n c a b i n enter");
        assert_eq!(ui.dialog(), None, "the Add Dialog confirmed");
        ui.press("a d e c k enter");
        assert_eq!(ui.dialog(), None, "the Add Area Dialog confirmed");
        ui.settle_writes();

        let before = shown(&mut ui);
        assert!(
            before
                .0
                .iter()
                .any(|(name, ..)| name == "12 Elm St contentsx")
        );
        assert!(
            before
                .1
                .iter()
                .any(|(location, areas)| location == "cabin" && areas == &["deck"])
        );
        bin_desktop::quit(&writer);
        before
    };

    let storage = bin_desktop::start_on(&config, common::today()).expect("the Ledger starts again");
    let mut ui = Harness::with_storage(app, storage);
    in_inventory(&mut ui);
    let after = shown(&mut ui);
    assert_eq!(after, before);
    assert!(
        after.2.iter().any(|(_, items)| *items > 0),
        "the renamed Location's Areas still hold its Items"
    );

    // The stub Documents link the Fridge, an Item of 12 Elm St, by id: the Link still resolves to
    // the stored Item.
    ui.press("g f");
    let rows = ui.documents().library_rows;
    let linked = (0..rows).any(|_| {
        let found = ui
            .documents()
            .selected_chips
            .iter()
            .any(|chip| chip == "Fridge");
        ui.press("j");
        found
    });
    assert!(linked, "a Document's Link names the stored Fridge");
}
