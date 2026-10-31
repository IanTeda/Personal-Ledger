//! Command history and execution of `Root`. An `impl Root` block for the concern, not the
//! command registry itself (`navigation::command`).

use gpui::{App, Context};
use lib_shell::{NavInput, ShellCommand, ToastInput};

use super::{Root, explorer::explorer_start_dir};
use crate::{
    accounts::AccountsPage,
    navigation::{
        command::{self, AccountsVerb, BudgetsVerb, CommandEffect, Entry, ModulePage},
        explorer::FileExplorer,
        nav::{InputMode, Noun},
    },
    settings::SettingsPage,
    shell::view::{palette::Palette, window_size::WindowSize},
    view::budgets::manage_dialog::ManageAction,
};

/// Records `name` as just-run in `history`, most-recent-first: drops any earlier occurrence
/// first so re-running a command moves it to the top rather than piling up a duplicate. Free
/// (rather than a `Palette` method) since `Root::command_history` outlives any one `Palette`
/// instance -- see that field's own doc.
pub(super) fn record_history(history: &mut Vec<String>, name: &str) {
    history.retain(|entry| entry != name);
    history.insert(0, name.to_string());
}

impl Root {
    /// Runs `command`'s effect -- one exhaustive match over [`CommandEffect`], the single
    /// source of truth for what a command does (issue #144's own architecture review, "deepen
    /// the command's interface": this replaced an `Option<fn(&mut NavState)>` handler plus a
    /// `Root`-side `command.name == "open"` string match that couldn't express opening a
    /// `Root`-owned dialog).
    ///
    /// A noun's Command (a `g` jump's) resets the view's scroll only when it lands on a different
    /// noun, as rail `Enter` does.
    /// [`CommandEffect::NotYetBuilt`] shows the same "not yet built" message
    /// `docs/ux/tui-mockups/navigation.md` describes for its own popup, reusing the status line's
    /// existing `status_message` slot (the "1d" spec's own COMMAND-mode status line has no
    /// message slot of its own, and the palette has already closed by the time this runs -- see
    /// the `enter` arm of [`Self::handle_palette_key`]).
    ///
    /// [`CommandEffect::OpenDialog`] (issues #165/#167) is the one variant that doesn't call
    /// `NavState::exit_mode` -- unlike every other effect, opening the dialog does *not* leave
    /// `InputMode::Command`: the "1e" file explorer's own status-line treatment (`Root::render`'s
    /// `command_echo`) depends on staying there for as long as the dialog is on screen, exiting
    /// only when it closes (`Self::handle_explorer_cancel`/`Self::confirm_explorer_open`, or
    /// `Self::handle_key_down`'s `escape` arm).
    ///
    /// [`CommandEffect::Shell`] runs as its global key does, at `window_size` (the rail toggle
    /// reads it); [`CommandEffect::Module`] opens its page and sends its event there.
    pub(super) fn run_command(
        &mut self,
        command: &Entry,
        argument: &str,
        window_size: WindowSize,
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
            CommandEffect::OpenSettingsPage(section) => {
                self.navigate(NavInput::ExitMode);
                self.open_settings_page(section, cx);
            }
            CommandEffect::CloseLedger => {
                self.navigate(NavInput::ExitMode);
                self.navigate(NavInput::CloseLedger);
            }
            CommandEffect::Accounts(verb) => {
                self.navigate(NavInput::ExitMode);
                self.run_accounts_command(command.name, verb, argument, cx);
            }
            CommandEffect::Colour(change) => {
                self.navigate(NavInput::ExitMode);
                self.state.pending_colour_change = Some(change);
            }
            CommandEffect::DismissToast => {
                self.navigate(NavInput::ExitMode);
                self.presenter.toast(ToastInput::DismissNewest);
            }
            CommandEffect::SetToasts(on) => {
                self.navigate(NavInput::ExitMode);
                self.set_toasts_on(on, cx);
            }
            CommandEffect::Shell(shell_command) => {
                self.navigate(NavInput::ExitMode);
                match shell_command {
                    // The Presenter takes this key for itself, so its command never reaches
                    // `run_global`.
                    ShellCommand::DismissToasts => self.presenter.toast(ToastInput::DismissAll),
                    ShellCommand::Jump(noun) => self.show_noun(noun, cx),
                    shell_command => {
                        self.run_global(shell_command, window_size, cx);
                    }
                }
            }
            CommandEffect::Module {
                page,
                command: name,
            } => {
                self.navigate(NavInput::ExitMode);
                if !self.run_module_command(page, name, cx) {
                    self.set_status(crate::msg::desktop_status_command_refused(command.name));
                }
            }
            CommandEffect::Import => {
                self.navigate(NavInput::ExitMode);
                self.open_import(cx);
            }
            CommandEffect::Documents(verb) => {
                self.navigate(NavInput::ExitMode);
                self.run_documents_command(verb, cx);
            }
            CommandEffect::Budgets(verb) => {
                self.navigate(NavInput::ExitMode);
                self.show_noun(Noun::Budgets, cx);
                match verb {
                    BudgetsVerb::Switch => self.open_budgets_switcher(cx),
                    BudgetsVerb::New => self.open_budgets_new(cx),
                    BudgetsVerb::Edit => self.open_budgets_edit(self.budgets_current(cx), cx),
                    BudgetsVerb::Manage => self.open_budgets_manage(cx),
                    BudgetsVerb::Duplicate => {
                        self.run_budgets_manage_action(
                            self.budgets_current(cx),
                            ManageAction::Duplicate,
                            cx,
                        );
                    }
                    BudgetsVerb::SetDefault => {
                        self.run_budgets_manage_action(
                            self.budgets_current(cx),
                            ManageAction::SetDefault,
                            cx,
                        );
                    }
                    BudgetsVerb::Archive => {
                        self.run_budgets_manage_action(
                            self.budgets_current(cx),
                            ManageAction::Archive,
                            cx,
                        );
                    }
                    BudgetsVerb::Restore => {
                        self.run_budgets_manage_action(
                            self.budgets_current(cx),
                            ManageAction::Restore,
                            cx,
                        );
                    }
                }
            }
            CommandEffect::NotYetBuilt => {
                self.navigate(NavInput::ExitMode);
                self.set_status(crate::msg::desktop_status_command_not_yet_built(
                    command.name,
                ));
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
        let Some(command) = command::find(command_name, &self.palette_entries) else {
            return;
        };
        self.navigate(NavInput::EnterMode(InputMode::Command));
        // `open` and `new` read no Window Size: only the rail toggle does.
        self.run_command(&command, "", WindowSize::Large, cx);
        if let Some(change) = self.state.pending_colour_change.take() {
            change.apply(cx);
        }
        cx.notify();
    }

    pub(super) fn open_palette(&mut self) {
        self.navigate(NavInput::EnterMode(InputMode::Command));
        self.state.palette = Some(Palette::new(
            self.palette_entries.clone(),
            self.command_history.clone(),
        ));
    }
}

impl Root {
    /// Opens a Settings page with focus in it -- what `:settings <page>`, the old `:accounts` /
    /// `:categories` / `:payees` / `:tags` aliases and every hand-off to a moved noun land on.
    /// Leaves the page's scroll alone when Settings is already showing.
    pub(super) fn open_settings_page(&mut self, section: SettingsPage, cx: &mut App) {
        let noun_before = self.nav().noun();
        self.navigate(NavInput::SettingsPage(section));
        self.navigate(NavInput::Show(Noun::Settings));
        if noun_before != Noun::Settings {
            self.reset_view_scroll(cx);
        }
        self.views
            .settings
            .update(cx, |view, cx| view.open_page(section, cx));
    }

