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

use chrono::{DateTime, Local, NaiveDate};
use lib_config::KeyBindingConfig;
use lib_core::{Clock, Period};
use lib_list_cursor::{HALF_PAGE, Step};
use lib_locale::msg;
use lib_toast::{Display, Notice, Toast, ToastKind, Toasts};
use lib_viper::keymap::Keymap;
use lib_viper::view::Hint;

use std::path::{Path, PathBuf};

use crate::dialog::{DialogCommand, dialog_keymap};
use crate::explorer::{
    ExplorerFilter, ExplorerFilters, ExplorerMode, ExplorerOutcome, ExplorerPresenter,
    ExplorerViewModel, LedgerFolders,
};
use crate::help::HelpPresenter;
use crate::keymap::{HELP, OPEN_COMMAND_POPUP, SEARCH, ShellCommand, TOGGLE_RAIL, global_keymap};
use crate::nav::{ContextSelection, FocusZone, InputMode, NavState};
use crate::navigation::{Movement, Noun};
use crate::palette::{
    Destination, PaletteCommand, PaletteEffect, PaletteEntry, PaletteOutcome, PalettePresenter,
    PaletteRun, PaletteViewModel, palette_keymap,
};
use crate::protocols::{RailCounts, ShellDialog, ShellRailCounts, ShellRouter};
use crate::rail::{RailMode, RailOverride};
use crate::toast_history::ToastHistoryPresenter;
use crate::window_size::WindowSize;

/// What the Client delivers to the Shell, each through its one `send`. `F` is the Client's write
/// failure ([`ShellRouter::Failure`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellInput<F> {
    /// The Client's day poll fired: read the Clock for a new month.
    DayTick,
    /// A View not yet a module raised a Toast or set the status line. It reaches the Router, which
    /// queues it with every module's Notices for the Client to show ([`ShellPresenter::show`]).
    Notice(Notice),
    /// A committed write failed.
    WriteFailed(F),
    /// The Client's timer ran out on a prefix (the `g` of `g d`) with no second key: drop it.
    PrefixExpired,
}

/// What the Shell made of a key the Client offered it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellKey {
    /// The Shell took the key: it ran a command, or finished or dropped a prefix.
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
    /// Cycles focus forward; the context rail is a stop only while a Ledger is open.
    FocusNext {
        ledger_open: bool,
    },
    /// Cycles focus backward, with the same stop.
    FocusPrevious {
        ledger_open: bool,
    },
    Focus(FocusZone),
    EnterMode(InputMode),
    /// Enters a mode for something the active View opened itself, so the View owns its keys.
    EnterViewMode(InputMode),
    /// Leaves the mode, restoring the focus from before it.
    ExitMode,
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
    /// The rail's badge counts as of the last input, `None` with no Ledger open.
    pub rail_counts: Option<RailCounts>,
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

/// The Shell's Presenter. `P` is the Client's Settings page, which the Back history steps between,
/// and `E` what one of the Client's own palette Commands does, which its Router acts on.
pub struct ShellPresenter<P, E = ()> {
    clock: Clock,
    /// The calendar month the Shell last saw the Clock read.
    month: Period,
    dialog_open: bool,
    keymap: Keymap<ShellCommand>,
    /// The keys every module Dialog shares, under `[keybindings]`, which the Client's Dialogs read
    /// through [`ShellPresenter::dialog_keymap`].
    dialog_keymap: Keymap<DialogCommand>,
    /// The palette's own key beside the Dialog keymap's, under `[keybindings]`.
    palette_keymap: Keymap<PaletteCommand>,
    /// The command palette, whose history outlives each time it opens.
    palette: PalettePresenter<PaletteEffect<P, E>>,
    /// The Client's name for a Noun, a Message the Shell doesn't own, which the Help Dialog lists
    /// the jumps under.
    noun_label: fn(Noun) -> String,
    /// The file explorer behind `:open` and `:new`, whose filters and start outlive each opening.
    explorer: ExplorerPresenter,
    /// The rail `b` last set, with the Window Size it was set at (ADR-0042).
    rail_override: Option<RailOverride>,
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
    /// The rail's badge counts with the day they were read on; `None` with no Ledger open.
    rail_counts: Option<(NaiveDate, RailCounts)>,
}

impl<P: Copy + Eq + Default, E: Clone> ShellPresenter<P, E> {
    /// The Shell on the month `clock` reads now, which the Client generated the Schedule for as
    /// it started, and at `nav`, where the launch restored the person to.
    pub fn new(clock: Clock, nav: NavState<P>) -> Self {
        let month = Period::of(clock.today());
        Self {
            clock,
            month,
            dialog_open: false,
            keymap: global_keymap(&KeyBindingConfig::default()),
            dialog_keymap: dialog_keymap(&KeyBindingConfig::default()),
            palette_keymap: palette_keymap(&KeyBindingConfig::default()),
            palette: PalettePresenter::default(),
            noun_label: |noun| format!("{noun:?}"),
            explorer: ExplorerPresenter::default(),
            rail_override: None,
            prefix: None,
            nav,
            status: None,
            toasts: Toasts::default(),
            toasts_hovered: false,
            last_tick: None,
            rail_counts: None,
        }
    }

    /// The Clock the Shell reads the day from, which a Ledger opened later is built on too.
    pub fn clock(&self) -> &Clock {
        &self.clock
    }

    /// Puts the global keymap, the Dialog keymap and the palette's key under `config`'s overrides.
    /// The palette's entries carry their keys too, so the Client rebuilds them with it
    /// ([`Self::set_palette_entries`]).
    pub fn set_keymap(&mut self, config: &KeyBindingConfig) {
        self.keymap = global_keymap(config);
        self.dialog_keymap = dialog_keymap(config);
        self.palette_keymap = palette_keymap(config);
    }

    /// The Commands the palette lists, one per Command, under the `[keybindings]` in effect.
    pub fn set_palette_entries(&mut self, entries: Vec<PaletteEntry<PaletteEffect<P, E>>>) {
        self.palette.set_entries(entries);
    }

    /// The Client's name for each Noun, which the Help Dialog lists the jumps under.
    pub fn set_noun_label(&mut self, noun_label: fn(Noun) -> String) {
        self.noun_label = noun_label;
    }

    /// `:`: opens the palette in Command mode, on an empty input.
    pub fn open_palette(&mut self) {
        self.navigate(NavInput::EnterMode(InputMode::Command));
        self.palette.open();
    }

    /// Drops the palette without leaving Command mode, for the Client leaving it some other way.
    pub fn close_palette(&mut self) {
        self.palette.close();
    }

    pub fn palette_open(&self) -> bool {
        self.palette.is_open()
    }

    /// What the palette draws, `None` while it is closed.
    pub fn palette(&self) -> Option<PaletteViewModel> {
        self.palette
            .view_model(&self.palette_keymap, &self.dialog_keymap)
    }

    /// The Commands run from the palette, as typed, newest first.
    pub fn palette_history(&self) -> &[String] {
        self.palette.history()
    }

