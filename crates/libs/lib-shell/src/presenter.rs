//! The Shell's Presenter. It holds the Clock and the last calendar month it saw, and tells its
//! Router once the Clock reads a new one. It resolves each key the Client offers it through the
//! global keymap, holding a `g` prefix until the next key or the Client's timer ends it. The
//! Client's own inputs, a Notice, a legacy write and a failed one, pass through it to the Router
//! until the Presenter holds what they change (#826).

use lib_config::KeyBindingConfig;
use lib_core::{Clock, Period};
use lib_toast::Notice;
use lib_viper::keymap::Keymap;

use crate::keymap::{ShellCommand, global_keymap};
use crate::protocols::ShellRouter;

/// What the Client delivers to the Shell, each through its one `send`. `F` is the Client's write
/// failure ([`ShellRouter::Failure`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellInput<F> {
    /// The Client's day poll fired: read the Clock for a new month.
    DayTick,
    /// A View not yet a module raised a Toast or set the status line.
    Notice(Notice),
    /// A View not yet a module ran a write on the Client's legacy store, outside any input the
    /// Shell saw. The write has run; this input exists so the `send` it arrives in commits it,
    /// the one commit path (#823). It goes when the last such View migrates.
    LegacyApply,
    /// A committed write failed.
    WriteFailed(F),
    /// The Client's timer ran out on a prefix (the `g` of `g d`) with no second key: drop it.
    PrefixExpired,
}

/// What the Shell made of a key the Client offered it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellKey {
    /// The Shell took the key: it handed its Router a command, or finished or dropped a prefix.
    Taken,
    /// The key began a sequence. The Client starts the timer that ends the wait with
    /// [`ShellInput::PrefixExpired`].
    Prefixed,
    /// Not the Shell's to take first. The focused module or View may take `chord`, a whole sequence
    /// (`g g`) when the key finished one; once both decline, `fallback` is the Shell's command for
    /// it, if it has one.
    Passed {
        chord: String,
        fallback: Option<ShellCommand>,
    },
}

/// A Dialog opening or closing over the View. Neither asks the Router anything, so the Client
/// delivers them through [`ShellPresenter::dialog`] wherever a Dialog opens, with no Router to hand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogInput {
    /// A Dialog opened: until it closes it takes every key.
    Opened,
    /// The open Dialog closed.
    Closed,
}

pub struct ShellPresenter {
    clock: Clock,
    /// The calendar month the Shell last saw the Clock read.
    month: Period,
    dialog_open: bool,
    keymap: Keymap<ShellCommand>,
    /// The first key of a sequence typed so far, until the next key or [`ShellInput::PrefixExpired`].
    prefix: Option<String>,
}

impl ShellPresenter {
    /// The Shell on the month `clock` reads now, which the Client generated the Schedule for as
    /// it started.
    pub fn new(clock: Clock) -> Self {
        let month = Period::of(clock.today());
        Self {
            clock,
            month,
            dialog_open: false,
            keymap: global_keymap(&KeyBindingConfig::default()),
            prefix: None,
        }
    }

    /// Puts the global keymap under `config`'s overrides.
    pub fn set_keymap(&mut self, config: &KeyBindingConfig) {
        self.keymap = global_keymap(config);
    }

    pub fn keymap(&self) -> &Keymap<ShellCommand> {
        &self.keymap
    }

    /// Resolves `chord`, a key as `[keybindings]` writes it, through the global keymap. A key
    /// after a prefix finishes the sequence: a kept command runs, one the Shell falls back on is
    /// passed on whole, and an unbound one is no jump. `back` after a prefix only drops it.
    pub fn key<R: ShellRouter + ?Sized>(&mut self, chord: &str, router: &mut R) -> ShellKey {
        if let Some(prefix) = self.prefix.take() {
            if self.keymap.event(chord) == Some(ShellCommand::Back) {
                return ShellKey::Taken;
            }
            let sequence = format!("{prefix} {chord}");
            if self.keymap.event(&sequence).is_none() {
                router.not_a_jump(sequence);
                return ShellKey::Taken;
            }
            return self.resolve(sequence, router);
        }
        if self.keymap.is_prefix(chord) {
            self.prefix = Some(chord.to_string());
            return ShellKey::Prefixed;
        }
        self.resolve(chord.to_string(), router)
    }

