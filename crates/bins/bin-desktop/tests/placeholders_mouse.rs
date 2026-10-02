//! The "not yet built" placeholder nouns reached by clicking their primary rail rows.

mod common;

use bin_desktop::nav::Noun;
use common::Harness;
use gpui::TestAppContext;

const PLACEHOLDERS: [Noun; 7] = [
    Noun::Notifications,
    Noun::Cash,
    Noun::Inventory,
    Noun::Loans,
    Noun::CreditCards,
    Noun::Investments,
    Noun::Reports,
];

#[gpui::test]
fn clicking_each_placeholder_rail_row_shows_its_placeholder(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);

    for noun in PLACEHOLDERS {
        ui.click(&format!("primary-rail-{noun:?}"));
        assert_eq!(ui.noun(), noun);
        assert!(
            ui.cx.debug_bounds("placeholder-view").is_some(),
            "{noun:?} draws the placeholder view"
        );
    }
}
