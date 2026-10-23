//! A gpui keystroke as a keymap names it: the chord text `[keybindings]` writes, which the Shell's
//! global keymap, the Dialog keymap and every module's keymap look keys up by.

use gpui::Keystroke;

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
    // A shifted symbol is written as the symbol it types (`?`, `:`), which already says Shift;
    // some platforms report `?` as Shift and `/`, as a US layout types it.
    let symbol = key.chars().count() == 1 && !letter && !key.chars().all(char::is_alphanumeric);
    if modifiers.shift && symbol {
        chord.push_str(if key == "/" { "?" } else { key });
        return Some(chord);
    }
    if modifiers.shift {
        chord.push_str("shift+");
    }
    // gpui names it `escape`; `[keybindings]` writes `esc`, as `back`'s default does.
    chord.push_str(if key == "escape" { "esc" } else { key });
    Some(chord)
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
        assert_eq!(chord(&keystroke("escape", none)).as_deref(), Some("esc"));
        assert_eq!(
            chord(&keystroke("enter", ctrl)).as_deref(),
            Some("ctrl+enter")
        );
    }

    #[test]
    fn a_shifted_symbol_is_the_symbol_it_types() {
        let shift = Modifiers {
            shift: true,
            ..Modifiers::default()
        };
        assert_eq!(chord(&keystroke("?", shift)).as_deref(), Some("?"));
        assert_eq!(chord(&keystroke("/", shift)).as_deref(), Some("?"));
        assert_eq!(chord(&keystroke(":", shift)).as_deref(), Some(":"));
        assert_eq!(
            chord(&keystroke("/", Modifiers::default())).as_deref(),
            Some("/")
        );
        assert_eq!(chord(&keystroke("1", shift)).as_deref(), Some("shift+1"));
    }
}