    /// A key while the palette is open, `chord` as `[keybindings]` writes it and `typed` the
    /// character it types, if any. Closing it leaves Command mode; a Command chosen is run
    /// ([`Self::run_palette`]). Whether it changed anything.
    pub fn palette_key<R: ShellRouter<Effect = E> + ?Sized>(
        &mut self,
        chord: &str,
        typed: Option<char>,
        router: &mut R,
    ) -> bool {
        match self
            .palette
            .key(chord, typed, &self.palette_keymap, &self.dialog_keymap)
        {
            PaletteOutcome::Ignored => false,
            PaletteOutcome::Changed => true,
            PaletteOutcome::Closed => {
                self.navigate(NavInput::ExitMode);
                true
            }
            PaletteOutcome::Run(run) => {
                self.run_palette(run, router);
                true
            }
        }
    }

    /// Runs a palette Command, chosen in the palette or clicked elsewhere (the no Ledger open
    /// content's `:open`), in Command mode. One needing a Ledger is refused with none open. The
    /// file explorer stays in Command mode while it is open; every other Command leaves it first,
    /// then runs: the Shell's own here, and a Client's through the Router once its destination
    /// shows, as its key would.
    pub fn run_palette<R: ShellRouter<Effect = E> + ?Sized>(
        &mut self,
        run: PaletteRun<PaletteEffect<P, E>>,
        router: &mut R,
    ) {
        if run.effect.needs_ledger() && !router.ledger_open() {
            router.refused_without_ledger();
            self.navigate(NavInput::ExitMode);
            return;
        }
        if let PaletteEffect::OpenExplorer(mode) = run.effect {
            self.explorer.open(mode, router.folders());
            return;
        }
        self.navigate(NavInput::ExitMode);
        match run.effect {
            PaletteEffect::OpenExplorer(_) => {}
            // A noun's Command resets the View's scroll only when it lands on another noun, as
            // the rail's `enter` does.
            PaletteEffect::Shell(ShellCommand::Jump(noun)) => self.jump(noun, false, router),
            PaletteEffect::Shell(command) => {
                self.command(command, router);
            }
            PaletteEffect::DismissNewestToast => {
                self.toasts.dismiss_newest();
            }
            PaletteEffect::CloseLedger => {
                router.close_ledger();
                self.show_noun(Noun::Dashboard, router);
                // Every place in the history was on the closed Ledger, so Back and Forward must
                // not reach one with none open.
                self.nav.forget_history();
            }
            PaletteEffect::Client {
                destination,
                effect,
                ..
            } => {
                match destination {
                    Some(Destination::Noun(noun)) => self.show_noun(noun, router),
                    Some(Destination::Settings(page)) => {
                        let before = self.nav.noun();
                        self.nav.record_settings_page(page);
                        self.nav.set_noun(Noun::Settings);
                        if before != Noun::Settings {
                            router.reset_view_scroll(Noun::Settings);
                        }
                    }
                    None => {}
                }
                router.run_effect(PaletteRun {
                    name: run.name,
                    effect,
                    argument: run.argument,
                });
            }
        }
    }

    /// Shows `noun`, its scroll reset when it wasn't already showing.
    fn show_noun<R: ShellRouter + ?Sized>(&mut self, noun: Noun, router: &mut R) {
        let before = self.nav.noun();
        self.nav.set_noun(noun);
        if self.nav.noun() != before {
            router.reset_view_scroll(noun);
        }
    }

    /// A `g` jump onto `noun`, refused with no Ledger open for a noun that needs one. A jump key
    /// always resets the View's scroll; the palette's only when it lands on another noun, or on
    /// Settings, which every way in lands on its index (its resting view) even from a page.
    fn jump<R: ShellRouter + ?Sized>(&mut self, noun: Noun, always_reset: bool, router: &mut R) {
        if noun.needs_ledger() && !router.ledger_open() {
            router.refused_without_ledger();
            return;
        }
        if always_reset || noun == Noun::Settings {
            self.nav.set_noun(noun);
            router.reset_view_scroll(noun);
        } else {
            self.show_noun(noun, router);
        }
    }

    /// `:open` or `:new`: opens the file explorer in `mode`, in Command mode, which it stays in
    /// while open, reading its start through `folders`.
    pub fn open_explorer(&mut self, mode: ExplorerMode, folders: &dyn LedgerFolders) {
        self.navigate(NavInput::EnterMode(InputMode::Command));
        self.explorer.open(mode, folders);
    }

    /// Drops the explorer without leaving Command mode, for the Client leaving it some other way.
    pub fn close_explorer(&mut self) {
        self.explorer.close();
    }

    pub fn explorer_open(&self) -> bool {
        self.explorer.is_open()
    }

    /// What the explorer draws, `None` while it is closed.
    pub fn explorer(&self) -> Option<ExplorerViewModel> {
        self.explorer.view_model(&self.dialog_keymap)
    }

    /// A key while the explorer is open, as [`Self::palette_key`] takes one. Closing it leaves
    /// Command mode; a file it chooses is opened or created through the Router
    /// ([`Self::explorer_chosen`]).
    pub fn explorer_key<R: ShellRouter + ?Sized>(
        &mut self,
        chord: &str,
        typed: Option<char>,
        router: &mut R,
    ) -> ExplorerOutcome {
        let outcome = self
            .explorer
            .key(chord, typed, &self.dialog_keymap, router.folders());
        self.explorer_outcome(outcome, router)
    }

    /// A click on one of the explorer's rows, `click_count` `2` for a double-click.
    pub fn explorer_click<R: ShellRouter + ?Sized>(
        &mut self,
        path: &Path,
        click_count: usize,
        router: &mut R,
    ) -> ExplorerOutcome {
        let outcome = self
            .explorer
            .click_entry(path, click_count, router.folders());
        self.explorer_outcome(outcome, router)
    }

    /// A click on one of the explorer's breadcrumbs, browsing `folder`.
    pub fn explorer_browse<R: ShellRouter + ?Sized>(
        &mut self,
        folder: PathBuf,
        router: &mut R,
    ) -> ExplorerOutcome {
        let outcome = self.explorer.navigate_to(folder, router.folders());
        self.explorer_outcome(outcome, router)
    }

    /// Follows what an input did to the explorer: closing it leaves Command mode, and a file it
    /// chose is opened or created through the Router.
    fn explorer_outcome<R: ShellRouter + ?Sized>(
        &mut self,
        outcome: ExplorerOutcome,
        router: &mut R,
    ) -> ExplorerOutcome {
        match &outcome {
            ExplorerOutcome::Closed => self.navigate(NavInput::ExitMode),
            ExplorerOutcome::Chosen(choice) => {
                let opened = match choice.mode {
                    ExplorerMode::Open => router.open_ledger(&choice.path),
                    ExplorerMode::New => router.create_ledger(&choice.path),
                    // A folder pick is never a file choice.
                    ExplorerMode::PickFolder => false,
                };
                self.explorer_chosen(opened, router);
            }
            // Picking closes the picker onto the View it opened over, as it was, whether the
            // write went or not: the Router says how it went.
            ExplorerOutcome::FolderPicked(folder) => {
                if router.set_data_folder(folder) {
                    self.explorer.set_start_folder(Some(folder.clone()));
                }
                self.explorer.close();
                self.navigate(NavInput::ExitMode);
            }
            ExplorerOutcome::Ignored | ExplorerOutcome::Changed => {}
        }
        outcome
    }

