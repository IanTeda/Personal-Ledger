//! The Shell's Presenter. It holds what the frame shows: the status line, the Toast stack and its
//! history (`lib-toast`'s model), and where the person is ([`NavState`]: the View showing, focus
//! and the input mode). It holds the Clock and the last calendar month it saw, telling its Router
//! once the Clock reads a new one, and resolves each key the Client offers it through the global
//! keymap, holding a `g` prefix until the next key or the Client's timer ends it.
//!
//! It never reads time itself: the Client's timers deliver [`ShellInput::DayTick`] and
//! [`ShellInput::PrefixExpired`], and the Toast timer's time to [`ShellPresenter::tick_toasts`],
//! and the Client stamps each Notice it shows.
//! A status message answers one key: every key clears it as it arrives ([`ShellPresenter::keypress`]),
//! before the key is handled, so one the key sets survives; clicks and ticks leave it.

use std::time::{Duration, Instant};

use chrono::{DateTime, Local};
use lib_config::KeyBindingConfig;
use lib_core::{Clock, Period};
use lib_list_cursor::{HALF_PAGE, Step};
use lib_locale::msg;
use lib_toast::{Display, Notice, Toasts};
use lib_viper::keymap::Keymap;
use lib_viper::view::Hint;

use crate::keymap::{HELP, OPEN_COMMAND_POPUP, SEARCH, ShellCommand, TOGGLE_RAIL, global_keymap};
use crate::nav::{ContextSelection, FocusZone, InputMode, NavState};
use crate::navigation::Noun;
use crate::protocols::ShellRouter;

/// What the Client delivers to the Shell, each through its one `send`. `F` is the Client's write
/// failure ([`ShellRouter::Failure`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellInput<F> {
    /// The Client's day poll fired: read the Clock for a new month.
    DayTick,
    /// A View not yet a module raised a Toast or set the status line. It reaches the Router, which
    /// queues it with every module's Notices for the Client to show ([`ShellPresenter::show`]).
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
    /// A Dialog opened: until it closes it takes every key, in `Dialog` mode.
    Opened,
    /// The open Dialog closed, leaving its mode.
    Closed,
}

/// A change to where the person is: the View, focus or mode. None asks the Router anything, so the
/// Client delivers them through [`ShellPresenter::navigate`] from inside the input that made them.
/// `P` is the Client's Settings page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavInput<P> {
    /// Shows `Noun`, a step in the Back history.
    Show(Noun),
    Back,
    Forward,
    /// The Settings page showing changed, a history step while Settings shows.
    SettingsPage(P),
    /// The launch restored the last Settings page, no history step.
    RestoreSettingsPage(P),
    /// Moves the primary rail's browsed row, leaving the Noun until [`NavInput::CommitHighlight`].
    MoveHighlight(Step),
    /// Shows the primary rail's browsed row.
    CommitHighlight,
    Context(ContextSelection),
    FocusNext,
    FocusPrevious,
    Focus(FocusZone),
    EnterMode(InputMode),
    /// Leaves the mode, restoring the focus from before it.
    ExitMode,
    OpenLedger,
    CloseLedger,
}

/// A change to the Toast stack from a click, a command or a Preference. None asks the Router
/// anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastInput {
    DismissAll,
    DismissNewest,
    /// Dismisses the `index`th Toast on show, top first.
    DismissVisible(usize),
    /// The pointer is over the stack, or has left it: the timers pause while it is there.
    Hover(bool),
    /// The Toasts Preference changed.
    Display(Display),
}

/// What the frame draws from the Shell, as owned values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellViewModel {
    pub mode: InputMode,
    pub focus: FocusZone,
    /// The message answering the last key, which replaces the hints until the next.
    pub status: Option<String>,
    /// The Shell's hints in the order they drop from the end: the palette, Search, Help and the
    /// rail toggle, each under its key now. A command `[keybindings]` unbinds has none. Each keeps
    /// its command, so the Client can tell what a click on it does.
    pub hints: Vec<(ShellCommand, Hint)>,
}

/// A Shell hint's Message: what its key does.
type Label = fn() -> String;