    /// A module Command from the palette: opens `page` (Settings' page with focus in it, or the
    /// noun), whose module Router sends the event its keymap binds `command` to, inside one
    /// [`crate::input::send`] as the key's is. Whether the page took it.
    fn run_module_command(
        &mut self,
        page: ModulePage,
        command: &'static str,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        match page {
            ModulePage::Settings(section) => {
                self.open_settings_page(section, cx);
                self.views
                    .settings
                    .update(cx, |view, cx| view.module_command(section, command, cx))
            }
            ModulePage::Bills => {
                self.show_noun(Noun::Bills, cx);
                self.views
                    .bills
                    .update(cx, |page, cx| page.module_command(command, cx))
            }
            ModulePage::Transactions => {
                self.show_noun(Noun::Transactions, cx);
                self.views
                    .transactions
                    .update(cx, |page, cx| page.module_command(command, cx))
            }
        }
    }

    /// Shows `noun`, resetting the View's scroll when it wasn't already showing.
    fn show_noun(&mut self, noun: Noun, cx: &mut App) {
        let noun_before = self.nav().noun();
        self.navigate(NavInput::Show(noun));
        if noun_before != noun {
            self.reset_view_scroll(cx);
        }
    }

    /// `accounts new|edit|delete [<account name>]`: jumps to the Accounts page, then hands the
    /// command to the Accounts module's Router, which opens the same dialog the page's `n`/`e`/`d`
    /// and buttons do.
    pub(super) fn run_accounts_command(
        &mut self,
        command_name: &str,
        verb: AccountsVerb,
        argument: &str,
        cx: &mut Context<'_, Self>,
    ) {
        self.open_settings_page(SettingsPage::Accounts, cx);
        let page = self.views.settings.read(cx).page::<AccountsPage>();
        crate::accounts::router::run_command(&page, command_name, verb, argument, cx);
    }
}
