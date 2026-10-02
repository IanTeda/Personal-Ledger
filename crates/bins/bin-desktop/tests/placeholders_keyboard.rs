//! The "not yet built" placeholder nouns reached by `g` jumps: each lands on its noun and draws
//! the placeholder view. Assertions read the Noun back from `Shell`, never pixels.

mod common;

use bin_desktop::nav::Noun;
use common::Harness;
use gpui::TestAppContext;

const PLACEHOLDERS: [(&str, Noun); 7] = [
    ("a", Noun::Notifications),
    ("c", Noun::Cash),
    ("o", Noun::Inventory),
    ("n", Noun::Loans),
    ("k", Noun::CreditCards),
    ("i", Noun::Investments),
    ("r", Noun::Reports),
];

#[gpui::test]
fn each_placeholder_noun_is_reachable_by_a_g_jump_and_renders(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);

    for (key, noun) in PLACEHOLDERS {
        ui.press(&format!("g {key}"));
        assert_eq!(ui.noun(), noun, "g {key}");
        assert!(
            ui.cx.debug_bounds("placeholder-view").is_some(),
            "{noun:?} draws the placeholder view"
        );
    }
}

#[gpui::test]
fn leaving_a_placeholder_noun_drops_the_placeholder_view(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.press("g c");

    ui.press("g l");

    assert_eq!(ui.noun(), Noun::Transactions);
    assert!(ui.cx.debug_bounds("placeholder-view").is_none());
}
