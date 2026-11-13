//! The `?` Help Dialog's Presenter (#1025): the shortcut lists the card shows, worded from the
//! Shell's keymap and the open View's page group when the Dialog opens, so a key rebound in
//! `[keybindings]` shows rebound for as long as the card is up. It is read-only: `?`, `Enter`, the
//! Close button and `Esc` close it, and every other key is left to the Shell.
//!
//! The project facts (description, links, licence, copyright) are the Client's to draw, since
//! they are links and styling rather than state. The Client also names a [`Noun`], as the Rail
//! shows it, so the jump list reads the same labels.

use lib_locale::msg;
use lib_viper::dialog::{DialogInput, DialogRouter};
use lib_viper::keymap::{BACK, Keymap, MOVE_DOWN, MOVE_UP, SELECT};
use lib_viper::presenter::{DialogPresenter, Presenter};

use crate::keymap::{GLOBAL_BINDINGS, HELP, OPEN_COMMAND_POPUP, ShellCommand};
use crate::navigation::Noun;

/// A View's own keys in the Help Dialog, under the View's heading: `keys` as `(action, keys)`,
/// read from the hints its status line shows so a rebound key shows rebound.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelpGroup {
    pub heading: String,
    pub keys: Vec<(String, String)>,
}

/// What the Help card shows of the Shell's keys: `(action, keys)` rows, laid out row-major in two
/// columns, and the open View's page group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelpViewModel {
    /// The `g`-jump cheat-sheet, one row for each Noun in the global keymap's own order.
    pub jumps: Vec<(String, String)>,
    /// The shortcuts that aren't jumps.
    pub globals: Vec<(String, String)>,
    /// The View the card opened over adds its own keys; `None` for one that adds none.
    pub group: Option<HelpGroup>,
}

/// The Help Dialog's Presenter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelpPresenter {
    view_model: HelpViewModel,
}

impl HelpPresenter {
    /// Opens over a View, wording every shortcut from `keymap` now. `noun_label` is the Client's
    /// name for a Noun, a Message the Shell doesn't own.
    pub fn open(
        keymap: &Keymap<ShellCommand>,
        noun_label: fn(Noun) -> String,
        group: Option<HelpGroup>,
    ) -> Self {
        Self {
            view_model: HelpViewModel {
                jumps: jump_shortcuts(keymap, noun_label),
                globals: global_shortcuts(keymap),
                group: group.filter(|group| !group.keys.is_empty()),
            },
        }
    }
}

/// What the Help card takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelpEvent {
    /// A Dialog command or typed key: `?` and `Enter` close the card, every other key is left to
    /// the Shell.
    Input(DialogInput),
    /// The Close button.
    Confirm,
    /// `Esc`, the Shell's `back`.
    Cancel,
}

impl Presenter for HelpPresenter {
    type Event = HelpEvent;
    type ViewModel = HelpViewModel;
    type Router<'r> = dyn DialogRouter<()> + 'r;

    fn handle(&mut self, event: HelpEvent, router: &mut Self::Router<'_>) {
        match event {
            HelpEvent::Input(DialogInput::Char('?') | DialogInput::Select) | HelpEvent::Confirm => {
                router.close(Some(()));
            }
            HelpEvent::Input(_) => {}
            HelpEvent::Cancel => router.close(None),
        }
    }

    fn view_model(&self) -> HelpViewModel {
        self.view_model.clone()
    }
}

impl DialogPresenter for HelpPresenter {
    type Outcome = ();

    fn input_event(input: DialogInput) -> HelpEvent {
        HelpEvent::Input(input)
    }

    fn confirm_event() -> HelpEvent {
        HelpEvent::Confirm
    }

    fn cancel_event() -> HelpEvent {
        HelpEvent::Cancel
    }

    fn can_confirm(&self) -> bool {
        true
    }
}

/// The `g`-jump cheat-sheet, as `(noun label, "g x")`, in the global keymap's own order. Each key
/// is the one `keymap` binds now, so a `[keybindings]` override shows here as it does in the
/// status line.
fn jump_shortcuts(
    keymap: &Keymap<ShellCommand>,
    noun_label: fn(Noun) -> String,
) -> Vec<(String, String)> {
    GLOBAL_BINDINGS
        .iter()
        .filter_map(|binding| match binding.event {
            ShellCommand::Jump(noun) => Some((noun_label(noun), bound(keymap, binding.command))),
            _ => None,
        })
        .collect()
}

/// The non-jump shortcuts, as `(action, keys)`, each key read from `keymap`.
fn global_shortcuts(keymap: &Keymap<ShellCommand>) -> Vec<(String, String)> {
    let key = |command| bound(keymap, command);
    vec![
        (msg::help_shortcut_palette(), key(OPEN_COMMAND_POPUP)),
        (msg::help_shortcut_help(), key(HELP)),
        (
            msg::help_shortcut_focus(),
            format!("{}/{}", key(MOVE_DOWN), key(MOVE_UP)),
        ),
        (msg::help_shortcut_confirm(), key(SELECT)),
        (msg::help_shortcut_close(), key(BACK)),
    ]
}

/// The chord `command` is bound to, empty when `[keybindings]` unbinds it.
fn bound(keymap: &Keymap<ShellCommand>, command: &str) -> String {
    keymap.hint_chord(command).to_string()
}

