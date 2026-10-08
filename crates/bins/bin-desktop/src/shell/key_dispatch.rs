//! Keystroke dispatch of `Shell`. An `impl Shell` block for the concern, not the keystroke
//! routing decision itself (`navigation::key_router`).

use std::time::{Duration, Instant};

use gpui::{Context, KeyDownEvent, Keystroke};
use lib_toast::ToastKind;

use super::Shell;
use crate::{
    chrome::dialog_host::{self, Dialog, DialogKey, DialogOutcome},
    navigation::{
        active_view::ActiveView,
        explorer::ExplorerMode,
        key_router::{self, KeyOutcome, route_key},
        nav::{InputMode, Noun},
    },
    settings::SettingsSection,
};

/// The handoff's own "Jumps" timeout: a `g` with no completing chord within this window is
/// abandoned rather than left waiting indefinitely.
pub(super) const PENDING_G_TIMEOUT: Duration = Duration::from_millis(1000);

impl Shell {
    /// The status line's own COMMAND-mode echo (`crate::chrome::statusline::StatusLine`'s
    /// `command_echo`): the palette's live input while it's open, or -- once `:open` has been
    /// confirmed and the palette has closed in its favour -- the file explorer's frozen
    /// `"open"` echo, each paired with its own "esc closes ..." hint text.
    pub(super) fn command_echo(&self) -> Option<(String, String)> {
        if let Some(palette) = self.palette.as_ref() {
            Some((
                palette.input().to_string(),
                crate::msg::desktop_status_close_command_window("esc"),
            ))
        } else if let Some(explorer) = self.file_explorer.as_ref() {
            let name = match explorer.mode() {
                ExplorerMode::Open => "open",
                ExplorerMode::New => "new",
            };
            Some((
                name.to_string(),
                crate::msg::desktop_status_close_file_explorer("esc"),
            ))
        } else {
            None
        }
    }