    /// A click on one of the explorer's footer checkboxes.
    pub fn explorer_toggle_filter(&mut self, filter: ExplorerFilter) -> ExplorerOutcome {
        self.explorer.toggle_filter(filter)
    }

    /// The explorer's Open or Create button.
    pub fn explorer_confirm<R: ShellRouter + ?Sized>(&mut self, router: &mut R) -> ExplorerOutcome {
        let outcome = self.explorer.confirm();
        self.explorer_outcome(outcome, router)
    }

    /// How the file the explorer chose went: one that opened closes it, leaves Command mode and
    /// lands on the new Ledger's Dashboard; one that failed leaves it open for another pick. A
    /// Client that finishes a `:new` after the explorer returned (the Desktop's Create ledger
    /// Dialog) reports it here.
    pub fn explorer_chosen<R: ShellRouter + ?Sized>(&mut self, opened: bool, router: &mut R) {
        self.explorer.chosen(opened);
        if opened {
            self.navigate(NavInput::ExitMode);
            self.show_noun(Noun::Dashboard, router);
        }
    }

    /// The explorer's footer checkboxes, which the Client keeps across a restart.
    pub fn explorer_filters(&self) -> ExplorerFilters {
        self.explorer.filters()
    }

    pub fn set_explorer_filters(&mut self, filters: ExplorerFilters) {
        self.explorer.set_filters(filters);
    }

    /// Where the explorer starts: `data_folder` while it can be read, else `fallback`.
    pub fn set_explorer_start(&mut self, data_folder: Option<PathBuf>, fallback: Option<PathBuf>) {
        self.explorer.set_start_folder(data_folder);
        self.explorer.set_fallback_folder(fallback);
    }

    /// The keys every module Dialog shares.
    pub fn dialog_keymap(&self) -> &Keymap<DialogCommand> {
        &self.dialog_keymap
    }

    /// The primary rail drawn at `window_size`: the `b` override while the window is still at the
    /// Window Size it was set at, otherwise that size's default. `start_hidden` is Settings ›
    /// Display's "start with the rail hidden".
    pub fn rail_mode(&self, window_size: WindowSize, start_hidden: bool) -> RailMode {
        crate::rail::rail_mode(window_size, start_hidden, self.rail_override)
    }

    /// `b`: flips the rail drawn at `window_size`, which holds until the window is drawn at another.
    pub fn toggle_rail(&mut self, window_size: WindowSize, start_hidden: bool) {
        self.rail_override = Some(crate::rail::toggled(
            window_size,
            start_hidden,
            self.rail_override,
        ));
    }

    /// The Client drew the window at `window_size`. An override set at another size is dropped for
    /// good, so coming back to that size draws its default (ADR-0042).
    pub fn window_drawn_at(&mut self, window_size: WindowSize) {
        if self
            .rail_override
            .is_some_and(|rail_override| rail_override.set_at != window_size)
        {
            self.rail_override = None;
        }
    }

    pub fn keymap(&self) -> &Keymap<ShellCommand> {
        &self.keymap
    }

    pub fn nav(&self) -> &NavState<P> {
        &self.nav
    }