    fn resolve<R: ShellRouter + ?Sized>(&self, chord: String, router: &mut R) -> ShellKey {
        match self.keymap.event(&chord) {
            Some(command) if !command.falls_back() => {
                router.run(command);
                ShellKey::Taken
            }
            fallback => ShellKey::Passed { chord, fallback },
        }
    }

    /// Whether a Dialog is open, so the Client hands it every key.
    pub fn dialog_open(&self) -> bool {
        self.dialog_open
    }

    /// A Dialog opening also drops a waiting prefix, since the Dialog takes the next key.
    pub fn dialog(&mut self, input: DialogInput) {
        self.dialog_open = input == DialogInput::Opened;
        self.prefix = None;
    }

    pub fn handle<R: ShellRouter + ?Sized>(
        &mut self,
        input: ShellInput<R::Failure>,
        router: &mut R,
    ) {
        match input {
            ShellInput::DayTick => {
                let today = self.clock.today();
                if Period::of(today) != self.month {
                    self.month = Period::of(today);
                    router.month_turned(today);
                }
            }
            ShellInput::Notice(notice) => router.notify(notice),
            ShellInput::LegacyApply => {}
            ShellInput::WriteFailed(failure) => router.write_failed(failure),
            ShellInput::PrefixExpired => self.prefix = None,
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use lib_list_cursor::Step;

    use super::*;
    use crate::Noun;
    use crate::fake::{RecordingRouter, Routed};
    use crate::keymap::{GLOBAL_BINDINGS, TOGGLE_RAIL};

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).expect("a real date")
    }

    #[test]
    fn a_day_tick_in_the_same_month_calls_nothing() {
        let (clock, day) = Clock::settable(date(2026, 9, 19));
        let mut shell = ShellPresenter::new(clock);
        let mut router = RecordingRouter::default();

        day.set(date(2026, 9, 30));
        shell.handle(ShellInput::DayTick, &mut router);

        assert!(router.routed.is_empty());
    }

    #[test]
    fn a_day_tick_in_a_new_month_turns_it_once_with_the_new_day() {
        let (clock, day) = Clock::settable(date(2026, 9, 30));
        let mut shell = ShellPresenter::new(clock);
        let mut router = RecordingRouter::default();

        day.set(date(2026, 10, 1));
        shell.handle(ShellInput::DayTick, &mut router);
        shell.handle(ShellInput::DayTick, &mut router);

        assert_eq!(router.routed, vec![Routed::MonthTurned(date(2026, 10, 1))]);
    }

    #[test]
    fn a_notice_and_a_write_failure_go_to_the_router_and_a_legacy_write_asks_nothing() {
        let mut shell = ShellPresenter::new(Clock::fixed(date(2026, 9, 19)));
        let mut router = RecordingRouter::default();
        let saved = Notice::Toast(lib_toast::ToastKind::Success, "Saved".to_string());

        shell.handle(ShellInput::Notice(saved.clone()), &mut router);
        shell.handle(ShellInput::LegacyApply, &mut router);
        shell.handle(
            ShellInput::WriteFailed("disk full".to_string()),
            &mut router,
        );

        assert_eq!(
            router.routed,
            vec![
                Routed::Notice(saved),
                Routed::WriteFailed("disk full".to_string())
            ]
        );
    }

    #[test]
    fn a_dialog_is_open_from_its_opening_to_its_closing() {
        let mut shell = ShellPresenter::new(Clock::fixed(date(2026, 9, 19)));
        assert!(!shell.dialog_open());

        shell.dialog(DialogInput::Opened);
        assert!(shell.dialog_open());

        shell.dialog(DialogInput::Closed);
        assert!(!shell.dialog_open());
    }

    #[test]
    fn a_day_tick_in_the_same_month_of_another_year_turns_it() {
        let (clock, day) = Clock::settable(date(2026, 9, 30));
        let mut shell = ShellPresenter::new(clock);
        let mut router = RecordingRouter::default();

        day.set(date(2027, 9, 1));
        shell.handle(ShellInput::DayTick, &mut router);

        assert_eq!(router.routed, vec![Routed::MonthTurned(date(2027, 9, 1))]);
    }

    fn shell() -> ShellPresenter {
        ShellPresenter::new(Clock::fixed(date(2026, 9, 19)))
    }

    /// Offers `chords` in turn, returning the last answer and what the Router was asked.
    fn press(shell: &mut ShellPresenter, chords: &[&str]) -> (ShellKey, Vec<Routed>) {
        let mut router = RecordingRouter::default();
        let mut answer = ShellKey::Taken;
        for chord in chords {
            answer = shell.key(chord, &mut router);
        }
        (answer, router.routed)
    }