    /// Routes a keystroke through the pure [`key_router::route_key`] decision function, then
    /// applies whatever [`KeyOutcome`] it returns. The routing logic itself -- `Esc`'s
    /// any-mode precedence, the `Command`-mode/other-non-`Normal` gates, a pending `g`'s
    /// completion/abort, the global mode-entry keys, `Tab` cycling, arming a fresh `g`, and
    /// [`Movement`] dispatch -- lives entirely in that `gpui`-free module now; this method is
    /// just the impure shell that owns `Shell`'s own state (`pending_g`, `status_message`,
    /// `palette`, `file_explorer`, `nav`) and applies the outcome to it. Returns `false` for a
    /// keystroke that changed nothing (nothing to redraw).
    pub(super) fn handle_key_down(&mut self, event: &KeyDownEvent) -> bool {
        let keystroke = &event.keystroke;
        let ctrl = keystroke.modifiers.control;
        let shift = keystroke.modifiers.shift;
        let key = keystroke.key.as_str();

        let pending_g_active = self
            .pending_g
            .take()
            .is_some_and(|since| since.elapsed() <= PENDING_G_TIMEOUT);

        let modifiers = key_router::Modifiers {
            ctrl,
            alt: keystroke.modifiers.alt,
            shift,
        };
        if key_router::dismisses_toasts(
            self.nav.mode(),
            pending_g_active,
            &self.dismiss_toasts_binding,
            key,
            modifiers,
        ) {
            self.toasts.dismiss_all();
            self.status_message = None;
            return true;
        }

        if key_router::opens_toast_history(
            self.nav.mode(),
            pending_g_active,
            self.toast_history_binding.as_deref(),
            key,
            modifiers,
        ) {
            self.open_toast_history();
            self.status_message = None;
            return true;
        }

        // Debug builds only: F9 raises each Kind in turn, to eyeball the layer before real
        // call sites exist (#346).
        #[cfg(debug_assertions)]
        if key == "f9" && self.nav.mode() == InputMode::Normal {
            let kind = ToastKind::ALL[self.debug_toast_kind % ToastKind::ALL.len()];
            self.debug_toast_kind += 1;
            self.raise_toast(kind, format!("{kind:?} Toast raised from F9"));
            return true;
        }

        if !pending_g_active && self.handle_budgets_plan_edit_key(keystroke) {
            self.status_message = None;
            return true;
        }

        if !pending_g_active && self.handle_import_key(keystroke) {
            self.status_message = None;
            return true;
        }

        if !pending_g_active && self.handle_documents_key(keystroke) {
            self.status_message = None;
            return true;
        }

        // `J`/`K` reorder the Documents page's types; the router would read them as `j`/`k`.
        if !pending_g_active && self.handle_settings_documents_reorder_key(keystroke) {
            self.status_message = None;
            return true;
        }

        // `J`/`K` reorder a Room within its Property; the router would read them as `j`/`k`.
        if !pending_g_active && self.handle_settings_inventory_reorder_key(keystroke) {
            self.status_message = None;
            return true;
        }

        let outcome = route_key(self.nav.mode(), pending_g_active, key, ctrl, shift);

        // `Esc`'s three shapes short-circuit before the hint-strip-clearing precedent below --
        // mirrors the original tier order exactly, including the quirk that a bare `Esc` with
        // nothing to do (`EscapeNoOp`) does *not* clear a stale status message.
        match outcome {
            KeyOutcome::ClearPendingG => return true,
            KeyOutcome::ClosePopupsAndExitMode => {
                if self.dialog.as_mut().is_some_and(Dialog::close_open_select) {
                    return true;
                }
                // The Schedule tab's filter selects: the first `Esc` closes an open list, the next
                // leaves the filter row.
                if let Some((_, state)) = self.bills_filter_focus.as_mut() {
                    if state.is_open() {
                        state.cancel();
                    } else {
                        self.bills_filter_focus = None;
                    }
                    return true;
                }
                self.palette = None;
                self.file_explorer = None;
                // The README's own Dialog lifecycle table: "`esc` closes any dialog without
                // saving" -- discards whatever was typed, same as Cancel.
                // The filter popover's dropdowns work the same way: the first `Esc` closes an open
                // list only, the next discards the draft and closes the popover.
                if let Some(form) = self.transactions_filter_form.as_mut()
                    && form.close_open_select()
                {
                    return true;
                }
                self.transactions_filter_form = None;
                self.close_dialog();
                self.budgets_plan_edit = None;
                // `Esc` while searching Transactions clears the search text as well as leaving the
                // mode (the map's decision: search is cleared by `Esc` or by emptying the box).
                if self.nav.mode() == InputMode::Search && self.nav.noun() == Noun::Transactions {
                    self.transactions_search.clear();
                    self.reset_transactions_selection();
                }
                if self.nav.mode() == InputMode::Search && self.nav.noun() == Noun::Documents {
                    self.documents_cancel_search();
                }
                self.nav.exit_mode();
                return true;
            }
            KeyOutcome::EscapeNoOp => return false,
            _ => {}
        }

        // The handoff's own precedent for hint-strip messages (`docs/ux/desktop/README.md`'s
        // "Loading and error states"): any keypress clears one, not just a timer.
        let had_status_message = self.status_message.take().is_some();

        match outcome {
            KeyOutcome::DelegateToPalette => {
                self.handle_palette_key(keystroke) || had_status_message
            }
            KeyOutcome::DelegateToSearch => self.handle_search_key(keystroke) || had_status_message,
            KeyOutcome::DelegateToDialog => self.handle_dialog_key(keystroke) || had_status_message,
            KeyOutcome::DelegateToFilter => self.handle_filter_key(keystroke) || had_status_message,
            KeyOutcome::Swallowed => had_status_message,
            KeyOutcome::JumpToNoun(noun) => {
                self.nav.set_noun(noun);
                self.reset_view_scroll();
                true
            }
            KeyOutcome::PendingGUnbound(message) => {
                self.status_message = Some(message);
                true
            }
            KeyOutcome::EnterCommand => {
                self.open_palette();
                true
            }
            KeyOutcome::EnterSearch => {
                // Settings has no Search: `/` is inert there rather than opening a mode with no box.
                if self.nav.noun() == Noun::Documents {
                    self.documents_start_search();
                } else if self.nav.noun() != Noun::Settings {
                    self.nav.enter_mode(InputMode::Search);
                }
                true
            }
            KeyOutcome::EnterHelp => {
                self.nav.enter_mode(InputMode::Help);
                true
            }
            KeyOutcome::CloseHelp => {
                self.nav.exit_mode();
                true
            }
            KeyOutcome::EnterInsert => {
                self.nav.enter_mode(InputMode::Insert);
                true
            }
            // Mirrors the TopBar's own rail-toggle button (`Shell::handle_toggle_rail`) --
            // same action, two entry points. Clears any settled collapsed-rail tooltip: it's
            // meaningless once the rail that anchors it changes shape.
            KeyOutcome::ToggleRail => {
                self.nav.toggle_primary_rail();
                self.collapsed_rail_tooltip = None;
                true
            }
            KeyOutcome::CycleFocusForward => {
                self.nav.cycle_focus_forward();
                true
            }
            KeyOutcome::CycleFocusBackward => {
                self.nav.cycle_focus_backward();
                true
            }
            KeyOutcome::ArmPendingG => {
                self.pending_g = Some(Instant::now());
                had_status_message
            }
            KeyOutcome::Movement(movement) => {
                self.apply_movement(movement);
                true
            }
            KeyOutcome::NoOp => {
                let handled = match self.active_view() {
                    ActiveView::Settings(SettingsSection::Accounts) => {
                        self.handle_accounts_key(keystroke)
                    }
                    ActiveView::Settings(SettingsSection::Categories) => {
                        self.handle_categories_key(keystroke)
                    }
                    ActiveView::Settings(SettingsSection::Payees) => {
                        self.handle_payees_key(keystroke)
                    }
                    ActiveView::Settings(SettingsSection::Documents) => {
                        self.handle_settings_documents_key(keystroke)
                    }
                    ActiveView::Settings(SettingsSection::Inventory) => {
                        self.handle_settings_inventory_key(keystroke)
                    }
                    ActiveView::Settings(SettingsSection::Tags) => self.handle_tags_key(keystroke),
                    ActiveView::Settings(
                        SettingsSection::General
                        | SettingsSection::Display
                        | SettingsSection::Units
                        | SettingsSection::Institutions
                        | SettingsSection::SyncServer
                        | SettingsSection::DataBackup
                        | SettingsSection::Tracing
                        | SettingsSection::About,
                    ) => false,
                    ActiveView::Bills => self.handle_bills_key(keystroke),
                    ActiveView::Budgets => self.handle_budgets_key(keystroke),
                    ActiveView::Transactions | ActiveView::Import => {
                        self.handle_transactions_key(keystroke)
                    }
                    ActiveView::Dashboard | ActiveView::Documents | ActiveView::Placeholder(_) => {
                        false
                    }
                };
                handled || had_status_message
            }
            KeyOutcome::ClearPendingG
            | KeyOutcome::ClosePopupsAndExitMode
            | KeyOutcome::EscapeNoOp => {
                unreachable!("handled above")
            }
        }
    }

