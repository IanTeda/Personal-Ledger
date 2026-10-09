//! Command history and execution of `Shell`. An `impl Shell` block for the concern, not the
//! command registry itself (`navigation::command`).

use gpui::Context;

use super::{Shell, explorer::explorer_start_dir};
use crate::{
    chrome::palette::Palette,
    navigation::{
        command::{self, BudgetsVerb, Command, CommandEffect},
        explorer::FileExplorer,
        nav::{InputMode, Noun},
    },
    settings::SettingsSection,
    view::budgets::manage_dialog::ManageAction,
};

/// Records `name` as just-run in `history`, most-recent-first: drops any earlier occurrence
/// first so re-running a command moves it to the top rather than piling up a duplicate. Free
/// (rather than a `Palette` method) since `Shell::command_history` outlives any one `Palette`
/// instance -- see that field's own doc.
pub(super) fn record_history(history: &mut Vec<String>, name: &str) {
    history.retain(|entry| entry != name);
    history.insert(0, name.to_string());
}

impl Shell {
    /// Runs `command`'s effect -- one exhaustive match over [`CommandEffect`], the single
    /// source of truth for what a command does (issue #144's own architecture review, "deepen
    /// the command's interface": this replaced an `Option<fn(&mut NavState)>` handler plus a
    /// `Shell`-side `command.name == "open"` string match that couldn't express opening a
    /// `Shell`-owned dialog).
    ///
    /// [`CommandEffect::Navigate`] resets the view's scroll when it lands on a different noun,
    /// matching every other navigation entry point (`g`-jumps, rail `Enter`).
    /// [`CommandEffect::NotYetBuilt`] shows the same "not yet built" message
    /// `docs/ux/tui-mockups/navigation.md` describes for its own popup, reusing the status line's
    /// existing `status_message` slot (the "1d" spec's own COMMAND-mode status line has no
    /// message slot of its own, and the palette has already closed by the time this runs -- see
    /// the `enter` arm of [`Self::handle_palette_key`]).
    ///
    /// [`CommandEffect::OpenDialog`] (issues #165/#167) is the one variant that doesn't call
    /// `NavState::exit_mode` -- unlike every other effect, opening the dialog does *not* leave
    /// `InputMode::Command`: the "1e" file explorer's own status-line treatment (`Shell::render`'s
    /// `command_echo`) depends on staying there for as long as the dialog is on screen, exiting
    /// only when it closes (`Self::handle_explorer_cancel`/`Self::confirm_explorer_open`, or
    /// `Self::handle_key_down`'s `escape` arm).
    pub(super) fn run_command(
        &mut self,
        command: &'static Command,
        argument: &str,
        cx: &mut Context<'_, Self>,
    ) {
        // History keeps what was typed, argument and all, so `^r` recalls `accounts delete Home
        // Loan` rather than just the bare command.
        if argument.is_empty() {
            record_history(&mut self.command_history, command.name);
        } else {
            record_history(
                &mut self.command_history,
                &format!("{} {argument}", command.name),
            );
        }
        match command.effect {
            CommandEffect::OpenDialog(mode) => {
                self.file_explorer = Some(FileExplorer::open_at(
                    mode,
                    explorer_start_dir(),
                    self.explorer_filters,
                ));
            }
            CommandEffect::Navigate(noun) => {
                self.nav.exit_mode();
                let noun_before = self.nav.noun();
                self.nav.set_noun(noun);
                if self.nav.noun() != noun_before {
                    self.reset_view_scroll();
                }
            }
            CommandEffect::OpenSettingsPage(section) => {
                self.nav.exit_mode();
                self.open_settings_page(section);
            }
            CommandEffect::CloseLedger => {
                self.nav.exit_mode();
                self.nav.close_ledger();
            }
            CommandEffect::Accounts(verb) => {
                self.nav.exit_mode();
                self.run_accounts_command(command.name, verb, argument, cx);
            }
            CommandEffect::Colour(change) => {
                self.nav.exit_mode();
                self.pending_colour_change = Some(change);
            }
            CommandEffect::DismissToast => {
                self.nav.exit_mode();
                self.chrome.toasts.dismiss_newest();
            }
            CommandEffect::DismissAllToasts => {
                self.nav.exit_mode();
                self.chrome.toasts.dismiss_all();
            }
            CommandEffect::SetToasts(on) => {
                self.nav.exit_mode();
                self.set_toasts_on(on);
            }
            CommandEffect::OpenToastHistory => {
                self.nav.exit_mode();
                self.open_toast_history();
            }
            CommandEffect::MergeTags => {
                self.nav.exit_mode();
                self.open_settings_page(SettingsSection::Tags);
                self.open_merge_tags_dialog(None);
            }
            CommandEffect::Import => {
                self.nav.exit_mode();
                self.open_import();
            }
            CommandEffect::Documents(verb) => {
                self.nav.exit_mode();
                self.run_documents_command(verb);
            }
            CommandEffect::Budgets(verb) => {
                self.nav.exit_mode();
                let noun_before = self.nav.noun();
                self.nav.set_noun(Noun::Budgets);
                if noun_before != Noun::Budgets {
                    self.reset_view_scroll();
                }
                match verb {
                    BudgetsVerb::Switch => self.open_budgets_switcher(),
                    BudgetsVerb::New => self.open_budgets_new(cx),
                    BudgetsVerb::Edit => self.open_budgets_edit(self.budgets_state.current, cx),
                    BudgetsVerb::Manage => self.open_budgets_manage(),
                    BudgetsVerb::Duplicate => {
                        self.run_budgets_manage_action(
                            self.budgets_state.current,
                            ManageAction::Duplicate,
                            cx,
                        );
                    }
                    BudgetsVerb::SetDefault => {
                        self.run_budgets_manage_action(
                            self.budgets_state.current,
                            ManageAction::SetDefault,
                            cx,
                        );
                    }
                    BudgetsVerb::Archive => {
                        self.run_budgets_manage_action(
                            self.budgets_state.current,
                            ManageAction::Archive,
                            cx,
                        );
                    }
                    BudgetsVerb::Restore => {
                        self.run_budgets_manage_action(
                            self.budgets_state.current,
                            ManageAction::Restore,
                            cx,
                        );
                    }
                }
            }
            CommandEffect::NotYetBuilt => {
                self.nav.exit_mode();
                self.chrome.status_message = Some(
                    crate::msg::desktop_status_command_not_yet_built(command.name),
                );
            }
        }
    }

    /// A click on the empty state's own `:open`/`:new` text (`OnEmptyStateCommandClick`, issue
    /// #167): looks `command_name` up in the registry and runs it exactly as the palette's own
    /// `enter` key would -- entering `InputMode::Command` first, same as typing `:` would, so
    /// the status line's `COMMAND` badge and `Esc` (which only acts outside `InputMode::Normal`)
    /// both behave identically regardless of which entry point opened the dialog.
    pub(super) fn handle_empty_state_command_click(
        &mut self,
        command_name: &'static str,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(command) = command::all().find(|command| command.name == command_name) else {
            return;
        };
        self.nav.enter_mode(InputMode::Command);
        self.run_command(command, "", cx);
        if let Some(change) = self.pending_colour_change.take() {
            change.apply(cx);
        }
        cx.notify();
    }

    pub(super) fn open_palette(&mut self) {
        self.nav.enter_mode(InputMode::Command);
        self.chrome.palette = Some(Palette::with_history(self.command_history.clone()));
    }
}
