//! The command palette's Presenter (#1063): the `:` window listing every Command, held by the
//! Shell's Presenter. It holds the input, the selection, matching and ranking, the domain-headed
//! resting list, completion and the history with its walk. The history outlives each palette
//! session, so the Presenter lives as long as the Shell and only the session opens and closes.
//!
//! What each Command does is the Client's: it hands the Presenter one [`PaletteEntry`] per
//! Command, its effect `E` included, rebuilt when `[keybindings]` change, and runs the effect the
//! Presenter returns on select ([`PaletteRun`]). The registry names the Client's pages and
//! modules, which the Shell may not depend on.
//!
//! Its keys are the Dialog keymap's, though the palette is not a Dialog: select next and previous
//! move the selection, next field completes the selected name, select runs it and back closes.
//! [`HISTORY_BACK`] is the one key of its own. A key neither binds is typed, `backspace` included;
//! a list step is a letter, so it types too; a Dialog command the palette has no use for (submit,
//! previous field) does nothing.

use std::ops::Range;

use lib_config::KeyBindingConfig;
use lib_viper::keymap::{BACK, Binding, Keymap, NEXT_FIELD, SELECT, SELECT_NEXT, SELECT_PREVIOUS};

use crate::dialog::DialogCommand;

/// Walks back through the Commands run before, newest first.
pub const HISTORY_BACK: &str = "history_back";

/// A key the palette binds beside the Dialog keymap's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaletteCommand {
    HistoryBack,
}

/// The palette's own commands with their default keys.
pub const PALETTE_BINDINGS: &[Binding<PaletteCommand>] = &[Binding::hidden(
    HISTORY_BACK,
    "ctrl+r",
    PaletteCommand::HistoryBack,
)];

/// The palette's own keymap under `config`'s overrides.
pub fn palette_keymap(config: &KeyBindingConfig) -> Keymap<PaletteCommand> {
    Keymap::new(PALETTE_BINDINGS, config)
}

/// One Command the palette lists, as the Client hands it over.
#[derive(Debug, Clone)]
pub struct PaletteEntry<E> {
    /// The Command as typed: a stable English id, never translated.
    pub name: &'static str,
    /// The description Message, resolved in the Locale in effect when drawn.
    pub description: fn() -> String,
    /// The resting list's group the Command sits under, as its header reads. The Client hands the
    /// entries over grouped, in the order the resting list shows them.
    pub domain: String,
    /// The key it is bound to now, `None` while unbound.
    pub key: Option<String>,
    /// Whether text typed after the name is an argument (`accounts delete Home Loan`) rather than
    /// more of the query.
    pub takes_argument: bool,
    pub effect: E,
}

/// One row of the palette's list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaletteRow {
    /// A domain's header, at rest only.
    Header(String),
    Entry(PaletteEntryRow),
}

/// A Command's row as drawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteEntryRow {
    pub name: &'static str,
    /// The part of `name` the query matched, picked out as drawn; `None` at rest.
    pub matched: Option<Range<usize>>,
    /// The key column: the live key, or an em dash while unbound.
    pub key: String,
    pub description: String,
    pub selected: bool,
}

/// The footer's keys, each as bound now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteKeys {
    /// Select previous and next, which step the selection.
    pub step_previous: String,
    pub step_next: String,
    pub complete: String,
    pub run: String,
    pub history: String,
    pub close: String,
}

/// What the palette draws.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteViewModel {
    pub input: String,
    pub rows: Vec<PaletteRow>,
    /// The selected entry's index in `rows`, for the Client to keep it scrolled into view.
    pub selected_row: Option<usize>,
    /// How many Commands the input matches, of `total`.
    pub match_count: usize,
    pub total: usize,
    pub keys: PaletteKeys,
}

/// A Command chosen on select, for the Client to run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteRun<E> {
    pub name: &'static str,
    pub effect: E,
    /// The text typed after a Command that takes an argument, trimmed; empty for any other.
    pub argument: String,
}