    /// Routes a keystroke while the palette is open (tier 2, "popup-owned keys" -- mirroring
    /// `docs/ux/mockups/navigation.md`): `Backspace` mutates the input buffer, `Up`/`Down` move the
    /// selection, `Tab` completes to the selected result's full name, `Ctrl-r` cycles backward
    /// through previously run commands, `Enter` runs the selected command (see
    /// [`Self::run_command`]), and any other unmodified, printable key is typed into the query.
    /// Everything else is swallowed here rather than falling through to the zone/movement
    /// handling below -- keeping "the popup owns every keystroke" true even for a modified key
    /// (e.g. a bare `Ctrl`) this palette gives no meaning to.
    fn handle_palette_key(&mut self, keystroke: &Keystroke) -> bool {
        let Some(palette) = self.palette.as_mut() else {
            return false;
        };

        match keystroke.key.as_str() {
            "backspace" => {
                palette.backspace();
                true
            }
            "up" => {
                palette.move_up();
                true
            }
            "down" => {
                palette.move_down();
                true
            }
            "tab" => {
                palette.complete_selected();
                true
            }
            "r" if keystroke.modifiers.control => {
                palette.cycle_history_back();
                true
            }
            "enter" => {
                let command = palette.selected_command();
                let argument = palette.argument().to_string();
                self.palette = None;
                match command {
                    Some(command) => self.run_command(command, &argument),
                    // No result to run (an empty registry match) -- there's nothing left for
                    // `run_command` to do, so leave Command mode directly instead.
                    None => self.nav.exit_mode(),
                }
                true
            }
            _ => match typed_char(keystroke) {
                Some(ch) => {
                    palette.push_char(ch);
                    true
                }
                None => false,
            },
        }
    }

