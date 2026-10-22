//! A gpui keystroke as a VIPER module's keymap names it: the chord text `[keybindings]` writes,
//! whether the Shell keeps it, and the `&'static str` the status line's hints take.

use std::cell::RefCell;
use std::collections::HashSet;

use gpui::Keystroke;

use super::key_router::is_global_key;

/// `keystroke` as `[keybindings]` writes a chord (`ctrl+d`, `G`, `tab`), or `None` for one the
/// keymap can't name.
pub(crate) fn chord(keystroke: &Keystroke) -> Option<String> {
    let modifiers = &keystroke.modifiers;
    if modifiers.platform || modifiers.function {
        return None;
    }
    let key = keystroke.key.as_str();
    let letter = key.len() == 1 && key.chars().all(|ch| ch.is_ascii_alphabetic());
    let mut chord = String::new();
    if modifiers.control {
        chord.push_str("ctrl+");
    }
    if modifiers.alt {
        chord.push_str("alt+");
    }
    // A shifted letter is written as its capital, as `G` is.
    if modifiers.shift && letter && !modifiers.control && !modifiers.alt {
        chord.push_str(&key.to_ascii_uppercase());
        return Some(chord);
    }
    if modifiers.shift {
        chord.push_str("shift+");
    }
    chord.push_str(key);
    Some(chord)
}

/// Whether the Shell keeps `keystroke` ([`is_global_key`]), so a module keymap never shadows it.
pub(crate) fn is_global(keystroke: &Keystroke) -> bool {
    let modifiers = &keystroke.modifiers;
    is_global_key(&keystroke.key, modifiers.control, modifiers.shift)
}

thread_local! {
    static CHORDS: RefCell<HashSet<&'static str>> = RefCell::new(HashSet::new());
}

/// `chord` as the `&'static str` the status line's hints take. Each distinct chord is leaked
/// once: the keymap has a fixed set, so this stays a handful of strings for the process's life.
pub(crate) fn interned(chord: &str) -> &'static str {
    CHORDS.with(|chords| {
        let mut chords = chords.borrow_mut();
        if let Some(found) = chords.get(chord) {
            return *found;
        }
        let leaked: &'static str = Box::leak(chord.to_string().into_boxed_str());
        chords.insert(leaked);
        leaked
    })
}

#[cfg(test)]
mod tests {
    use gpui::Modifiers;

    use super::*;

    fn keystroke(key: &str, modifiers: Modifiers) -> Keystroke {
        Keystroke {
            modifiers,
            key: key.to_string(),
            key_char: None,
        }
    }

    #[test]
    fn a_keystroke_becomes_the_chord_keybindings_writes() {
        let none = Modifiers::default();
        let shift = Modifiers {
            shift: true,
            ..Modifiers::default()
        };
        let ctrl = Modifiers {
            control: true,
            ..Modifiers::default()
        };
        assert_eq!(chord(&keystroke("d", none)).as_deref(), Some("d"));
        assert_eq!(chord(&keystroke("g", shift)).as_deref(), Some("G"));
        assert_eq!(chord(&keystroke("d", ctrl)).as_deref(), Some("ctrl+d"));
        assert_eq!(chord(&keystroke("tab", none)).as_deref(), Some("tab"));
        assert_eq!(
            chord(&keystroke("tab", shift)).as_deref(),
            Some("shift+tab")
        );
    }

    #[test]
    fn the_shells_global_keys_stay_global() {
        let none = Modifiers::default();
        for key in ["b", "a", "g", ":", "/"] {
            assert!(is_global(&keystroke(key, none)), "{key}");
        }
        for key in ["j", "d", "n", "tab", "t"] {
            assert!(!is_global(&keystroke(key, none)), "{key}");
        }
    }

    #[test]
    fn a_chord_is_interned_once() {
        assert!(std::ptr::eq(interned("ctrl+x"), interned("ctrl+x")));
    }
}