/// What a key did to the palette.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaletteOutcome<E> {
    /// Nothing changed.
    Ignored,
    /// The input or the selection changed.
    Changed,
    /// The palette closed and left Command mode: back, or select with nothing matching.
    Closed,
    /// Select on a match: the palette closed, still in Command mode, for the Client to run the
    /// effect, which leaves the mode as it decides (the file explorer stays in it).
    Run(PaletteRun<E>),
}

/// One opening of the palette.
#[derive(Debug, Default)]
struct Session {
    input: String,
    /// Index into the matches, clamped there.
    selected: usize,
    /// How far the history walk has gone, `None` before its first step. Any edit resets it.
    history_cursor: Option<usize>,
}

/// The palette's Presenter.
#[derive(Debug)]
pub struct PalettePresenter<E> {
    entries: Vec<PaletteEntry<E>>,
    /// The Commands run, as typed, newest first, each once.
    history: Vec<String>,
    session: Option<Session>,
}

impl<E> Default for PalettePresenter<E> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            history: Vec::new(),
            session: None,
        }
    }
}

impl<E: Clone> PalettePresenter<E> {
    /// Lists `entries` from now on, as `[keybindings]` binds them.
    pub fn set_entries(&mut self, entries: Vec<PaletteEntry<E>>) {
        self.entries = entries;
        if let Some(session) = self.session.as_mut() {
            session.selected = 0;
        }
    }

    /// Opens a fresh session: an empty input, the first Command selected.
    pub fn open(&mut self) {
        self.session = Some(Session::default());
    }

    pub fn close(&mut self) {
        self.session = None;
    }

    pub fn is_open(&self) -> bool {
        self.session.is_some()
    }

    /// The Commands run, as typed, newest first.
    pub fn history(&self) -> &[String] {
        &self.history
    }

    /// The input typed so far, `None` while closed.
    pub fn input(&self) -> Option<&str> {
        self.session.as_ref().map(|session| session.input.as_str())
    }

    /// Resolves `chord` through the palette's keymap, then the Dialog keymap; a key neither binds
    /// is typed: `backspace` deletes, and `typed` is the character the key types, if any.
    pub fn key(
        &mut self,
        chord: &str,
        typed: Option<char>,
        keymap: &Keymap<PaletteCommand>,
        dialog_keymap: &Keymap<DialogCommand>,
    ) -> PaletteOutcome<E> {
        if self.session.is_none() {
            return PaletteOutcome::Ignored;
        }
        if keymap.event(chord) == Some(PaletteCommand::HistoryBack) {
            return self.history_back();
        }
        match dialog_keymap.event(chord) {
            Some(DialogCommand::Back) => {
                self.close();
                PaletteOutcome::Closed
            }
            Some(DialogCommand::Select) => self.select(),
            Some(DialogCommand::SelectNext) => self.move_selection(1),
            Some(DialogCommand::SelectPrevious) => self.move_selection(-1),
            Some(DialogCommand::NextField) => self.complete(),
            Some(command) if !command.is_list_step() => PaletteOutcome::Ignored,
            _ if chord == "backspace" => self.edit(|input| {
                input.pop();
            }),
            _ => match typed {
                Some(ch) => self.edit(|input| input.push(ch)),
                None => PaletteOutcome::Ignored,
            },
        }
    }