    /// Routes a keystroke while `InputMode::Search` is active (tier 2, mirroring
    /// [`Self::handle_palette_key`]'s shape): only Transactions has a Search box today. A no-op
    /// everywhere else.
    fn handle_search_key(&mut self, keystroke: &Keystroke) -> bool {
        if self.nav.noun() == Noun::Transactions {
            return self.handle_transactions_search_key(keystroke);
        }
        if self.nav.noun() == Noun::Documents {
            return self.handle_documents_search_key(keystroke);
        }
        false
    }

    /// Routes a keystroke while `InputMode::Dialog` is active (tier 2, mirroring
    /// [`Self::handle_search_key`]'s shape). A Dialog in the Dialog host goes through
    /// [`chrome::dialog_host::handle_key`]; the dialogs not yet moved there keep their own handlers.
    /// This tier returns before `route_key`'s `Tab` tier is ever checked, so the shell-wide zones
    /// stay untouched while a dialog is up.
    fn handle_dialog_key(&mut self, keystroke: &Keystroke) -> bool {
        if let Some(dialog) = self.dialog.as_mut() {
            return match dialog_host::handle_key(dialog, dialog_key(keystroke)) {
                DialogOutcome::Ignored => false,
                DialogOutcome::Handled => true,
                DialogOutcome::Confirm => {
                    self.confirm_open_dialog();
                    true
                }
            };
        }
        false
    }

    /// Runs what the key handlers left pending for after the `cx.notify()` -- effects that need
    /// the `gpui::App` rather than just `Shell`'s own state.
    pub(super) fn run_pending_effects(&mut self, cx: &mut Context<'_, Self>) {
        if std::mem::take(&mut self.pending_budgets_export) {
            self.export_budgets_history(cx);
        }
        if let Some(change) = self.pending_colour_change.take() {
            change.apply(cx);
        }
        self.run_pending_file_action(cx);
    }
}

/// The one character an unmodified keystroke types into a text field, or `None` for a modified
/// key (a chord this field gives no meaning to) or anything that isn't a single character.
/// `keystroke` as the Dialog host reads it.
fn dialog_key(keystroke: &Keystroke) -> DialogKey {
    match keystroke.key.as_str() {
        "backspace" => DialogKey::Backspace,
        "tab" if keystroke.modifiers.shift => DialogKey::BackTab,
        "tab" => DialogKey::Tab,
        "enter" if keystroke.modifiers.control => DialogKey::CtrlEnter,
        "enter" => DialogKey::Enter,
        "up" => DialogKey::Up,
        "down" => DialogKey::Down,
        "left" => DialogKey::Left,
        "right" => DialogKey::Right,
        key => {
            let modifiers = &keystroke.modifiers;
            let mut chars = key.chars();
            match (chars.next(), chars.next()) {
                (Some(ch), None)
                    if modifiers.control
                        && !modifiers.alt
                        && !modifiers.platform
                        && !modifiers.function =>
                {
                    DialogKey::Ctrl(ch)
                }
                _ => typed_char(keystroke).map_or(DialogKey::Other, DialogKey::Char),
            }
        }
    }
}

pub(super) fn typed_char(keystroke: &Keystroke) -> Option<char> {
    let modifiers = &keystroke.modifiers;
    if modifiers.control || modifiers.alt || modifiers.platform || modifiers.function {
        return None;
    }
    let mut chars = keystroke.key_char.as_deref()?.chars();
    match (chars.next(), chars.next()) {
        (Some(ch), None) => Some(ch),
        _ => None,
    }
}