    fn passed(chord: &str, fallback: Option<ShellCommand>) -> ShellKey {
        ShellKey::Passed {
            chord: chord.to_string(),
            fallback,
        }
    }

    #[test]
    fn each_kept_command_runs_on_its_default_chord() {
        for binding in GLOBAL_BINDINGS {
            if binding.event.falls_back() || binding.chord.is_empty() {
                continue;
            }
            let chords: Vec<&str> = binding.chord.split(' ').collect();
            let (answer, routed) = press(&mut shell(), &chords);
            assert_eq!(answer, ShellKey::Taken, "{}", binding.command);
            assert_eq!(
                routed,
                vec![Routed::Ran(binding.event)],
                "{}",
                binding.command
            );
        }
    }

    #[test]
    fn each_fallback_command_is_passed_on_with_its_whole_chord() {
        for binding in GLOBAL_BINDINGS.iter().filter(|b| b.event.falls_back()) {
            let chords: Vec<&str> = binding.chord.split(' ').collect();
            let (answer, routed) = press(&mut shell(), &chords);
            assert_eq!(
                answer,
                passed(binding.chord, Some(binding.event)),
                "{}",
                binding.command
            );
            assert!(routed.is_empty(), "{}", binding.command);
        }
    }

    #[test]
    fn a_key_the_shell_does_not_bind_is_passed_on_with_no_fallback() {
        assert_eq!(press(&mut shell(), &["n"]), (passed("n", None), vec![]));
    }

    #[test]
    fn an_override_moves_a_command_and_frees_its_default() {
        let mut shell = shell();
        let mut config = KeyBindingConfig::default();
        config
            .bindings
            .insert(TOGGLE_RAIL.to_string(), "ctrl+b".to_string());
        shell.set_keymap(&config);

        assert_eq!(
            press(&mut shell, &["ctrl+b"]).1,
            vec![Routed::Ran(ShellCommand::ToggleRail)]
        );
        assert_eq!(press(&mut shell, &["b"]), (passed("b", None), vec![]));
    }

    #[test]
    fn g_waits_for_the_next_key_and_completes_a_jump() {
        let mut shell = shell();

        assert_eq!(press(&mut shell, &["g"]), (ShellKey::Prefixed, vec![]));
        assert_eq!(
            press(&mut shell, &["s"]),
            (
                ShellKey::Taken,
                vec![Routed::Ran(ShellCommand::Jump(Noun::Settings))]
            )
        );
        assert_eq!(
            press(&mut shell, &["s"]),
            (passed("s", None), vec![]),
            "the jump used the prefix up"
        );
    }

    #[test]
    fn g_then_an_unbound_key_is_not_a_jump_and_consumes_the_key() {
        let mut shell = shell();

        assert_eq!(
            press(&mut shell, &["g", "x"]),
            (ShellKey::Taken, vec![Routed::NotAJump("g x".to_string())])
        );
        assert_eq!(press(&mut shell, &["x"]), (passed("x", None), vec![]));
    }

    #[test]
    fn back_after_g_only_drops_the_prefix() {
        let mut shell = shell();

        assert_eq!(press(&mut shell, &["g", "esc"]), (ShellKey::Taken, vec![]));
        assert_eq!(press(&mut shell, &["l"]), (passed("l", None), vec![]));
    }

    #[test]
    fn prefix_expired_cancels_a_waiting_g() {
        let mut shell = shell();
        let mut router = RecordingRouter::default();

        press(&mut shell, &["g"]);
        shell.handle(ShellInput::PrefixExpired, &mut router);

        assert!(router.routed.is_empty());
        assert_eq!(
            press(&mut shell, &["l"]),
            (passed("l", None), vec![]),
            "`l` after a lapsed `g` is no jump"
        );
    }

    #[test]
    fn a_dialog_opening_drops_a_waiting_g() {
        let mut shell = shell();

        press(&mut shell, &["g"]);
        shell.dialog(DialogInput::Opened);
        shell.dialog(DialogInput::Closed);

        assert_eq!(press(&mut shell, &["l"]), (passed("l", None), vec![]));
    }

    #[test]
    fn g_g_is_passed_on_whole_for_the_list_to_move_first() {
        assert_eq!(
            press(&mut shell(), &["g", "g"]),
            (passed("g g", Some(ShellCommand::Move(Step::First))), vec![])
        );
    }
}