    /// What the palette draws, `None` while closed. `keymap` and `dialog_keymap` word the footer.
    pub fn view_model(
        &self,
        keymap: &Keymap<PaletteCommand>,
        dialog_keymap: &Keymap<DialogCommand>,
    ) -> Option<PaletteViewModel> {
        let session = self.session.as_ref()?;
        let matches = self.matches(&session.input);
        let selected = session.selected.min(matches.len().saturating_sub(1));
        let query = self.split(&session.input).0.to_lowercase();
        let at_rest = session.input.is_empty();

        let mut rows = Vec::with_capacity(matches.len() * 2);
        let mut selected_row = None;
        let mut last_domain: Option<&str> = None;
        for (index, entry) in matches.iter().enumerate() {
            if at_rest && last_domain != Some(entry.domain.as_str()) {
                rows.push(PaletteRow::Header(entry.domain.clone()));
                last_domain = Some(entry.domain.as_str());
            }
            if index == selected {
                selected_row = Some(rows.len());
            }
            rows.push(PaletteRow::Entry(PaletteEntryRow {
                name: entry.name,
                matched: matched(entry.name, &query),
                key: entry.key.clone().unwrap_or_else(|| "\u{2014}".to_string()),
                description: (entry.description)(),
                selected: index == selected,
            }));
        }

        let chord = |command| dialog_keymap.chord(command).unwrap_or_default().to_string();
        Some(PaletteViewModel {
            input: session.input.clone(),
            match_count: matches.len(),
            total: self.entries.len(),
            rows,
            selected_row,
            keys: PaletteKeys {
                step_previous: chord(SELECT_PREVIOUS),
                step_next: chord(SELECT_NEXT),
                complete: chord(NEXT_FIELD),
                run: chord(SELECT),
                history: keymap.chord(HISTORY_BACK).unwrap_or_default().to_string(),
                close: chord(BACK),
            },
        })
    }

    /// Splits `input` into the query and its argument: when it starts with the full name of a
    /// Command that takes an argument (case-insensitively, then a space or the end), that name is
    /// the query and the rest, trimmed, the argument. The longest such name wins; any other input
    /// is all query.
    fn split<'a>(&self, input: &'a str) -> (&'a str, &'a str) {
        let best = self
            .entries
            .iter()
            .filter(|entry| entry.takes_argument)
            .map(|entry| entry.name)
            .filter(|name| {
                input
                    .get(..name.len())
                    .is_some_and(|head| head.eq_ignore_ascii_case(name))
                    && input[name.len()..]
                        .chars()
                        .next()
                        .is_none_or(|next| next == ' ')
            })
            .max_by_key(|name| name.len());
        match best {
            Some(name) => (&input[..name.len()], input[name.len()..].trim()),
            None => (input, ""),
        }
    }

    /// Every entry matching `input`'s query case-insensitively, against its name, description or
    /// domain, ranked by [`rank`]; ties keep the entries' order.
    fn matches(&self, input: &str) -> Vec<&PaletteEntry<E>> {
        let needle = self.split(input).0.to_lowercase();
        let mut matches: Vec<_> = self
            .entries
            .iter()
            .filter(|entry| {
                needle.is_empty()
                    || entry.name.to_lowercase().contains(&needle)
                    || (entry.description)().to_lowercase().contains(&needle)
                    || entry.domain.to_lowercase().contains(&needle)
            })
            .collect();
        matches.sort_by_key(|entry| rank(entry, &needle));
        matches
    }

    /// The argument typed after the input's Command, trimmed; empty while closed or for none.
    fn argument(&self) -> String {
        self.input()
            .map(|input| self.split(input).1.to_string())
            .unwrap_or_default()
    }

    fn selected(&self) -> Option<&PaletteEntry<E>> {
        let session = self.session.as_ref()?;
        self.matches(&session.input).get(session.selected).copied()
    }

    /// Changes the input, putting the selection back on the first match and ending a history walk.
    fn edit(&mut self, change: impl FnOnce(&mut String)) -> PaletteOutcome<E> {
        let Some(session) = self.session.as_mut() else {
            return PaletteOutcome::Ignored;
        };
        change(&mut session.input);
        session.selected = 0;
        session.history_cursor = None;
        PaletteOutcome::Changed
    }

    /// Moves the selection `by` rows, clamped to the matches.
    fn move_selection(&mut self, by: isize) -> PaletteOutcome<E> {
        let count = self
            .session
            .as_ref()
            .map_or(0, |session| self.matches(&session.input).len());
        let Some(session) = self.session.as_mut() else {
            return PaletteOutcome::Ignored;
        };
        let last = count.saturating_sub(1);
        session.selected = session.selected.saturating_add_signed(by).min(last);
        PaletteOutcome::Changed
    }

    /// Fills the input with the selected Command's name, with a trailing space and any argument
    /// already typed for one taking an argument. Nothing with nothing selected.
    fn complete(&mut self) -> PaletteOutcome<E> {
        let Some((name, takes_argument)) = self
            .selected()
            .map(|entry| (entry.name, entry.takes_argument))
        else {
            return PaletteOutcome::Ignored;
        };
        let argument = self.argument();
        self.edit(|input| {
            *input = name.to_string();
            if takes_argument {
                input.push(' ');
                input.push_str(&argument);
            }
        })
    }

    /// Recalls the next older Command run into the input: the newest first, clamped at the oldest.
    fn history_back(&mut self) -> PaletteOutcome<E> {
        let Some(session) = self.session.as_mut() else {
            return PaletteOutcome::Ignored;
        };
        if self.history.is_empty() {
            return PaletteOutcome::Ignored;
        }
        let next = session
            .history_cursor
            .map_or(0, |index| (index + 1).min(self.history.len() - 1));
        session.history_cursor = Some(next);
        session.input = self.history[next].clone();
        session.selected = 0;
        PaletteOutcome::Changed
    }

    /// Closes the palette, returning the selected Command to run and recording it in the history
    /// as typed, argument and all. With nothing matching it just closes.
    fn select(&mut self) -> PaletteOutcome<E> {
        let chosen = self
            .selected()
            .map(|entry| (entry.name, entry.effect.clone()));
        let argument = self.argument();
        self.close();
        let Some((name, effect)) = chosen else {
            return PaletteOutcome::Closed;
        };
        let typed = if argument.is_empty() {
            name.to_string()
        } else {
            format!("{name} {argument}")
        };
        self.history.retain(|entry| *entry != typed);
        self.history.insert(0, typed);
        PaletteOutcome::Run(PaletteRun {
            name,
            effect,
            argument,
        })
    }
}