    /// Drops the Back and Forward history (`NavState::forget_history`), as `:close` does.
    pub fn forget_history(&mut self) {
        self.nav.forget_history();
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
            rail_counts: self.rail_counts.map(|(_, counts)| counts),
        }
    }

    /// Reads the rail's badge counts afresh from `ledger` on the day the Clock reads, or drops
    /// them with no Ledger open. The Client calls it once each input's writes are committed, so
    /// a paid Bill, a changed Budget, a filed Document or a month turn shows at once, and as a
    /// Ledger opens or closes. The counts are read again only when the Client says the Ledger
    /// `changed`, the day has moved since they were read or none are held, so an input that wrote
    /// nothing, such as a cursor move, costs no read.
    pub fn refresh_rail_counts(&mut self, ledger: Option<&dyn ShellRailCounts>, changed: bool) {
        let Some(ledger) = ledger else {
            self.rail_counts = None;
            return;
        };
        let today = self.clock.today();
        let fresh = self.rail_counts.is_some_and(|(day, _)| day == today);
        if changed || !fresh {
            self.rail_counts = Some((today, ledger.rail_counts(today)));
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
                self.restart_idle_count();
                self.toasts.raise(kind, text, at);
            }
            Notice::Status(text) => self.status = Some(text),
        }
    }

    /// The Toasts test, from Settings › General or `:toasts test` (DESKTOP-184): one Toast of each
    /// Kind a second apart, `text` being the Client's test Message for each, stamped `at` for the
    /// history. The Toast timer raises all but the first.
    pub fn test_toasts(&mut self, text: impl Fn(ToastKind) -> String, at: DateTime<Local>) {
        self.restart_idle_count();
        self.toasts.raise_test(text, at);
    }

    /// Whether a Toast is up, echoed or waiting to be raised, for the Client's timer to tick.
    pub fn toasts_ticking(&self) -> bool {
        self.toasts.is_ticking()
    }

    /// Before a raise: with nothing ticking, the time since the last tick is idle, not a Toast's
    /// age, so the next tick only starts the count.
    fn restart_idle_count(&mut self) {
        if !self.toasts.is_ticking() {
            self.last_tick = None;
        }
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
            NavInput::FocusNext { ledger_open } => nav.cycle_focus_forward(ledger_open),
            NavInput::FocusPrevious { ledger_open } => nav.cycle_focus_backward(ledger_open),
            NavInput::Focus(zone) => nav.set_focus(zone),
            // The next key belongs to the mode, or to the one left, so a waiting prefix can't
            // finish on it; a Dialog opening drops it the same way.
            NavInput::EnterMode(mode) => {
                self.prefix = None;
                nav.enter_mode(mode);
            }
            NavInput::EnterViewMode(mode) => {
                self.prefix = None;
                nav.enter_view_mode(mode);
            }
            NavInput::ExitMode => {
                self.prefix = None;
                nav.exit_mode();
            }
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

    /// Runs a kept command, or passes the key on with the command the Shell falls back on.
    fn resolve<R: ShellRouter + ?Sized>(&mut self, chord: String, router: &mut R) -> ShellKey {
        match self.keymap.event(&chord) {
            Some(command) if !command.falls_back() => {
                self.command(command, router);
                ShellKey::Taken
            }
            fallback => ShellKey::Passed { chord, fallback },
        }
    }

    /// Runs a global command, from its key, the key route's fallback once the focused View has
    /// declined the key, or a click on its status-line hint. The commands that change only the
    /// Shell's own state run here; the rest reach the View or the frame through the Router's typed
    /// methods. Whether what the window draws may have changed.
    pub fn command<R: ShellRouter + ?Sized>(
        &mut self,
        command: ShellCommand,
        router: &mut R,
    ) -> bool {
        let noun = self.nav.noun();
        match command {
            ShellCommand::Help => {
                let group = router.help_group(noun);
                let help = HelpPresenter::open(&self.keymap, self.noun_label, group);
                self.host(ShellDialog::Help(help), router);
            }
            ShellCommand::Command => self.open_palette(),
            ShellCommand::Search => {
                if router.start_search(noun) {
                    self.navigate(NavInput::EnterViewMode(InputMode::Search));
                }
            }
            ShellCommand::ToggleRail => {
                let (window_size, start_hidden) = router.toggle_rail();
                self.toggle_rail(window_size, start_hidden);
            }
            ShellCommand::FocusPrevious => self.navigate(NavInput::FocusPrevious {
                ledger_open: router.ledger_open(),
            }),
            ShellCommand::FocusNext => self.navigate(NavInput::FocusNext {
                ledger_open: router.ledger_open(),
            }),
            // Only the View knows what of its own is open, and it holds `esc` only while it has
            // the keys or entered the mode itself.
            ShellCommand::Back => {
                let view_has_keys =
                    self.nav.focus() == FocusZone::View && self.nav.mode() == InputMode::Normal;
                return (self.nav.view_owns_mode() || view_has_keys) && router.view_back(noun);
            }
            ShellCommand::DismissToasts => self.toasts.dismiss_all(),
            ShellCommand::ToastHistory => {
                let history = ToastHistoryPresenter::open(self.toasts.history());
                self.host(ShellDialog::ToastHistory(history), router);
            }
            ShellCommand::Jump(noun) => self.jump(noun, true, router),
            ShellCommand::ChangeDataFolder => {
                self.palette.close();
                self.open_explorer(ExplorerMode::PickFolder, router.folders());
            }
            ShellCommand::Insert => self.navigate(NavInput::EnterMode(InputMode::Insert)),
            ShellCommand::Move(_) | ShellCommand::Select => {
                if let Some(movement) = Movement::of(command) {
                    self.movement(movement, router);
                }
            }
        }
        true
    }

    /// Hosts one of the Shell's own Dialogs, over a closed palette, in `Dialog` mode.
    fn host<R: ShellRouter + ?Sized>(&mut self, dialog: ShellDialog, router: &mut R) {
        self.palette.close();
        router.host_dialog(dialog);
        self.dialog(DialogInput::Opened);
    }

    /// A movement through the focused zone: the rails' rows are the Shell's, the View's are its
    /// own. Landing on another noun resets its scroll.
    fn movement<R: ShellRouter + ?Sized>(&mut self, movement: Movement, router: &mut R) {
        let before = self.nav.noun();
        match self.nav.focus() {
            FocusZone::PrimaryRail => {
                // Moving the highlight browses every row; committing it to a noun that needs a
                // Ledger is refused with none open.
                if movement == Movement::Enter
                    && self.nav.primary_highlight().needs_ledger()
                    && !router.ledger_open()
                {
                    router.refused_without_ledger();
                    return;
                }
                let input = match movement.step() {
                    Some(step) => NavInput::MoveHighlight(step),
                    None => NavInput::CommitHighlight,
                };
                self.navigate(input);
            }
            FocusZone::ContextRail => {
                let count = router.context_rail_count(before);
                // A noun with nothing in its context rail has nowhere to move.
                if count == 0 {
                    return;
                }
                let current = self.nav.context().unwrap_or(0);
                // `enter` selects the row and leaves focus where it is: the movement already set
                // it, and there is no per-row detail to open yet.
                let next = movement.step().map_or(current, |step| {
                    lib_list_cursor::step_by(current, count, step, (count / 2).max(1))
                });
                self.navigate(NavInput::Context(Some(next)));
            }
            FocusZone::View => router.move_selection(before, movement),
        }
        if self.nav.noun() != before {
            router.reset_view_scroll(self.nav.noun());
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
            DialogInput::Opened if !self.dialog_open => {
                self.dialog_open = true;
                self.nav.enter_mode(InputMode::Dialog);
            }
            DialogInput::Closed if self.dialog_open => {
                self.dialog_open = false;
                self.nav.exit_mode();
            }
            DialogInput::Opened | DialogInput::Closed => {}
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
        }
        if !self.toasts.is_ticking() {
            self.last_tick = None;
            return false;
        }
        // What the stack and the echo draw, less the timers: a raise that evicts keeps the count.
        let drawn = |toasts: &Toasts<DateTime<Local>>| {
            let shown = |toast: &Toast| (toast.kind(), toast.text().to_owned(), toast.count());
            (
                toasts.visible().iter().map(shown).collect::<Vec<_>>(),
                toasts.more_count(),
                toasts.echo().map(shown),
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
    use lib_viper::presenter::Presenter as _;

    use super::*;
    use crate::fake::{FakeRailCounts, RecordingRouter, Routed};
    use crate::keymap::GLOBAL_BINDINGS;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).expect("a real date")
    }

    /// A Client with no Settings pages to step between.
    type Shell = ShellPresenter<(), &'static str>;

    fn presenter(clock: Clock) -> Shell {
        ShellPresenter::new(clock, NavState::new())
    }

    #[test]
    fn the_rail_defaults_by_window_size_and_start_hidden() {
        let shell = presenter(Clock::fixed(date(2026, 9, 19)));

        assert_eq!(
            shell.rail_mode(WindowSize::Medium, false),
            RailMode::Collapsed
        );
        assert_eq!(
            shell.rail_mode(WindowSize::Large, false),
            RailMode::Expanded
        );
        assert_eq!(
            shell.rail_mode(WindowSize::XLarge, false),
            RailMode::Expanded
        );
        assert_eq!(
            shell.rail_mode(WindowSize::Medium, true),
            RailMode::Collapsed
        );
        assert_eq!(
            shell.rail_mode(WindowSize::Large, true),
            RailMode::Collapsed
        );
        assert_eq!(
            shell.rail_mode(WindowSize::XLarge, true),
            RailMode::Collapsed
        );
    }

    #[test]
    fn toggling_the_rail_flips_what_is_drawn_at_that_window_size() {
        let mut shell = presenter(Clock::fixed(date(2026, 9, 19)));

        shell.toggle_rail(WindowSize::Medium, false);

        assert_eq!(
            shell.rail_mode(WindowSize::Medium, false),
            RailMode::Expanded
        );
    }

    #[test]
    fn toggling_the_rail_twice_comes_back_to_the_default() {
        let mut shell = presenter(Clock::fixed(date(2026, 9, 19)));

        shell.toggle_rail(WindowSize::Large, false);
        shell.toggle_rail(WindowSize::Large, false);

        assert_eq!(
            shell.rail_mode(WindowSize::Large, false),
            RailMode::Expanded
        );
    }

    #[test]
    fn a_rail_override_does_not_apply_at_another_window_size() {
        let mut shell = presenter(Clock::fixed(date(2026, 9, 19)));
        shell.toggle_rail(WindowSize::Medium, false);

        assert_eq!(
            shell.rail_mode(WindowSize::Large, false),
            RailMode::Expanded
        );
    }

    #[test]
    fn drawing_the_window_at_another_size_drops_the_rail_override_for_good() {
        let mut shell = presenter(Clock::fixed(date(2026, 9, 19)));
        shell.toggle_rail(WindowSize::Medium, false);

        shell.window_drawn_at(WindowSize::Large);
        shell.window_drawn_at(WindowSize::Medium);

        assert_eq!(
            shell.rail_mode(WindowSize::Medium, false),
            RailMode::Collapsed
        );
    }

    #[test]
    fn drawing_the_window_at_the_same_size_keeps_the_rail_override() {
        let mut shell = presenter(Clock::fixed(date(2026, 9, 19)));
        shell.toggle_rail(WindowSize::Medium, false);

        shell.window_drawn_at(WindowSize::Medium);

        assert_eq!(
            shell.rail_mode(WindowSize::Medium, false),
            RailMode::Expanded
        );
    }

    /// The Shell with `:open` or `:new`'s explorer open on an empty `/ledgers`, and a Router
    /// reading those folders.
    fn exploring(mode: ExplorerMode) -> (Shell, RecordingRouter) {
        let router = RecordingRouter {
            folders: crate::fake::FakeLedgerFolders::default().folder(PathBuf::from("/ledgers")),
            ..RecordingRouter::default()
        };
        let mut shell = shell();
        shell.set_explorer_start(Some(PathBuf::from("/ledgers")), None);
        shell.open_explorer(mode, &router.folders);
        (shell, router)
    }

    #[test]
    fn the_explorer_stays_in_command_mode_while_open_and_back_leaves_it() {
        let (mut shell, mut router) = exploring(ExplorerMode::Open);
        assert_eq!(shell.nav().mode(), InputMode::Command);
        assert_eq!(
            shell.explorer().map(|explorer| explorer.mode),
            Some(ExplorerMode::Open)
        );

        assert_eq!(
            shell.explorer_key("esc", None, &mut router),
            ExplorerOutcome::Closed
        );

        assert!(!shell.explorer_open());
        assert_eq!(shell.nav().mode(), InputMode::Normal);
    }

    /// `:new`'s explorer with `teda` typed, chosen with `enter` on a Router whose files `open`.
    fn create_teda(opens: bool) -> (Shell, RecordingRouter) {
        let (mut shell, mut router) = exploring(ExplorerMode::New);
        shell.navigate(NavInput::Show(Noun::Settings));
        for ch in "teda".chars() {
            shell.explorer_key("", Some(ch), &mut router);
        }
        router.opens = opens;
        router.routed.clear();
        assert!(matches!(
            shell.explorer_key("enter", None, &mut router),
            ExplorerOutcome::Chosen(_)
        ));
        (shell, router)
    }

    #[test]
    fn a_created_file_that_opened_closes_the_explorer_on_the_dashboard() {
        let (shell, router) = create_teda(true);

        assert_eq!(
            router.routed,
            [
                Routed::CreatedLedger(PathBuf::from("/ledgers/teda.pldb")),
                Routed::ViewScrollReset(Noun::Dashboard),
            ]
        );
        assert!(!shell.explorer_open());
        assert_eq!(shell.nav().mode(), InputMode::Normal);
        assert_eq!(shell.nav().noun(), Noun::Dashboard);
    }

    #[test]
    fn a_file_that_failed_to_open_keeps_the_explorer_for_another_pick() {
        let (shell, router) = create_teda(false);

        assert_eq!(
            router.routed,
            [Routed::CreatedLedger(PathBuf::from("/ledgers/teda.pldb"))]
        );
        assert!(shell.explorer_open());
        assert_eq!(shell.nav().mode(), InputMode::Command);
        assert_eq!(shell.nav().noun(), Noun::Settings);
    }

    /// `:data` run from the palette on Bills, with or without a Ledger open, on a Router reading
    /// `/ledgers` and `/ledgers/finance`.
    fn picking_from_bills(ledger_open: bool) -> (Shell, RecordingRouter) {
        let mut shell = palette_on(PaletteEffect::Shell(ShellCommand::ChangeDataFolder));
        shell.navigate(NavInput::Show(Noun::Bills));
        shell.set_explorer_start(Some(PathBuf::from("/ledgers")), None);
        let mut router = RecordingRouter {
            ledger_open,
            folders: crate::fake::FakeLedgerFolders::default()
                .folder(PathBuf::from("/ledgers"))
                .folder(PathBuf::from("/ledgers/finance")),
            ..RecordingRouter::default()
        };
        run_on(&mut shell, &mut router);
        (shell, router)
    }

    #[test]
    fn data_opens_the_folder_picker_in_the_data_folder_with_or_without_a_ledger() {
        for ledger_open in [true, false] {
            let (shell, router) = picking_from_bills(ledger_open);

            let explorer = shell.explorer().expect("the picker is open");
            assert_eq!(explorer.mode, ExplorerMode::PickFolder);
            assert_eq!(explorer.folder, PathBuf::from("/ledgers"));
            assert_eq!(shell.nav().mode(), InputMode::Command);
            assert_eq!(shell.nav().noun(), Noun::Bills);
            assert!(router.routed.is_empty(), "{:?}", router.routed);
        }
    }

    #[test]
    fn use_sets_the_data_folder_and_returns_to_the_view_it_came_from() {
        let (mut shell, mut router) = picking_from_bills(true);
        shell.explorer_key("l", None, &mut router);

        assert_eq!(
            shell.explorer_key("ctrl+enter", None, &mut router),
            ExplorerOutcome::FolderPicked(PathBuf::from("/ledgers/finance"))
        );

        assert_eq!(
            router.routed,
            [Routed::SetDataFolder(PathBuf::from("/ledgers/finance"))]
        );
        assert!(!shell.explorer_open());
        assert_eq!(shell.nav().mode(), InputMode::Normal);
        assert_eq!(shell.nav().noun(), Noun::Bills);

        shell.open_explorer(ExplorerMode::Open, &router.folders);
        assert_eq!(
            shell.explorer().map(|explorer| explorer.folder),
            Some(PathBuf::from("/ledgers/finance")),
            ":open starts in the new Data folder"
        );
    }

    #[test]
    fn a_data_folder_not_in_effect_leaves_where_the_explorer_starts() {
        let (mut shell, mut router) = picking_from_bills(true);
        router.data_folder_applies = false;
        shell.explorer_key("l", None, &mut router);
        shell.explorer_key("ctrl+enter", None, &mut router);

        shell.open_explorer(ExplorerMode::Open, &router.folders);

        assert_eq!(
            shell.explorer().map(|explorer| explorer.folder),
            Some(PathBuf::from("/ledgers"))
        );
    }

    #[test]
    fn cancelling_the_folder_picker_changes_nothing_and_returns_to_the_view() {
        let (mut shell, mut router) = picking_from_bills(false);
        shell.explorer_key("l", None, &mut router);

        assert_eq!(
            shell.explorer_key("esc", None, &mut router),
            ExplorerOutcome::Closed
        );

        assert!(router.routed.is_empty());
        assert_eq!(shell.nav().mode(), InputMode::Normal);
        assert_eq!(shell.nav().noun(), Noun::Bills);
    }

    #[test]
    fn the_explorers_filters_outlive_each_opening() {
        let (mut shell, mut router) = exploring(ExplorerMode::Open);
        shell.explorer_toggle_filter(ExplorerFilter::PldbOnly);
        shell.explorer_key("esc", None, &mut router);

        shell.open_explorer(ExplorerMode::Open, &router.folders);

        assert!(!shell.explorer_filters().pldb_only);
    }

    #[test]
    fn the_dialog_keymap_follows_the_keybindings_the_shell_is_given() {
        let mut shell = presenter(Clock::fixed(date(2026, 9, 19)));
        assert_eq!(
            shell.dialog_keymap().event("enter"),
            Some(DialogCommand::Select)
        );

        let mut config = KeyBindingConfig::default();
        config
            .bindings
            .insert(lib_viper::keymap::SELECT.to_string(), "ctrl+y".to_string());
        shell.set_keymap(&config);

        assert_eq!(
            shell.dialog_keymap().event("ctrl+y"),
            Some(DialogCommand::Select)
        );
        assert_eq!(shell.dialog_keymap().event("enter"), None);
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
    fn a_notice_and_a_write_failure_go_to_the_router() {
        let mut shell = presenter(Clock::fixed(date(2026, 9, 19)));
        let mut router = RecordingRouter::default();
        let saved = Notice::Toast(ToastKind::Success, "Saved".to_string());

        shell.handle(ShellInput::Notice(saved.clone()), &mut router);
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
    fn each_kept_command_is_taken_on_its_default_chord() {
        for binding in GLOBAL_BINDINGS {
            if binding.event.falls_back() || binding.chord.is_empty() {
                continue;
            }
            let chords: Vec<&str> = binding.chord.split(' ').collect();
            let (answer, _) = press(&mut shell(), &chords);
            assert_eq!(answer, ShellKey::Taken, "{}", binding.command);
        }
    }

    #[test]
    fn help_hosts_the_help_dialog_with_the_views_keys_in_dialog_mode() {
        let mut shell = shell();
        shell.set_noun_label(|noun| format!("the {noun:?}"));
        let group = crate::help::HelpGroup {
            heading: "Bills".to_string(),
            keys: vec![("pay".to_string(), "p".to_string())],
        };
        let mut router = RecordingRouter {
            help_group: Some(group.clone()),
            ..RecordingRouter::default()
        };

        shell.key("?", &mut router);

        let [
            Routed::HelpGroup(Noun::Dashboard),
            Routed::Hosted(ShellDialog::Help(help)),
        ] = router.routed.as_slice()
        else {
            panic!("asked for the group, then hosted Help: {:?}", router.routed);
        };
        assert_eq!(help.view_model().group, Some(group));
        assert!(
            help.view_model()
                .jumps
                .iter()
                .any(|(action, _)| action == "the Dashboard")
        );
        assert!(shell.dialog_open());
        assert_eq!(shell.nav().mode(), InputMode::Dialog);
    }

    #[test]
    fn the_toast_history_hosts_the_toasts_raised_so_far() {
        let mut shell = shell();
        shell.show(toast(ToastKind::Info, "Synced"), local(0));
        let mut router = RecordingRouter::default();

        shell.command(ShellCommand::ToastHistory, &mut router);

        let [Routed::Hosted(ShellDialog::ToastHistory(history))] = router.routed.as_slice() else {
            panic!("hosted the Toast history: {:?}", router.routed);
        };
        assert_eq!(history.view_model().rows.len(), 1);
        assert_eq!(shell.nav().mode(), InputMode::Dialog);
    }

    #[test]
    fn the_palette_key_opens_the_palette_in_command_mode_on_the_shell_alone() {
        let mut shell = shell();

        assert_eq!(press(&mut shell, &[":"]), (ShellKey::Taken, vec![]));
        assert!(shell.palette_open());
        assert_eq!(shell.nav().mode(), InputMode::Command);
    }

    #[test]
    fn search_enters_search_mode_only_when_the_view_has_a_search() {
        let mut shell = shell();
        let mut router = RecordingRouter {
            has_search: false,
            ..RecordingRouter::default()
        };
        shell.key("/", &mut router);
        assert_eq!(router.routed, [Routed::StartedSearch(Noun::Dashboard)]);
        assert_eq!(shell.nav().mode(), InputMode::Normal, "`/` is inert");

        assert_eq!(
            press(&mut shell, &["/"]).1,
            [Routed::StartedSearch(Noun::Dashboard)]
        );
        assert_eq!(shell.nav().mode(), InputMode::Search);
        assert!(shell.nav().view_owns_mode(), "the View reads its keys");
    }

    #[test]
    fn the_rail_toggle_flips_the_rail_where_the_frame_draws_it() {
        let mut shell = shell();
        let mut router = RecordingRouter {
            drawn_at: (WindowSize::Medium, false),
            ..RecordingRouter::default()
        };

        shell.key("b", &mut router);

        assert_eq!(router.routed, [Routed::ToggledRail]);
        assert_eq!(
            shell.rail_mode(WindowSize::Medium, false),
            RailMode::Expanded
        );
    }

    #[test]
    fn shift_tab_cycles_focus_on_the_shell_alone_with_the_context_rail_only_with_a_ledger() {
        let mut shell = shell();
        assert_eq!(press(&mut shell, &["shift+tab"]), (ShellKey::Taken, vec![]));
        assert_eq!(shell.nav().focus(), FocusZone::ContextRail);

        let mut shell = self::shell();
        let mut router = RecordingRouter {
            ledger_open: false,
            ..RecordingRouter::default()
        };
        shell.key("shift+tab", &mut router);
        assert_eq!(shell.nav().focus(), FocusZone::PrimaryRail);
    }

    #[test]
    fn esc_asks_the_view_only_while_it_has_the_keys_or_owns_the_mode() {
        let mut shell = shell();
        assert_eq!(
            press(&mut shell, &["esc"]).1,
            [Routed::ViewBack(Noun::Dashboard)]
        );

        shell.navigate(NavInput::Focus(FocusZone::PrimaryRail));
        assert_eq!(press(&mut shell, &["esc"]).1, []);

        let mut router = RecordingRouter::default();
        shell.navigate(NavInput::EnterViewMode(InputMode::Filter));
        assert!(shell.command(ShellCommand::Back, &mut router));
        assert_eq!(router.routed, [Routed::ViewBack(Noun::Dashboard)]);

        router.takes_back = false;
        assert!(
            !shell.command(ShellCommand::Back, &mut router),
            "nothing to redraw"
        );
    }

    #[test]
    fn a_jump_shows_its_noun_and_resets_its_scroll() {
        let mut shell = shell();

        assert_eq!(
            press(&mut shell, &["g", "w"]),
            (ShellKey::Taken, vec![Routed::ViewScrollReset(Noun::Bills)])
        );
        assert_eq!(shell.nav().noun(), Noun::Bills);
        assert_eq!(
            press(&mut shell, &["g", "w"]).1,
            [Routed::ViewScrollReset(Noun::Bills)],
            "a jump key resets even onto the noun showing"
        );
    }

    #[test]
    fn a_jump_needing_a_ledger_is_refused_with_none_open() {
        let mut shell = shell();
        let mut router = RecordingRouter {
            ledger_open: false,
            ..RecordingRouter::default()
        };

        shell.key("g", &mut router);
        shell.key("w", &mut router);
        assert_eq!(router.routed, [Routed::RefusedWithoutLedger]);
        assert_eq!(shell.nav().noun(), Noun::Dashboard);

        router.routed.clear();
        shell.key("g", &mut router);
        shell.key("s", &mut router);
        assert_eq!(router.routed, [Routed::ViewScrollReset(Noun::Settings)]);
    }

    #[test]
    fn insert_and_focus_next_change_the_shells_own_state() {
        let mut shell = shell();
        let mut router = RecordingRouter::default();

        shell.command(ShellCommand::Insert, &mut router);
        assert_eq!(shell.nav().mode(), InputMode::Insert);
        shell.navigate(NavInput::ExitMode);
        shell.command(ShellCommand::FocusNext, &mut router);
        assert_eq!(shell.nav().focus(), FocusZone::PrimaryRail);

        assert!(router.routed.is_empty());
    }

    #[test]
    fn a_movement_in_the_view_is_the_views() {
        let mut shell = shell();
        let mut router = RecordingRouter::default();

        shell.command(ShellCommand::Move(Step::Next), &mut router);
        shell.command(ShellCommand::Select, &mut router);

        assert_eq!(
            router.routed,
            [
                Routed::Moved(Noun::Dashboard, Movement::Next),
                Routed::Moved(Noun::Dashboard, Movement::Enter),
            ]
        );
    }

    #[test]
    fn a_movement_on_the_primary_rail_browses_and_enter_shows_the_row() {
        let mut shell = shell();
        shell.navigate(NavInput::Focus(FocusZone::PrimaryRail));
        let mut router = RecordingRouter::default();

        shell.command(ShellCommand::Move(Step::Next), &mut router);
        assert_eq!(shell.nav().noun(), Noun::Dashboard, "browsing only");
        assert!(router.routed.is_empty());

        shell.command(ShellCommand::Select, &mut router);
        assert_eq!(shell.nav().noun(), Noun::Transactions);
        assert_eq!(router.routed, [Routed::ViewScrollReset(Noun::Transactions)]);
    }

    #[test]
    fn enter_on_a_rail_row_needing_a_ledger_is_refused_with_none_open() {
        let mut shell = shell();
        shell.navigate(NavInput::Focus(FocusZone::PrimaryRail));
        let mut router = RecordingRouter {
            ledger_open: false,
            ..RecordingRouter::default()
        };

        shell.command(ShellCommand::Move(Step::Next), &mut router);
        shell.command(ShellCommand::Select, &mut router);

        assert_eq!(router.routed, [Routed::RefusedWithoutLedger]);
        assert_eq!(shell.nav().noun(), Noun::Dashboard);
    }

    #[test]
    fn a_movement_on_the_context_rail_steps_through_its_rows() {
        let mut shell = shell();
        shell.navigate(NavInput::Focus(FocusZone::ContextRail));
        let mut router = RecordingRouter {
            context_rows: 3,
            ..RecordingRouter::default()
        };

        shell.command(ShellCommand::Move(Step::Last), &mut router);
        assert_eq!(shell.nav().context(), Some(2));
        shell.command(ShellCommand::Move(Step::Next), &mut router);
        assert_eq!(shell.nav().context(), Some(2), "clamped");

        router.context_rows = 0;
        shell.command(ShellCommand::Move(Step::First), &mut router);
        assert_eq!(shell.nav().context(), Some(2), "nothing to move through");
        assert!(router.routed.is_empty());
    }

    /// The Shell with the palette open on `effect`, a Command called `run`.
    fn palette_on(effect: PaletteEffect<(), &'static str>) -> Shell {
        let mut shell = shell();
        shell.set_palette_entries(vec![PaletteEntry {
            name: "run",
            description: String::new,
            domain: "Tests".to_string(),
            key: None,
            takes_argument: false,
            effect,
        }]);
        shell.command(ShellCommand::Command, &mut RecordingRouter::default());
        shell
    }

    /// Runs the palette's one Command with `enter`.
    fn run_on(shell: &mut Shell, router: &mut RecordingRouter) {
        assert!(shell.palette_key("enter", None, router));
        assert!(!shell.palette_open());
    }

    #[test]
    fn a_client_command_shows_its_settings_page_then_runs_through_the_router() {
        let mut shell = palette_on(PaletteEffect::Client {
            destination: Some(Destination::Settings(())),
            needs_ledger: true,
            effect: "units set-base",
        });
        let mut router = RecordingRouter::default();

        run_on(&mut shell, &mut router);

        assert_eq!(
            router.routed,
            [
                Routed::ViewScrollReset(Noun::Settings),
                Routed::RanEffect(PaletteRun {
                    name: "run",
                    effect: "units set-base",
                    argument: String::new(),
                }),
            ]
        );
        assert_eq!(shell.nav().noun(), Noun::Settings);
        assert_eq!(shell.nav().mode(), InputMode::Normal);
    }

    #[test]
    fn a_command_needing_a_ledger_is_refused_with_none_open_and_leaves_command_mode() {
        let mut shell = palette_on(PaletteEffect::CloseLedger);
        let mut router = RecordingRouter {
            ledger_open: false,
            ..RecordingRouter::default()
        };

        run_on(&mut shell, &mut router);

        assert_eq!(router.routed, [Routed::RefusedWithoutLedger]);
        assert_eq!(shell.nav().mode(), InputMode::Normal);
    }

    #[test]
    fn close_lands_on_the_dashboard_with_no_history() {
        let mut shell = palette_on(PaletteEffect::CloseLedger);
        shell.navigate(NavInput::Show(Noun::Bills));
        let mut router = RecordingRouter::default();

        run_on(&mut shell, &mut router);

        assert_eq!(
            router.routed,
            [
                Routed::ClosedLedger,
                Routed::ViewScrollReset(Noun::Dashboard)
            ]
        );
        assert_eq!(shell.nav().noun(), Noun::Dashboard);
        assert!(!shell.nav().can_go_back());
    }

    #[test]
    fn open_from_the_palette_opens_the_explorer_still_in_command_mode() {
        let mut shell = palette_on(PaletteEffect::OpenExplorer(ExplorerMode::Open));
        let mut router = RecordingRouter::default();

        run_on(&mut shell, &mut router);

        assert!(shell.explorer_open());
        assert_eq!(shell.nav().mode(), InputMode::Command);
        assert!(router.routed.is_empty());
    }

    #[test]
    fn a_palette_jump_resets_the_scroll_only_onto_another_noun() {
        let mut shell = palette_on(PaletteEffect::Shell(ShellCommand::Jump(Noun::Dashboard)));
        let mut router = RecordingRouter::default();

        run_on(&mut shell, &mut router);

        assert!(router.routed.is_empty());
        assert_eq!(shell.nav().mode(), InputMode::Normal);
    }

    #[test]
    fn a_palette_jump_onto_settings_always_enters_it_afresh() {
        let mut shell = palette_on(PaletteEffect::Shell(ShellCommand::Jump(Noun::Settings)));
        shell.navigate(NavInput::Show(Noun::Settings));
        let mut router = RecordingRouter::default();

        run_on(&mut shell, &mut router);

        assert_eq!(
            router.routed,
            [Routed::ViewScrollReset(Noun::Settings)],
            "`:settings` on a Settings page lands on the index"
        );
    }

    #[test]
    fn a_palette_shell_command_runs_as_its_key_does() {
        let mut shell = palette_on(PaletteEffect::Shell(ShellCommand::Search));
        let mut router = RecordingRouter::default();

        run_on(&mut shell, &mut router);

        assert_eq!(router.routed, [Routed::StartedSearch(Noun::Dashboard)]);
        assert_eq!(shell.nav().mode(), InputMode::Search);
    }

    #[test]
    fn dismiss_removes_the_newest_toast() {
        let mut shell = palette_on(PaletteEffect::DismissNewestToast);
        shell.show(toast(ToastKind::Error, "first"), local(0));
        shell.show(toast(ToastKind::Error, "second"), local(1));

        run_on(&mut shell, &mut RecordingRouter::default());

        assert_eq!(texts(&shell), ["first"]);
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

        assert_eq!(press(&mut shell, &["ctrl+b"]).1, vec![Routed::ToggledRail]);
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
                vec![Routed::ViewScrollReset(Noun::Settings)]
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
    fn a_tick_or_a_dialog_button_leaves_the_status() {
        let mut shell = shell();
        let mut router = RecordingRouter::default();
        shell.show(status("Exported"), local(0));
        shell.show(toast(ToastKind::Info, "Synced"), local(0));
        shell.dialog(DialogInput::Opened);

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
    fn the_toasts_test_raises_one_a_second_and_each_redraws() {
        let mut shell = shell();
        let start = Instant::now();
        shell.toast(ToastInput::Display(Display { toasts_on: false }));
        shell.test_toasts(|kind| format!("{kind:?}"), local(0));
        let echo = |shell: &Shell| shell.toasts().echo().map(|t| t.text().to_owned());
        assert_eq!(echo(&shell).as_deref(), Some("Success"));
        assert!(
            shell.toasts_ticking(),
            "the queue ticks with nothing on the stack"
        );

        assert!(
            !shell.tick_toasts(start),
            "the first tick only starts the count"
        );
        for (second, echoed) in [(1, "Info"), (2, "Warning")] {
            assert!(
                shell.tick_toasts(start + Duration::from_secs(second)),
                "the echo turning to {echoed} redraws"
            );
            assert_eq!(echo(&shell).as_deref(), Some(echoed));
        }
        assert!(shell.tick_toasts(start + Duration::from_secs(3)));
        assert_eq!(texts(&shell), ["Error"]);
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

        shell.navigate(NavInput::FocusNext { ledger_open: true });
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
        shell.navigate(NavInput::FocusNext { ledger_open: true });

        shell.navigate(NavInput::Show(Noun::Bills));
        assert_eq!(shell.nav().noun(), Noun::Bills);
        assert_eq!(shell.view_model().focus, FocusZone::View);

        shell.navigate(NavInput::Back);
        assert_eq!(shell.nav().noun(), Noun::Dashboard);
        assert!(!shell.nav().can_go_back(), "nowhere further back");
    }

    fn counts(bills: usize, budgets: usize, documents: usize) -> RailCounts {
        RailCounts {
            bills_attention: bills,
            budgets_over: budgets,
            documents_inbox: documents,
        }
    }

    #[test]
    fn with_no_ledger_open_the_shell_holds_no_rail_counts() {
        let mut shell = shell();
        assert_eq!(shell.view_model().rail_counts, None);

        shell.refresh_rail_counts(None, true);
        assert_eq!(shell.view_model().rail_counts, None);
    }

    #[test]
    fn the_rail_counts_are_held_from_a_refresh_until_the_next() {
        let mut shell = shell();
        let ledger = FakeRailCounts::default();
        ledger.counts.set(counts(2, 1, 3));

        shell.refresh_rail_counts(Some(&ledger), true);
        ledger.counts.set(counts(1, 1, 3));
        assert_eq!(
            shell.view_model().rail_counts,
            Some(counts(2, 1, 3)),
            "held until refreshed"
        );

        shell.refresh_rail_counts(Some(&ledger), true);
        assert_eq!(shell.view_model().rail_counts, Some(counts(1, 1, 3)));
    }

    #[test]
    fn an_input_that_changed_nothing_on_the_same_day_reads_no_counts() {
        let mut shell = shell();
        let ledger = FakeRailCounts::default();
        ledger.counts.set(counts(2, 1, 3));
        shell.refresh_rail_counts(Some(&ledger), false);
        assert_eq!(
            shell.view_model().rail_counts,
            Some(counts(2, 1, 3)),
            "none held yet, so the first refresh reads them"
        );

        ledger.counts.set(counts(1, 1, 3));
        shell.refresh_rail_counts(Some(&ledger), false);

        assert_eq!(shell.view_model().rail_counts, Some(counts(2, 1, 3)));
        assert_eq!(ledger.read_on.borrow().len(), 1, "read once");
    }

    #[test]
    fn closing_the_ledger_clears_the_rail_counts() {
        let mut shell = shell();
        let ledger = FakeRailCounts::default();
        ledger.counts.set(counts(2, 1, 3));
        shell.refresh_rail_counts(Some(&ledger), true);

        shell.refresh_rail_counts(None, true);

        assert_eq!(shell.view_model().rail_counts, None);
    }

    #[test]
    fn the_rail_counts_are_read_on_the_day_the_clock_reads_after_a_month_turn() {
        let (clock, day) = Clock::settable(date(2026, 9, 30));
        let mut shell = presenter(clock);
        let ledger = FakeRailCounts::default();
        shell.refresh_rail_counts(Some(&ledger), false);

        day.set(date(2026, 10, 1));
        shell.handle(ShellInput::DayTick, &mut RecordingRouter::default());
        // Nothing changed in the Ledger, but the day moved, so the counts are read again.
        shell.refresh_rail_counts(Some(&ledger), false);

        assert_eq!(
            *ledger.read_on.borrow(),
            [date(2026, 9, 30), date(2026, 10, 1)]
        );
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

        let config = KeyBindingConfig::overriding(TOGGLE_RAIL, "ctrl+b").and(SEARCH, "");
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
