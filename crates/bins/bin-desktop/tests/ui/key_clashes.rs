//! No page's default chord is a key the Shell keeps (#1170). The keys are each hosted page's live
//! ones through the one hosting trait, iterated rather than named, so a page added to `Root` is
//! checked without this file hearing of it.

use super::common::Harness;
use gpui::TestAppContext;
use lib_config::KeyBindingConfig;
use lib_shell::keymap::keeps;
use lib_transactions::keymap::RESET_FILTERS;
use lib_viper::keymap::{BACK, PREVIOUS_FIELD, SEARCH};

/// The `(command, chord)` bindings among `bindings` the Shell takes before any page sees them. The
/// Shell keeps `search` (`/`) and `back` (`esc`) and sends them on by name, so a page binding
/// either is no clash. The Settings frame's moves and `select` and the Colour Theme grid's `tab`
/// and half-page moves are the Shell's fall-backs, which `keeps` already lets through. The
/// Transactions filter popover's `previous_field` (`shift+tab`) and `transactions.reset_filters`
/// (`ctrl+r`, beside the Shell's Redo) are a mode the Shell offers its keys to first, so they are
/// no clash either.
fn clashes<'a>(bindings: impl IntoIterator<Item = (&'a str, &'a str)>) -> Vec<(String, String)> {
    let keymap = lib_shell::global_keymap(&KeyBindingConfig::default());
    bindings
        .into_iter()
        .filter(|(command, _)| ![SEARCH, BACK, PREVIOUS_FIELD, RESET_FILTERS].contains(command))
        .filter(|(_, chord)| keeps(&keymap, chord))
        .map(|(command, chord)| (command.to_string(), chord.to_string()))
        .collect()
}

#[gpui::test]
fn no_hosted_page_default_chord_is_a_key_the_shell_keeps(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    let pages = ui.read_app(|shell, cx| shell.page_keys_snapshot(cx));
    assert!(!pages.is_empty());
    for page in &pages {
        let bound = page
            .keys
            .iter()
            .map(|(command, chord)| (*command, chord.as_str()));
        assert_eq!(clashes(bound), [], "{}", page.page);
    }
}

#[gpui::test]
fn a_default_on_a_kept_key_is_a_clash_and_one_the_shell_falls_back_on_is_not(
    _app: &mut TestAppContext,
) {
    let defaults = [
        ("demo.rail", "b"),
        ("demo.dismiss", "ctrl+l"),
        ("demo.back", "shift+tab"),
        ("demo.jump", "g x"),
        ("demo.help", "?"),
        ("demo.undo", "u"),
        ("demo.redo", "ctrl+r"),
        ("demo.insert", "a"),
        ("demo.free", "B"),
        ("demo.page", "ctrl+u"),
        ("demo.tab", "tab"),
        (lib_viper::keymap::MOVE_FIRST, "g g"),
        (SEARCH, "/"),
        (BACK, "esc"),
    ];
    let clashing: Vec<_> = clashes(defaults)
        .into_iter()
        .map(|(_, chord)| chord)
        .collect();
    assert_eq!(
        clashing,
        ["b", "ctrl+l", "shift+tab", "g x", "?", "u", "ctrl+r"]
    );
}