/// Ranks `entry` against a lowercased, possibly empty `needle`: `0` for a name starting with it
/// (every entry at rest, so the entries' order wins), `1` for the needle anywhere in the name or
/// the domain, `2` for a match in the description alone.
fn rank<E>(entry: &PaletteEntry<E>, needle: &str) -> u8 {
    let name = entry.name.to_lowercase();
    if needle.is_empty() || name.starts_with(needle) {
        0
    } else if name.contains(needle) || entry.domain.to_lowercase().contains(needle) {
        1
    } else {
        2
    }
}

/// The first case-insensitive occurrence of `needle` in `name`; `None` for an empty needle.
fn matched(name: &str, needle: &str) -> Option<Range<usize>> {
    if needle.is_empty() {
        return None;
    }
    let start = name.to_lowercase().find(needle)?;
    Some(start..start + needle.len())
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use lib_core::Clock;

    use super::*;
    use crate::nav::{InputMode, NavState};
    use crate::presenter::ShellPresenter;

    /// What a test Command does: its name.
    type Shell = ShellPresenter<(), &'static str>;

    fn entry(
        name: &'static str,
        domain: &str,
        description: fn() -> String,
        takes_argument: bool,
    ) -> PaletteEntry<&'static str> {
        PaletteEntry {
            name,
            description,
            domain: domain.to_string(),
            key: None,
            takes_argument,
            effect: name,
        }
    }

    fn plain() -> String {
        "goes somewhere".to_string()
    }

    fn preferences() -> String {
        "the ledger preferences".to_string()
    }

    /// A registry in the Client's shape: grouped by domain, a noun before its verbs.
    fn entries() -> Vec<PaletteEntry<&'static str>> {
        let mut dashboard = entry("dashboard", "Dashboard", plain, false);
        dashboard.key = Some("g d".to_string());
        vec![
            dashboard,
            entry("accounts", "Accounts", plain, false),
            entry("accounts new", "Accounts", plain, true),
            entry("accounts edit", "Accounts", plain, true),
            entry("accounts delete", "Accounts", plain, true),
            entry("budgets", "Budgets", plain, false),
            entry("budgets new", "Budgets", plain, false),
            entry("open", "Ledger", plain, false),
            entry("close", "Ledger", plain, false),
            entry("reports", "Reports", plain, false),
            entry("settings", "Settings", preferences, false),
            entry("transactions reports", "Transactions", plain, false),
        ]
    }

    fn shell() -> Shell {
        shell_under(&KeyBindingConfig::default())
    }

    fn shell_under(config: &KeyBindingConfig) -> Shell {
        let date = NaiveDate::from_ymd_opt(2026, 9, 19).expect("a real date");
        let mut shell = ShellPresenter::new(Clock::fixed(date), NavState::new());
        shell.set_keymap(config);
        shell.set_palette_entries(entries());
        shell
    }

    fn overriding(command: &str, chord: &str) -> KeyBindingConfig {
        let mut config = KeyBindingConfig::default();
        config
            .bindings
            .insert(command.to_string(), chord.to_string());
        config
    }

    /// Presses each space-separated chord, a single character typing itself.
    fn press(shell: &mut Shell, chords: &str) -> PaletteOutcome<&'static str> {
        let mut outcome = PaletteOutcome::Ignored;
        for chord in chords.split(' ') {
            let typed = match chord {
                "space" => Some(' '),
                _ if chord.chars().count() == 1 => chord.chars().next(),
                _ => None,
            };
            outcome = shell.palette_key(chord, typed);
        }
        outcome
    }

    /// Types `text` a character at a time.
    fn type_text(shell: &mut Shell, text: &str) {
        for ch in text.chars() {
            let chord = if ch == ' ' {
                "space".to_string()
            } else {
                ch.to_string()
            };
            shell.palette_key(&chord, Some(ch));
        }
    }

    fn opened() -> Shell {
        let mut shell = shell();
        shell.open_palette();
        shell
    }

    fn typed(text: &str) -> Shell {
        let mut shell = opened();
        type_text(&mut shell, text);
        shell
    }

    fn model(shell: &Shell) -> PaletteViewModel {
        shell.palette().expect("the palette is open")
    }

    fn input(shell: &Shell) -> String {
        model(shell).input
    }

    fn names(shell: &Shell) -> Vec<&'static str> {
        model(shell)
            .rows
            .into_iter()
            .filter_map(|row| match row {
                PaletteRow::Entry(entry) => Some(entry.name),
                PaletteRow::Header(_) => None,
            })
            .collect()
    }

    fn selected(shell: &Shell) -> Option<&'static str> {
        model(shell).rows.into_iter().find_map(|row| match row {
            PaletteRow::Entry(entry) if entry.selected => Some(entry.name),
            _ => None,
        })
    }

    fn headers(shell: &Shell) -> Vec<String> {
        model(shell)
            .rows
            .into_iter()
            .filter_map(|row| match row {
                PaletteRow::Header(domain) => Some(domain),
                PaletteRow::Entry(_) => None,
            })
            .collect()
    }

    /// Runs `text` from a freshly opened palette.
    fn run(shell: &mut Shell, text: &str) -> PaletteOutcome<&'static str> {
        shell.open_palette();
        type_text(shell, text);
        press(shell, "enter")
    }

    #[test]
    fn opening_enters_command_mode_on_an_empty_input_listing_every_command() {
        let shell = opened();

        assert_eq!(shell.nav().mode(), InputMode::Command);
        assert_eq!(input(&shell), "");
        assert_eq!(names(&shell).len(), entries().len());
        assert_eq!(model(&shell).match_count, entries().len());
        assert_eq!(model(&shell).total, entries().len());
        assert_eq!(selected(&shell), Some("dashboard"));
    }

    #[test]
    fn a_closed_palette_draws_nothing_and_takes_no_key() {
        let mut shell = shell();

        assert_eq!(shell.palette(), None);
        assert_eq!(press(&mut shell, "a"), PaletteOutcome::Ignored);
    }

    #[test]
    fn typing_appends_to_the_input_and_backspace_deletes() {
        let mut shell = typed("bud");
        assert_eq!(input(&shell), "bud");

        assert_eq!(press(&mut shell, "backspace"), PaletteOutcome::Changed);
        assert_eq!(input(&shell), "bu");

        press(&mut shell, "backspace backspace backspace");
        assert_eq!(input(&shell), "", "backspace on nothing does nothing");
    }

    #[test]
    fn a_list_step_letter_is_typed() {
        let shell = typed("jk");

        assert_eq!(input(&shell), "jk");
    }

    #[test]
    fn a_key_typing_nothing_and_a_dialog_command_the_palette_has_no_use_for_do_nothing() {
        let mut shell = typed("a");

        for chord in ["ctrl+x", "ctrl+enter", "shift+tab", "left"] {
            assert_eq!(press(&mut shell, chord), PaletteOutcome::Ignored, "{chord}");
        }
        assert_eq!(input(&shell), "a");
        assert!(shell.palette_open());
    }

    #[test]
    fn the_resting_list_heads_each_domain_once_and_a_filtered_list_has_no_headers() {
        let shell = opened();
        assert_eq!(
            headers(&shell),
            [
                "Dashboard",
                "Accounts",
                "Budgets",
                "Ledger",
                "Reports",
                "Settings",
                "Transactions"
            ]
        );

        assert!(headers(&typed("acc")).is_empty());
    }

    #[test]
    fn typing_filters_to_the_commands_matching_by_name_description_or_domain() {
        let shell = typed("budg");
        assert_eq!(names(&shell), ["budgets", "budgets new"]);
        assert_eq!(model(&shell).match_count, 2);

        let by_domain = typed("ledger");
        assert!(names(&by_domain).contains(&"open"), "the Ledger domain");
        assert!(
            names(&by_domain).contains(&"settings"),
            "its description says ledger"
        );

        assert!(names(&typed("zzzq")).is_empty());
        assert_eq!(selected(&typed("zzzq")), None);
    }

    #[test]
    fn a_name_prefix_outranks_a_name_or_domain_match_which_outranks_a_description_match() {
        assert_eq!(
            names(&typed("reports")),
            ["reports", "transactions reports"]
        );

        let ledger = names(&typed("ledger"));
        let open = ledger.iter().position(|name| *name == "open");
        let settings = ledger.iter().position(|name| *name == "settings");
        assert!(open < settings, "{ledger:?}");
    }

    #[test]
    fn the_matched_part_of_each_name_is_picked_out() {
        let shell = typed("ports");
        let PaletteRow::Entry(row) = &model(&shell).rows[0] else {
            panic!("a filtered list has no headers");
        };

        assert_eq!(row.name, "reports");
        assert_eq!(row.matched, Some(2..7));
    }

    #[test]
    fn each_row_shows_its_key_or_a_dash_while_unbound() {
        let shell = opened();
        let keys: Vec<_> = model(&shell)
            .rows
            .into_iter()
            .filter_map(|row| match row {
                PaletteRow::Entry(entry) => Some(entry.key),
                PaletteRow::Header(_) => None,
            })
            .take(2)
            .collect();

        assert_eq!(keys, ["g d", "\u{2014}"]);
    }

    #[test]
    fn select_next_and_previous_move_the_selection_clamped_at_the_ends() {
        let mut shell = typed("acc");
        assert_eq!(selected(&shell), Some("accounts"));

        press(&mut shell, "down");
        assert_eq!(selected(&shell), Some("accounts new"));
        press(&mut shell, "up up");
        assert_eq!(selected(&shell), Some("accounts"), "clamps at the first");

        press(&mut shell, "down down down down down");
        assert_eq!(
            selected(&shell),
            Some("accounts delete"),
            "clamps at the last"
        );
        press(&mut shell, "ctrl+p");
        assert_eq!(selected(&shell), Some("accounts edit"), "ctrl+p steps too");
    }

    #[test]
    fn the_selected_row_counts_the_headers_above_it() {
        let mut shell = opened();
        assert_eq!(
            model(&shell).selected_row,
            Some(1),
            "below the Dashboard header"
        );

        press(&mut shell, "down");
        assert_eq!(
            model(&shell).selected_row,
            Some(3),
            "below the Accounts header"
        );
    }

    #[test]
    fn typing_puts_the_selection_back_on_the_first_match() {
        let mut shell = typed("acc");
        press(&mut shell, "down down");

        type_text(&mut shell, "o");

        assert_eq!(selected(&shell), Some("accounts"));
    }

    #[test]
    fn next_field_completes_the_selected_name() {
        let mut shell = typed("bud");
        press(&mut shell, "down");

        assert_eq!(press(&mut shell, "tab"), PaletteOutcome::Changed);

        assert_eq!(input(&shell), "budgets new");
    }

    #[test]
    fn completing_a_command_taking_an_argument_leaves_room_for_it_and_keeps_one_typed() {
        let mut shell = typed("accounts del");
        press(&mut shell, "tab");
        assert_eq!(input(&shell), "accounts delete ");

        let mut shell = typed("accounts delete Home");
        press(&mut shell, "tab");
        assert_eq!(input(&shell), "accounts delete Home");
    }

    #[test]
    fn completing_with_nothing_matching_does_nothing() {
        let mut shell = typed("zzzq");

        assert_eq!(press(&mut shell, "tab"), PaletteOutcome::Ignored);
        assert_eq!(input(&shell), "zzzq");
    }

    #[test]
    fn a_typed_argument_keeps_its_command_selected_and_runs_with_it_trimmed() {
        let mut shell = typed("Accounts Delete  Home Loan ");
        assert_eq!(names(&shell), ["accounts delete"]);

        assert_eq!(
            press(&mut shell, "enter"),
            PaletteOutcome::Run(PaletteRun {
                name: "accounts delete",
                effect: "accounts delete",
                argument: "Home Loan".to_string(),
            })
        );
    }

    #[test]
    fn a_command_taking_no_argument_takes_what_follows_as_query() {
        assert_eq!(names(&typed("accounts")).len(), 4);
        assert!(names(&typed("open something")).is_empty());
    }

    #[test]
    fn select_closes_the_palette_and_returns_the_effect_still_in_command_mode() {
        let mut shell = typed("budg");

        assert_eq!(
            press(&mut shell, "enter"),
            PaletteOutcome::Run(PaletteRun {
                name: "budgets",
                effect: "budgets",
                argument: String::new(),
            })
        );
        assert!(!shell.palette_open());
        assert_eq!(
            shell.nav().mode(),
            InputMode::Command,
            "the Client leaves it"
        );
    }

    #[test]
    fn select_with_nothing_matching_closes_and_leaves_command_mode() {
        let mut shell = typed("zzzq");

        assert_eq!(press(&mut shell, "enter"), PaletteOutcome::Closed);

        assert!(!shell.palette_open());
        assert_eq!(shell.nav().mode(), InputMode::Normal);
        assert!(shell.palette_history().is_empty());
    }

    #[test]
    fn back_closes_and_leaves_command_mode() {
        let mut shell = typed("bud");

        assert_eq!(press(&mut shell, "esc"), PaletteOutcome::Closed);

        assert_eq!(shell.palette(), None);
        assert_eq!(shell.nav().mode(), InputMode::Normal);
    }

    #[test]
    fn the_history_keeps_each_command_once_as_typed_newest_first() {
        let mut shell = shell();
        run(&mut shell, "budgets");
        run(&mut shell, "accounts delete Home Loan");
        run(&mut shell, "budgets");

        assert_eq!(
            shell.palette_history(),
            ["budgets", "accounts delete Home Loan"]
        );
    }

    #[test]
    fn history_back_walks_older_and_clamps_at_the_oldest() {
        let mut shell = shell();
        run(&mut shell, "reports");
        run(&mut shell, "budgets");
        shell.open_palette();

        press(&mut shell, "ctrl+r");
        assert_eq!(input(&shell), "budgets");
        press(&mut shell, "ctrl+r");
        assert_eq!(input(&shell), "reports");
        press(&mut shell, "ctrl+r");
        assert_eq!(input(&shell), "reports", "clamps at the oldest");
    }

    #[test]
    fn history_back_with_no_history_does_nothing() {
        let mut shell = typed("x");

        assert_eq!(press(&mut shell, "ctrl+r"), PaletteOutcome::Ignored);
        assert_eq!(input(&shell), "x");
    }

    #[test]
    fn an_edit_after_a_recall_starts_the_walk_again() {
        let mut shell = shell();
        run(&mut shell, "reports");
        run(&mut shell, "budgets");
        shell.open_palette();
        press(&mut shell, "ctrl+r ctrl+r");

        press(&mut shell, "backspace ctrl+r");

        assert_eq!(input(&shell), "budgets");
    }

    #[test]
    fn the_history_outlives_each_session_and_a_new_one_opens_empty() {
        let mut shell = shell();
        run(&mut shell, "budgets");
        shell.navigate(crate::presenter::NavInput::ExitMode);

        shell.open_palette();

        assert_eq!(input(&shell), "");
        assert_eq!(shell.palette_history(), ["budgets"]);
    }

    #[test]
    fn the_palette_follows_the_dialog_keys_rebound_under_keybindings() {
        let mut config = overriding(BACK, "ctrl+h");
        for (command, chord) in [
            (SELECT, "ctrl+y"),
            (NEXT_FIELD, "ctrl+t"),
            (SELECT_NEXT, "ctrl+j"),
        ] {
            config
                .bindings
                .insert(command.to_string(), chord.to_string());
        }
        let mut shell = shell_under(&config);
        shell.open_palette();
        type_text(&mut shell, "acc");

        press(&mut shell, "ctrl+j");
        assert_eq!(selected(&shell), Some("accounts new"));
        press(&mut shell, "down");
        assert_eq!(
            selected(&shell),
            Some("accounts new"),
            "down no longer steps"
        );
        press(&mut shell, "ctrl+t");
        assert_eq!(input(&shell), "accounts new ");
        assert_eq!(press(&mut shell, "esc"), PaletteOutcome::Ignored);
        assert!(matches!(
            press(&mut shell, "ctrl+y"),
            PaletteOutcome::Run(_)
        ));

        shell.open_palette();
        assert_eq!(press(&mut shell, "ctrl+h"), PaletteOutcome::Closed);
    }

    #[test]
    fn history_back_follows_keybindings_and_frees_ctrl_r() {
        let mut shell = shell_under(&overriding(HISTORY_BACK, "ctrl+b"));
        run(&mut shell, "budgets");
        shell.open_palette();

        assert_eq!(press(&mut shell, "ctrl+r"), PaletteOutcome::Ignored);
        press(&mut shell, "ctrl+b");
        assert_eq!(input(&shell), "budgets");
    }

    #[test]
    fn the_footer_names_each_key_as_bound_now() {
        let defaults = model(&opened()).keys;
        assert_eq!(
            defaults,
            PaletteKeys {
                step_previous: "up".to_string(),
                step_next: "down".to_string(),
                complete: "tab".to_string(),
                run: "enter".to_string(),
                history: "ctrl+r".to_string(),
                close: "esc".to_string(),
            }
        );

        let mut shell = shell_under(&overriding(HISTORY_BACK, "ctrl+b"));
        shell.open_palette();
        assert_eq!(model(&shell).keys.history, "ctrl+b");
    }

    #[test]
    fn new_entries_relist_an_open_palette() {
        let mut shell = typed("dash");
        let mut rebound = entries();
        rebound[0].key = Some("g h".to_string());

        shell.set_palette_entries(rebound);

        let PaletteRow::Entry(row) = &model(&shell).rows[0] else {
            panic!("a filtered list has no headers");
        };
        assert_eq!(row.key, "g h");
    }
}