/// The commands the Shell's hints name, with their labels, in the order they drop from the end.
const HINTS: [(&str, ShellCommand, Label); 4] = [
    (OPEN_COMMAND_POPUP, ShellCommand::Command, msg::hint_command),
    (SEARCH, ShellCommand::Search, msg::hint_search),
    (HELP, ShellCommand::Help, msg::hint_help),
    (
        TOGGLE_RAIL,
        ShellCommand::ToggleRail,
        msg::hint_toggle_sidebar,
    ),
];

/// The Shell's Presenter. `P` is the Client's Settings page, which the Back history steps between.
pub struct ShellPresenter<P> {
    clock: Clock,
    /// The calendar month the Shell last saw the Clock read.
    month: Period,
    dialog_open: bool,
    keymap: Keymap<ShellCommand>,
    /// The first key of a sequence typed so far, until the next key or [`ShellInput::PrefixExpired`].
    prefix: Option<String>,
    nav: NavState<P>,
    status: Option<String>,
    /// Stamped with the local time each was raised at, for the history.
    toasts: Toasts<DateTime<Local>>,
    toasts_hovered: bool,
    /// The last [`ShellPresenter::tick_toasts`]' time, which the next advances from. `None` while the
    /// stack is empty, so the first tick after a raise only starts the count.
    last_tick: Option<Instant>,
}

impl<P: Copy + Eq + Default> ShellPresenter<P> {
    /// The Shell on the month `clock` reads now, which the Client generated the Schedule for as
    /// it started, and at `nav`, where the launch restored the person to.
    pub fn new(clock: Clock, nav: NavState<P>) -> Self {
        let month = Period::of(clock.today());
        Self {
            clock,
            month,
            dialog_open: false,
            keymap: global_keymap(&KeyBindingConfig::default()),
            prefix: None,
            nav,
            status: None,
            toasts: Toasts::default(),
            toasts_hovered: false,
            last_tick: None,
        }
    }

    /// Puts the global keymap under `config`'s overrides.
    pub fn set_keymap(&mut self, config: &KeyBindingConfig) {
        self.keymap = global_keymap(config);
    }

    pub fn keymap(&self) -> &Keymap<ShellCommand> {
        &self.keymap
    }

    pub fn nav(&self) -> &NavState<P> {
        &self.nav
    }

    pub fn toasts(&self) -> &Toasts<DateTime<Local>> {
        &self.toasts
    }

    pub fn view_model(&self) -> ShellViewModel {
        ShellViewModel {
            mode: self.nav.mode(),
            focus: self.nav.focus(),
            status: self.status.clone(),
            hints: HINTS
                .iter()
                .filter_map(|&(name, command, label)| {
                    let chord = self.keymap.chord(name).filter(|chord| !chord.is_empty())?;
                    Some((
                        command,
                        Hint {
                            chord: chord.to_string(),
                            action: label(),
                        },
                    ))
                })
                .collect(),
        }
    }

    /// A key arrived, before anything handles it: clears the status message, so the message a key
    /// sets answers that key alone. Returns whether one was showing, which asks for a redraw.
    pub fn keypress(&mut self) -> bool {
        self.status.take().is_some()
    }

    /// Clears the status message at a View's request: a View not yet a module that changed what it
    /// shows from under the message (Budgets' tab). Goes with the last such View.
    pub fn clear_status(&mut self) {
        self.status = None;
    }

    /// Shows a Notice the Client drained after an input, stamped `at` for the Toast history: a
    /// Toast on the stack, or a message on the status line.
    pub fn show(&mut self, notice: Notice, at: DateTime<Local>) {
        match notice {
            Notice::Toast(kind, text) => {
                if self.toasts.visible().is_empty() {
                    self.last_tick = None;
                }
                self.toasts.raise(kind, text, at);
            }
            Notice::Status(text) => self.status = Some(text),
        }
    }

    /// Whether a Toast is up for the Client's timer to tick.
    pub fn toasts_ticking(&self) -> bool {
        !self.toasts.visible().is_empty()
    }

    pub fn toast(&mut self, input: ToastInput) {
        match input {
            ToastInput::DismissAll => self.toasts.dismiss_all(),
            ToastInput::DismissNewest => {
                self.toasts.dismiss_newest();
            }
            ToastInput::DismissVisible(index) => {
                self.toasts.dismiss_visible(index);
            }
            ToastInput::Hover(hovered) => self.toasts_hovered = hovered,
            ToastInput::Display(display) => self.toasts.set_display(display),
        }
    }

