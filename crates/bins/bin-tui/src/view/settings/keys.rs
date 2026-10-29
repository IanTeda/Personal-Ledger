//! A Settings page's key table (#898): one [`Binding`] per row of its hint bar, holding the keys
//! that row names, how the bar names them, the hint's Message and the command the keys run. The
//! page's `handle_key` looks a key up in its table and its hint bar is built from the same rows,
//! so a key can't be shown without working, or work without being shown.
//!
//! A row's command may still read the key, where one row names several (`j/k` moves by the key
//! pressed). The paging tail (`[/] pages · p picker · esc settings`) is the Settings View's own,
//! not a page's. Settings › Units keeps its `lib-units` Presenter's hints rather than joining a
//! table: its `lib-viper` Keymap is already the one table the Presenter dispatches on and builds
//! its hints from.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// One key a table row takes. Shift is never part of it: an upper-case character carries it
/// already, and terminals differ on whether they report it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Press {
    code: KeyCode,
    ctrl: bool,
}

impl Press {
    pub(super) const fn key(code: KeyCode) -> Self {
        Self { code, ctrl: false }
    }

    pub(super) const fn char(ch: char) -> Self {
        Self::key(KeyCode::Char(ch))
    }

    pub(super) const fn ctrl(ch: char) -> Self {
        Self {
            code: KeyCode::Char(ch),
            ctrl: true,
        }
    }

    fn matches(self, key: KeyEvent) -> bool {
        let modifiers = key.modifiers - KeyModifiers::SHIFT;
        let wanted = if self.ctrl {
            KeyModifiers::CONTROL
        } else {
            KeyModifiers::NONE
        };
        key.code == self.code && modifiers == wanted
    }
}

/// One row of a page's hint bar and the keys it names.
pub(super) struct Binding<C> {
    pub keys: &'static [Press],
    /// How the hint bar names the keys: command syntax, as a chord is, so not a Message.
    pub label: &'static str,
    pub hint: fn() -> String,
    pub command: C,
}

/// The keys every list page moves its selection with, which its `j/k` row names.
pub(super) const ROW_KEYS: &[Press] = &[
    Press::char('j'),
    Press::char('k'),
    Press::key(KeyCode::Down),
    Press::key(KeyCode::Up),
    Press::key(KeyCode::Home),
    Press::char('G'),
    Press::key(KeyCode::End),
];

/// The command of the row in `table` that takes `key`, or `None` when no row does.
pub(super) fn lookup<C: Copy>(table: &[Binding<C>], key: KeyEvent) -> Option<C> {
    table
        .iter()
        .find(|binding| binding.keys.iter().any(|press| press.matches(key)))
        .map(|binding| binding.command)
}

/// The hint bar `table` draws, a `(keys, hint)` pair per row.
pub(super) fn hints<C>(table: &[Binding<C>]) -> Vec<(String, String)> {
    table
        .iter()
        .map(|binding| (binding.label.to_string(), (binding.hint)()))
        .collect()
}

/// The `(keys, hint)` of each row in `table` that runs one of `commands`, in `commands`' order:
/// the keys a detail pane names under a group ([`super::pane`]), so it names only keys that work.
pub(super) fn named<C: PartialEq>(table: &[Binding<C>], commands: &[C]) -> Vec<(String, String)> {
    commands
        .iter()
        .filter_map(|command| table.iter().find(|binding| binding.command == *command))
        .map(|binding| (binding.label.to_string(), (binding.hint)()))
        .collect()
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::view::Action;

    /// Every key a page could be handed: each printable character, `ctrl` with each letter, and
    /// the named keys a terminal sends.
    fn every_key() -> Vec<KeyEvent> {
        let named = [
            KeyCode::Enter,
            KeyCode::Tab,
            KeyCode::BackTab,
            KeyCode::Backspace,
            KeyCode::Delete,
            KeyCode::Esc,
            KeyCode::Left,
            KeyCode::Right,
            KeyCode::Up,
            KeyCode::Down,
            KeyCode::Home,
            KeyCode::End,
            KeyCode::PageUp,
            KeyCode::PageDown,
        ];
        (' '..='~')
            .map(|ch| {
                let modifiers = if ch.is_ascii_uppercase() {
                    KeyModifiers::SHIFT
                } else {
                    KeyModifiers::NONE
                };
                KeyEvent::new(KeyCode::Char(ch), modifiers)
            })
            .chain(('a'..='z').map(|ch| KeyEvent::new(KeyCode::Char(ch), KeyModifiers::CONTROL)))
            .chain(named.map(|code| KeyEvent::new(code, KeyModifiers::NONE)))
            .collect()
    }

    /// Fails if the page's hint bar isn't `table`'s, if a key a row names isn't handled, or if a
    /// key is handled that no row names. `handle` gives each key to a fresh page, so one key's
    /// overlay or selection can't change what the next does.
    pub(in crate::view::settings) fn assert_the_hints_are_the_keys<C: Copy>(
        table: &[Binding<C>],
        hints: Vec<(String, String)>,
        handle: impl Fn(KeyEvent) -> Option<Action>,
    ) {
        crate::locale::init_for_tests();
        assert_eq!(
            hints,
            super::hints(table),
            "the hint bar isn't the key table"
        );
        for binding in table {
            for press in binding.keys {
                let modifiers = if press.ctrl {
                    KeyModifiers::CONTROL
                } else {
                    KeyModifiers::NONE
                };
                let key = KeyEvent::new(press.code, modifiers);
                assert!(
                    handle(key).is_some(),
                    "`{}` ({key:?}) is in the hint bar but isn't handled",
                    binding.label
                );
            }
        }
        for key in every_key() {
            if handle(key).is_some() {
                assert!(
                    lookup(table, key).is_some(),
                    "{key:?} is handled but isn't in the hint bar"
                );
            }
        }
    }

    #[test]
    fn a_press_ignores_shift_but_not_ctrl() {
        let shifted = KeyEvent::new(KeyCode::Char('G'), KeyModifiers::SHIFT);
        assert!(Press::char('G').matches(shifted));
        assert!(Press::char('G').matches(KeyEvent::new(KeyCode::Char('G'), KeyModifiers::NONE)));
        let ctrl_r = KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL);
        assert!(!Press::char('r').matches(ctrl_r));
        assert!(Press::ctrl('r').matches(ctrl_r));
        assert!(!Press::ctrl('r').matches(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE)));
    }
}
