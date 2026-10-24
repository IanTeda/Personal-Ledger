//! Command history and execution of `Root`. An `impl Root` block for the concern, not the
//! command registry itself (`navigation::command`).

use gpui::{App, Context};
use lib_shell::{NavInput, ToastInput};

use super::{Root, explorer::explorer_start_dir};
use crate::{
    navigation::{
        command::{self, AccountsVerb, BudgetsVerb, Command, CommandEffect},
        explorer::FileExplorer,
        nav::{InputMode, Noun},
    },
    settings::SettingsPage,
    shell::view::palette::Palette,
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
    /// `InputMode::Command`: the "1e" file explorer's own status-line treatment (`Root::render`'s
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
                self.navigate(NavInput::ExitMode);
                let noun_before = self.nav().noun();
                self.navigate(NavInput::Show(noun));
                if self.nav().noun() != noun_before {
                    self.reset_view_scroll(cx);
                }
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
            CommandEffect::DismissAllToasts => {
                self.navigate(NavInput::ExitMode);
                self.presenter.toast(ToastInput::DismissAll);
            }
            CommandEffect::SetToasts(on) => {
                self.navigate(NavInput::ExitMode);
                self.set_toasts_on(on, cx);
            }
            CommandEffect::OpenToastHistory => {
                self.navigate(NavInput::ExitMode);
                self.open_toast_history();
            }
            CommandEffect::MergeTags => {
                self.navigate(NavInput::ExitMode);
                self.open_settings_page(SettingsPage::Tags, cx);
                let tags = self.views.settings.read(cx).tags().clone();
                tags.update(cx, |page, cx| page.open_merge_dialog(None, cx));
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
                let noun_before = self.nav().noun();
                self.navigate(NavInput::Show(Noun::Budgets));
                if noun_before != Noun::Budgets {
                    self.reset_view_scroll(cx);
                }
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
        let Some(command) = command::all().find(|command| command.name == command_name) else {
            return;
        };
        self.navigate(NavInput::EnterMode(InputMode::Command));
        self.run_command(command, "", cx);
        if let Some(change) = self.state.pending_colour_change.take() {
            change.apply(cx);
        }
        cx.notify();
    }

    pub(super) fn open_palette(&mut self) {
        self.navigate(NavInput::EnterMode(InputMode::Command));
        self.state.palette = Some(Palette::with_history(self.command_history.clone()));
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

    /// `accounts new|edit|delete [<account name>]`: jumps to the Accounts page, whose Entity then
    /// opens the same dialog its `n`/`e`/`d` and buttons do.
    pub(super) fn run_accounts_command(
        &mut self,
        command_name: &str,
        verb: AccountsVerb,
        argument: &str,
        cx: &mut Context<'_, Self>,
    ) {
        self.open_settings_page(SettingsPage::Accounts, cx);
        self.accounts_page(cx).update(cx, |page, cx| {
            page.run_command(command_name, verb, argument, cx);
        });
    }
}