#[cfg(test)]
mod tests {
    use lib_config::KeyBindingConfig;
    use lib_toast::{Notice, Notify};

    use super::*;
    use crate::keymap::global_keymap;

    /// A Router recording how the Dialog closed.
    #[derive(Default)]
    struct Closing {
        closed: Vec<Option<()>>,
    }

    impl Notify for Closing {
        fn notify(&mut self, _notice: Notice) {}
    }

    impl DialogRouter<()> for Closing {
        fn close(&mut self, outcome: Option<()>) {
            self.closed.push(outcome);
        }
    }

    fn name(noun: Noun) -> String {
        format!("{noun:?}")
    }

    fn group() -> HelpGroup {
        HelpGroup {
            heading: "Bills keys".to_string(),
            keys: vec![("Pay".to_string(), "p".to_string())],
        }
    }

    fn open(config: &KeyBindingConfig, group: Option<HelpGroup>) -> HelpPresenter {
        HelpPresenter::open(&global_keymap(config), name, group)
    }

    #[test]
    fn opening_over_a_view_with_a_page_group_carries_it() {
        let help = open(&KeyBindingConfig::default(), Some(group()));

        assert_eq!(help.view_model().group, Some(group()));
    }

    #[test]
    fn opening_over_a_view_without_a_page_group_or_with_no_keys_shows_none() {
        let without = open(&KeyBindingConfig::default(), None);
        let empty = open(
            &KeyBindingConfig::default(),
            Some(HelpGroup {
                heading: "Dashboard keys".to_string(),
                keys: Vec::new(),
            }),
        );

        assert_eq!(without.view_model().group, None);
        assert_eq!(
            empty.view_model().group,
            None,
            "an empty group draws nothing"
        );
    }

    #[test]
    fn the_jump_list_covers_every_noun_in_the_keymaps_order() {
        let jumps = open(&KeyBindingConfig::default(), None)
            .view_model()
            .jumps
            .clone();

        assert_eq!(jumps.len(), Noun::ALL.len());
        assert_eq!(jumps[0], ("Dashboard".to_string(), "g d".to_string()));
        assert!(jumps.contains(&("Transactions".to_string(), "g l".to_string())));
        assert!(jumps.contains(&("CreditCards".to_string(), "g k".to_string())));
    }

    #[test]
    fn the_cheat_sheet_shows_the_keys_keybindings_rebinds() {
        let mut config = KeyBindingConfig::default();
        for (command, chord) in [
            ("jump_dashboard", "g h"),
            ("help", "f1"),
            ("move_down", "down"),
            ("back", "q"),
        ] {
            config
                .bindings
                .insert(command.to_string(), chord.to_string());
        }
        let help = open(&config, None);
        let model = help.view_model();

        assert_eq!(model.jumps[0], ("Dashboard".to_string(), "g h".to_string()));
        assert!(
            model
                .globals
                .contains(&(msg::help_shortcut_help(), "f1".to_string()))
        );
        assert!(
            model
                .globals
                .contains(&(msg::help_shortcut_close(), "q".to_string()))
        );
        assert!(model.globals.iter().any(|(_, keys)| keys == "down/k"));
    }

    #[test]
    fn the_default_cheat_sheet_lists_help_and_close() {
        let help = open(&KeyBindingConfig::default(), None);
        let globals = &help.view_model().globals;

        assert!(globals.contains(&(msg::help_shortcut_help(), "?".to_string())));
        assert!(globals.contains(&(msg::help_shortcut_close(), "esc".to_string())));
        assert!(globals.contains(&(msg::help_shortcut_confirm(), "enter".to_string())));
    }

    #[test]
    fn the_words_are_fixed_when_the_card_opens() {
        let defaults = KeyBindingConfig::default();
        let before = open(&defaults, None);
        let config = defaults.and("help", "f1");

        assert!(
            before
                .view_model()
                .globals
                .contains(&(msg::help_shortcut_help(), "?".to_string()))
        );
        assert_eq!(config.bindings.get("help").map(String::as_str), Some("f1"));
    }

    #[test]
    fn question_mark_and_enter_close_it() {
        for input in [DialogInput::Char('?'), DialogInput::Select] {
            let mut help = open(&KeyBindingConfig::default(), None);
            let mut router = Closing::default();

            help.handle(HelpEvent::Input(input), &mut router);

            assert_eq!(router.closed, [Some(())], "{input:?}");
        }
    }

    #[test]
    fn the_close_button_confirms_and_escape_cancels() {
        let mut help = open(&KeyBindingConfig::default(), None);
        let mut router = Closing::default();

        help.handle(HelpEvent::Confirm, &mut router);
        help.handle(HelpEvent::Cancel, &mut router);

        assert_eq!(router.closed, [Some(()), None]);
    }

    #[test]
    fn every_other_key_is_left_to_the_shell() {
        let mut help = open(&KeyBindingConfig::default(), Some(group()));
        let before = help.clone();
        let mut router = Closing::default();

        for input in [
            DialogInput::Char('x'),
            DialogInput::Char('/'),
            DialogInput::NextField,
            DialogInput::SelectNext,
            DialogInput::Submit,
            DialogInput::Backspace,
            DialogInput::Left,
        ] {
            help.handle(HelpEvent::Input(input), &mut router);
        }

        assert!(router.closed.is_empty());
        assert_eq!(help, before);
    }
}