    /// Moves the person to another View, zone or mode. A Back or Forward with nowhere to go does
    /// nothing.
    pub fn navigate(&mut self, input: NavInput<P>) {
        let nav = &mut self.nav;
        match input {
            NavInput::Show(noun) => nav.set_noun(noun),
            NavInput::Back => {
                nav.go_back();
            }
            NavInput::Forward => {
                nav.go_forward();
            }
            NavInput::SettingsPage(page) => nav.record_settings_page(page),
            NavInput::RestoreSettingsPage(page) => nav.restore_settings_page(page),
            NavInput::MoveHighlight(step) => match step {
                Step::Next => nav.move_primary_highlight_next(),
                Step::Prev => nav.move_primary_highlight_prev(),
                Step::First => nav.move_primary_highlight_first(),
                Step::Last => nav.move_primary_highlight_last(),
                // The rail has no viewport to page, so half a list's page of rows stands in.
                Step::HalfPageDown => {
                    (0..HALF_PAGE).for_each(|_| nav.move_primary_highlight_next())
                }
                Step::HalfPageUp => (0..HALF_PAGE).for_each(|_| nav.move_primary_highlight_prev()),
            },
            NavInput::CommitHighlight => nav.commit_primary_highlight(),
            NavInput::Context(context) => nav.set_context(context),
            NavInput::FocusNext => nav.cycle_focus_forward(),
            NavInput::FocusPrevious => nav.cycle_focus_backward(),
            NavInput::Focus(zone) => nav.set_focus(zone),
            // The next key belongs to the mode, or to the one left, so a waiting prefix can't
            // finish on it; a Dialog opening drops it the same way.
            NavInput::EnterMode(mode) => {
                self.prefix = None;
                nav.enter_mode(mode);
            }
            NavInput::ExitMode => {
                self.prefix = None;
                nav.exit_mode();
            }
            NavInput::OpenLedger => nav.open_ledger(),
            NavInput::CloseLedger => nav.close_ledger(),
        }
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

    /// Runs a kept command: the Toast keys on the Shell's own stack, the rest by the Router.
    fn resolve<R: ShellRouter + ?Sized>(&mut self, chord: String, router: &mut R) -> ShellKey {
        match self.keymap.event(&chord) {
            Some(ShellCommand::DismissToasts) => {
                self.toasts.dismiss_all();
                ShellKey::Taken
            }
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

    /// A Dialog opening enters `Dialog` mode and drops a waiting prefix, since the Dialog takes the
    /// next key; one closing leaves the mode.
    pub fn dialog(&mut self, input: DialogInput) {
        self.prefix = None;
        match input {
            DialogInput::Opened => {
                self.dialog_open = true;
                self.nav.enter_mode(InputMode::Dialog);
            }
            DialogInput::Closed if self.dialog_open => {
                self.dialog_open = false;
                self.nav.exit_mode();
            }
            DialogInput::Closed => {}
        }
    }

    /// Whether a modal surface is up (the palette or file explorer, a Dialog, a filter), which
    /// pauses the Toast timers.
    fn modal_open(&self) -> bool {
        matches!(
            self.nav.mode(),
            InputMode::Command | InputMode::Dialog | InputMode::Filter
        )
    }

    /// The Client's Toast timer fired at `now`, on its monotonic clock: advances the Toasts by the
    /// time since the last tick, paused while the pointer is over them or a modal is up. Returns
    /// whether what the stack draws changed, the only time the Client need redraw. Not an input:
    /// a tick writes nothing, so it needs no `send`. The Client ticks only while a Toast is up
    /// ([`Self::toasts_ticking`]).
    pub fn tick_toasts(&mut self, now: Instant) -> bool {
        let elapsed = self
            .last_tick
            .replace(now)
            .map_or(Duration::ZERO, |last| now.saturating_duration_since(last));
        if self.toasts.visible().is_empty() {
            // A dismissed stack never reports the pointer leaving it.
            self.toasts_hovered = false;
            self.last_tick = None;
            return false;
        }
        let drawn = |toasts: &Toasts<DateTime<Local>>| {
            (
                toasts.visible().len(),
                toasts.more_count(),
                toasts.echo().is_some(),
            )
        };
        let before = drawn(&self.toasts);
        if self.toasts_hovered || self.modal_open() {
            self.toasts.pause();
        } else {
            self.toasts.resume();
        }
        self.toasts.advance(elapsed);
        drawn(&self.toasts) != before
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

    use lib_toast::ToastKind;

    use super::*;
    use crate::fake::{RecordingRouter, Routed};
    use crate::keymap::GLOBAL_BINDINGS;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).expect("a real date")
    }

    /// A Client with no Settings pages to step between.
    type Shell = ShellPresenter<()>;

    fn presenter(clock: Clock) -> Shell {
        ShellPresenter::new(clock, NavState::new())
    }

    #[test]
    fn a_day_tick_in_the_same_month_calls_nothing() {
        let (clock, day) = Clock::settable(date(2026, 9, 19));
        let mut shell = presenter(clock);
        let mut router = RecordingRouter::default();

        day.set(date(2026, 9, 30));
        shell.handle(ShellInput::DayTick, &mut router);

        assert!(router.routed.is_empty());
    }

    #[test]
    fn a_day_tick_in_a_new_month_turns_it_once_with_the_new_day() {
        let (clock, day) = Clock::settable(date(2026, 9, 30));
        let mut shell = presenter(clock);
        let mut router = RecordingRouter::default();

        day.set(date(2026, 10, 1));
        shell.handle(ShellInput::DayTick, &mut router);
        shell.handle(ShellInput::DayTick, &mut router);

        assert_eq!(router.routed, vec![Routed::MonthTurned(date(2026, 10, 1))]);
    }

    #[test]
    fn a_notice_and_a_write_failure_go_to_the_router_and_a_legacy_write_asks_nothing() {
        let mut shell = presenter(Clock::fixed(date(2026, 9, 19)));
        let mut router = RecordingRouter::default();
        let saved = Notice::Toast(ToastKind::Success, "Saved".to_string());

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
        let mut shell = presenter(Clock::fixed(date(2026, 9, 19)));
        assert!(!shell.dialog_open());

        shell.dialog(DialogInput::Opened);
        assert!(shell.dialog_open());

        shell.dialog(DialogInput::Closed);
        assert!(!shell.dialog_open());
    }

    #[test]
    fn a_day_tick_in_the_same_month_of_another_year_turns_it() {
        let (clock, day) = Clock::settable(date(2026, 9, 30));
        let mut shell = presenter(clock);
        let mut router = RecordingRouter::default();

        day.set(date(2027, 9, 1));
        shell.handle(ShellInput::DayTick, &mut router);

        assert_eq!(router.routed, vec![Routed::MonthTurned(date(2027, 9, 1))]);
    }

    fn shell() -> Shell {
        presenter(Clock::fixed(date(2026, 9, 19)))
    }

    /// Offers `chords` in turn, returning the last answer and what the Router was asked.
    fn press(shell: &mut Shell, chords: &[&str]) -> (ShellKey, Vec<Routed>) {
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
            // The Shell clears its own Toasts.
            if binding.event.falls_back()
                || binding.chord.is_empty()
                || binding.event == ShellCommand::DismissToasts
            {
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
    fn a_mode_change_drops_a_waiting_g() {
        let mut shell = shell();

        press(&mut shell, &["g"]);
        shell.navigate(NavInput::EnterMode(InputMode::Search));
        shell.navigate(NavInput::ExitMode);

        assert_eq!(
            press(&mut shell, &["d"]),
            (passed("d", None), vec![]),
            "`d` after leaving Search is no jump"
        );
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

    fn local(seconds: i64) -> DateTime<Local> {
        DateTime::from_timestamp(seconds, 0)
            .expect("a real time")
            .with_timezone(&Local)
    }

    fn texts(shell: &Shell) -> Vec<String> {
        shell
            .toasts()
            .visible()
            .iter()
            .map(|toast| toast.text().to_string())
            .collect()
    }

    fn toast(kind: ToastKind, text: &str) -> Notice {
        Notice::Toast(kind, text.to_string())
    }

    fn status(text: &str) -> Notice {
        Notice::Status(text.to_string())
    }

    #[test]
    fn a_toast_notice_goes_on_the_stack_and_a_status_notice_on_the_status_line() {
        let mut shell = shell();

        shell.show(toast(ToastKind::Success, "Saved"), local(100));
        shell.show(status("g x is not a jump"), local(100));

        assert_eq!(texts(&shell), ["Saved"]);
        assert_eq!(
            shell
                .toasts()
                .history()
                .iter()
                .next()
                .map(|e| *e.last_raised()),
            Some(local(100)),
            "stamped as the Client said"
        );
        assert_eq!(
            shell.view_model().status.as_deref(),
            Some("g x is not a jump")
        );
    }

    #[test]
    fn a_key_clears_the_status_before_it_is_handled_so_one_it_sets_survives() {
        let mut shell = shell();
        shell.show(status("Exported"), local(0));

        assert!(shell.keypress(), "a showing message asks for a redraw");
        assert_eq!(shell.view_model().status, None);

        shell.show(status("Undo is not built yet"), local(0));
        assert_eq!(
            shell.view_model().status.as_deref(),
            Some("Undo is not built yet")
        );
        assert!(shell.keypress());
        assert!(!shell.keypress(), "nothing left to clear");
    }

    #[test]
    fn a_click_a_tick_or_a_dialog_button_leaves_the_status() {
        let mut shell = shell();
        let mut router = RecordingRouter::default();
        shell.show(status("Exported"), local(0));
        shell.show(toast(ToastKind::Info, "Synced"), local(0));
        shell.dialog(DialogInput::Opened);

        shell.handle(ShellInput::LegacyApply, &mut router);
        shell.handle(ShellInput::DayTick, &mut router);
        shell.tick_toasts(Instant::now());
        shell.toast(ToastInput::DismissVisible(0));
        shell.dialog(DialogInput::Closed);

        assert_eq!(shell.view_model().status.as_deref(), Some("Exported"));
    }

    #[test]
    fn a_toast_lives_out_its_lifetime_over_toast_ticks() {
        let mut shell = shell();
        let start = Instant::now();
        let tick = |shell: &mut Shell, seconds: u64| {
            shell.tick_toasts(start + Duration::from_secs(seconds));
        };
        shell.show(toast(ToastKind::Success, "Saved"), local(0));
        shell.show(toast(ToastKind::Error, "Save failed"), local(0));
        assert!(shell.toasts_ticking());

        tick(&mut shell, 10);
        tick(&mut shell, 13);
        assert_eq!(
            texts(&shell),
            ["Saved", "Save failed"],
            "the first tick only starts the count"
        );

        tick(&mut shell, 15);
        assert_eq!(texts(&shell), ["Save failed"], "an Error stays");
    }

    #[test]
    fn the_toasts_pause_while_hovered_or_while_a_modal_is_up() {
        let mut shell = shell();
        let start = Instant::now();
        shell.show(toast(ToastKind::Info, "Synced"), local(0));
        shell.tick_toasts(start);

        shell.toast(ToastInput::Hover(true));
        shell.tick_toasts(start + Duration::from_secs(10));
        assert_eq!(texts(&shell), ["Synced"], "hovered");

        shell.toast(ToastInput::Hover(false));
        shell.navigate(NavInput::EnterMode(InputMode::Command));
        shell.tick_toasts(start + Duration::from_secs(20));
        assert_eq!(texts(&shell), ["Synced"], "the palette is up");

        shell.navigate(NavInput::ExitMode);
        shell.tick_toasts(start + Duration::from_secs(30));
        assert!(texts(&shell).is_empty());
        assert!(!shell.toasts_ticking());
    }

    #[test]
    fn a_toast_tick_reports_only_a_change_to_the_drawn_stack() {
        let mut shell = shell();
        let start = Instant::now();
        assert!(!shell.tick_toasts(start), "nothing up");

        shell.show(toast(ToastKind::Error, "Save failed"), local(0));
        assert!(
            !shell.tick_toasts(start),
            "the first tick only starts the count"
        );
        assert!(
            !shell.tick_toasts(start + Duration::from_secs(60)),
            "a sticky Error never changes"
        );

        shell.show(toast(ToastKind::Success, "Saved"), local(0));
        assert!(!shell.tick_toasts(start + Duration::from_secs(61)));
        assert!(
            shell.tick_toasts(start + Duration::from_secs(70)),
            "the Success expiring changes the stack"
        );
        assert_eq!(texts(&shell), ["Save failed"]);
    }

    #[test]
    fn a_toast_raised_onto_an_empty_stack_starts_a_fresh_count() {
        let mut shell = shell();
        let start = Instant::now();
        shell.show(toast(ToastKind::Info, "one"), local(0));
        shell.tick_toasts(start);
        shell.toast(ToastInput::DismissAll);

        shell.show(toast(ToastKind::Info, "two"), local(0));
        shell.tick_toasts(start + Duration::from_secs(60));

        assert_eq!(texts(&shell), ["two"], "the idle minute isn't its age");
    }

    #[test]
    fn ctrl_l_dismisses_the_toasts_on_the_shell_alone() {
        let mut shell = shell();
        shell.show(toast(ToastKind::Error, "Save failed"), local(0));

        assert_eq!(press(&mut shell, &["ctrl+l"]), (ShellKey::Taken, vec![]));
        assert!(texts(&shell).is_empty());
    }

    #[test]
    fn tab_cycles_focus_and_a_mode_returns_focus_where_it_found_it() {
        let mut shell = shell();
        assert_eq!(shell.view_model().focus, FocusZone::View);

        shell.navigate(NavInput::FocusNext);
        assert_eq!(shell.view_model().focus, FocusZone::PrimaryRail);

        shell.navigate(NavInput::EnterMode(InputMode::Search));
        shell.navigate(NavInput::Focus(FocusZone::View));
        assert_eq!(shell.view_model().mode, InputMode::Search);

        shell.navigate(NavInput::ExitMode);
        assert_eq!(shell.view_model().mode, InputMode::Normal);
        assert_eq!(shell.view_model().focus, FocusZone::PrimaryRail);
    }

    #[test]
    fn a_dialog_is_dialog_mode_from_its_opening_to_its_closing() {
        let mut shell = shell();

        shell.dialog(DialogInput::Opened);
        assert_eq!(shell.view_model().mode, InputMode::Dialog);

        shell.dialog(DialogInput::Closed);
        assert_eq!(shell.view_model().mode, InputMode::Normal);

        shell.navigate(NavInput::EnterMode(InputMode::Insert));
        shell.dialog(DialogInput::Closed);
        assert_eq!(
            shell.view_model().mode,
            InputMode::Insert,
            "no Dialog was open to close"
        );
    }

    #[test]
    fn showing_a_noun_moves_focus_to_the_view_and_back_returns() {
        let mut shell = shell();
        shell.navigate(NavInput::FocusNext);

        shell.navigate(NavInput::Show(Noun::Bills));
        assert_eq!(shell.nav().noun(), Noun::Bills);
        assert_eq!(shell.view_model().focus, FocusZone::View);

        shell.navigate(NavInput::Back);
        assert_eq!(shell.nav().noun(), Noun::Dashboard);
        assert!(!shell.nav().can_go_back(), "nowhere further back");
    }

    fn hint_keys(shell: &Shell) -> Vec<(ShellCommand, String)> {
        shell
            .view_model()
            .hints
            .into_iter()
            .map(|(command, hint)| (command, hint.chord))
            .collect()
    }

    #[test]
    fn the_hints_carry_their_labels() {
        let actions: Vec<String> = shell()
            .view_model()
            .hints
            .into_iter()
            .map(|(_, hint)| hint.action)
            .collect();

        assert_eq!(
            actions,
            [
                msg::hint_command(),
                msg::hint_search(),
                msg::hint_help(),
                msg::hint_toggle_sidebar(),
            ]
        );
    }

    #[test]
    fn the_hints_name_each_key_as_bound_now() {
        let mut shell = shell();
        assert_eq!(
            hint_keys(&shell),
            [
                (ShellCommand::Command, ":".to_string()),
                (ShellCommand::Search, "/".to_string()),
                (ShellCommand::Help, "?".to_string()),
                (ShellCommand::ToggleRail, "b".to_string()),
            ]
        );

        let mut config = KeyBindingConfig::default();
        config
            .bindings
            .insert(TOGGLE_RAIL.to_string(), "ctrl+b".to_string());
        config.bindings.insert(SEARCH.to_string(), String::new());
        shell.set_keymap(&config);

        assert_eq!(
            hint_keys(&shell),
            [
                (ShellCommand::Command, ":".to_string()),
                (ShellCommand::Help, "?".to_string()),
                (ShellCommand::ToggleRail, "ctrl+b".to_string()),
            ],
            "a rebound key, and an unbound command drops"
        );
    }
}
