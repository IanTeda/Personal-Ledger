//! Keystroke dispatch of `Shell`. An `impl Shell` block for the concern, not the keystroke
//! routing decision itself (`navigation::key_router`).

use std::time::{Duration, Instant};

use gpui::{Context, KeyDownEvent, Keystroke};
use lib_toast::ToastKind;

use super::Shell;
use crate::{
    chrome::dialog_host::{self, Dialog, DialogKey, DialogOutcome},
    navigation::{
        explorer::ExplorerMode,
        key_router::{self, KeyOutcome, route_key},
        nav::{FocusZone, InputMode, Noun},
    },
    settings::SettingsSection,
    view::{
        active::{ActiveViewEntity, ShellView},
        event::ViewMode,
    },
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
        if let Some(palette) = self.chrome.palette.as_ref() {
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
    pub(super) fn handle_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<'_, Self>,
    ) -> bool {
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
            &self.chrome.dismiss_toasts_binding,
            key,
            modifiers,
        ) {
            self.chrome.toasts.dismiss_all();
            self.chrome.status_message = None;
            return true;
        }

        if key_router::opens_toast_history(
            self.nav.mode(),
            pending_g_active,
            self.chrome.toast_history_binding.as_deref(),
            key,
            modifiers,
        ) {
            self.open_toast_history();
            self.chrome.status_message = None;
            return true;
        }

        // Debug builds only: F9 raises each Kind in turn, to eyeball the layer before real
        // call sites exist (#346).
        #[cfg(debug_assertions)]
        if key == "f9" && self.nav.mode() == InputMode::Normal {
            let kind = ToastKind::ALL[self.chrome.debug_toast_kind % ToastKind::ALL.len()];
            self.chrome.debug_toast_kind += 1;
            self.raise_toast(kind, format!("{kind:?} Toast raised from F9"));
            return true;
        }

        // Keys a View claims ahead of the router (Bills' and Budgets' `tab`, Budgets' Plan cell
        // edit, the Transactions filter popover and Import step, the Documents keys, `J`/`K`
        // reorders); the router would otherwise read them as `tab`, `a` or `j`/`k`.
        if !pending_g_active && self.handle_view_key_ahead_of_router(keystroke, cx) {
            self.chrome.status_message = None;
            return true;
        }

        let outcome = route_key(self.nav.mode(), pending_g_active, key, ctrl, shift);

        // `Esc` asks the Dialog host's open select first, then the View, then falls back to the
        // close-and-exit-mode below (ADR-0037). The View is asked even when the router has nothing
        // for `Esc` to do, since only the View knows what of its own is open.
        if matches!(
            outcome,
            KeyOutcome::ClosePopupsAndExitMode | KeyOutcome::EscapeNoOp
        ) {
            if self
                .chrome
                .dialog
                .as_mut()
                .is_some_and(Dialog::close_open_select)
            {
                return true;
            }
            if self.chrome.dialog.is_none()
                && (self.view_owns_mode() || self.view_takes_key(keystroke))
                && self.active_view_entity(cx).escape(cx)
            {
                return true;
            }
        }

        // `Esc`'s three shapes short-circuit before the hint-strip-clearing precedent below --
        // mirrors the original tier order exactly, including the quirk that a bare `Esc` with
        // nothing to do (`EscapeNoOp`) does *not* clear a stale status message.
        match outcome {
            KeyOutcome::ClearPendingG => return true,
            KeyOutcome::ClosePopupsAndExitMode => {
                self.chrome.palette = None;
                self.file_explorer = None;
                // The README's own Dialog lifecycle table: "`esc` closes any dialog without
                // saving" -- discards whatever was typed, same as Cancel.
                self.close_dialog();
                self.nav.exit_mode();
                self.view_mode = None;
                return true;
            }
            KeyOutcome::EscapeNoOp => return false,
            _ => {}
        }

        // The handoff's own precedent for hint-strip messages (`docs/ux/desktop-mockups/README.md`'s
        // "Loading and error states"): any keypress clears one, not just a timer.
        let had_status_message = self.chrome.status_message.take().is_some();

        match outcome {
            KeyOutcome::DelegateToPalette => {
                self.handle_palette_key(keystroke, cx) || had_status_message
            }
            KeyOutcome::DelegateToSearch => {
                self.handle_search_key(keystroke, cx) || had_status_message
            }
            KeyOutcome::DelegateToDialog => {
                self.handle_dialog_key(keystroke, cx) || had_status_message
            }
            // Only a View opens a filter popover, and it reads the popover's keys ahead of the
            // router; what it leaves is swallowed.
            KeyOutcome::DelegateToFilter => had_status_message,
            KeyOutcome::Swallowed => had_status_message,
            KeyOutcome::JumpToNoun(noun) => {
                self.nav.set_noun(noun);
                self.reset_view_scroll(cx);
                true
            }
            KeyOutcome::PendingGUnbound(message) => {
                self.chrome.status_message = Some(message);
                true
            }
            KeyOutcome::EnterCommand => {
                self.open_palette();
                true
            }
            KeyOutcome::EnterSearch => {
                self.start_search(cx);
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
                self.view_mode = None;
                true
            }
            // Mirrors the TopBar's own rail-toggle button (`Shell::handle_toggle_rail`) --
            // same action, two entry points. Clears any settled collapsed-rail tooltip: it's
            // meaningless once the rail that anchors it changes shape.
            KeyOutcome::ToggleRail => {
                self.nav.toggle_primary_rail();
                self.chrome.collapsed_rail_tooltip = None;
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
                self.apply_movement(movement, cx);
                true
            }
            KeyOutcome::NoOp => {
                let view = match self.active_view_entity(cx) {
                    ActiveViewEntity::Shell(view) => view,
                    view => {
                        let handled =
                            self.view_takes_key(keystroke) && view.handle_key(keystroke, cx);
                        return handled || had_status_message;
                    }
                };
                let handled = match view {
                    ShellView::Settings(SettingsSection::Accounts) => {
                        self.handle_accounts_key(keystroke, cx)
                    }
                    ShellView::Settings(SettingsSection::Categories) => {
                        self.handle_categories_key(keystroke, cx)
                    }
                    ShellView::Settings(SettingsSection::Payees) => {
                        self.handle_payees_key(keystroke, cx)
                    }
                    ShellView::Settings(SettingsSection::Documents) => {
                        self.handle_settings_documents_key(keystroke, cx)
                    }
                    ShellView::Settings(SettingsSection::Inventory) => {
                        self.handle_settings_inventory_key(keystroke, cx)
                    }
                    ShellView::Settings(SettingsSection::Tags) => {
                        self.handle_tags_key(keystroke, cx)
                    }
                    ShellView::Settings(
                        SettingsSection::General
                        | SettingsSection::Display
                        | SettingsSection::Units
                        | SettingsSection::Institutions
                        | SettingsSection::SyncServer
                        | SettingsSection::DataBackup
                        | SettingsSection::Tracing
                        | SettingsSection::About,
                    ) => false,
                    ShellView::Dashboard | ShellView::Placeholder(_) => false,
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

    /// Whether a key reaches the active View: the View zone has focus, in `Normal` mode, with no
    /// `Ctrl`, `Alt` or `Cmd` held (ADR-0037). A pending `g` is ruled out before this is asked.
    /// `Shift` is the View's to read.
    fn view_takes_key(&self, keystroke: &Keystroke) -> bool {
        let modifiers = &keystroke.modifiers;
        self.nav.focus() == FocusZone::View
            && self.nav.mode() == InputMode::Normal
            && !(modifiers.control || modifiers.alt || modifiers.platform)
    }

    /// Whether a key reaches the active View ahead of the router: as [`Self::view_takes_key`], and
    /// every key in an `Insert` or `Filter` mode the View entered for something it opened itself
    /// (`ViewEvent::EnterMode`). The router swallows every key in those modes, so only the View can
    /// use one; `Esc` comes back to the View as `escape`. A Search the View entered reads its keys
    /// through `search_key` instead.
    fn view_takes_key_ahead(&self, keystroke: &Keystroke) -> bool {
        (self.view_owns_mode() && self.nav.mode() != InputMode::Search)
            || self.view_takes_key(keystroke)
    }

    /// `/` (or the hint strip's `/ search`): the active View's Search, when it has one. Settings
    /// has no Search: `/` is inert there rather than opening a mode with no box.
    pub(super) fn start_search(&mut self, cx: &mut Context<'_, Self>) {
        let view = self.active_view_entity(cx);
        if !matches!(view, ActiveViewEntity::Shell(_)) {
            if view.start_search(cx) {
                self.nav.enter_mode(InputMode::Search);
                self.view_mode = Some(ViewMode::Search);
            }
            return;
        }
        if self.nav.noun() != Noun::Settings {
            self.nav.enter_mode(InputMode::Search);
        }
    }

    /// The active View's keys that sit ahead of the router. Each arm is exclusive with the others
    /// because `ShellView` is: the handler behind an arm only fires for the View it names, so
    /// matching on the View first keeps the same keys owned by the same View.
    fn handle_view_key_ahead_of_router(
        &mut self,
        keystroke: &Keystroke,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        let view = match self.active_view_entity(cx) {
            ActiveViewEntity::Shell(view) => view,
            view => return self.view_takes_key_ahead(keystroke) && view.key_ahead(keystroke, cx),
        };
        match view {
            ShellView::Settings(SettingsSection::Documents) => {
                self.handle_settings_documents_reorder_key(keystroke, cx)
            }
            ShellView::Settings(SettingsSection::Inventory) => {
                self.handle_settings_inventory_reorder_key(keystroke, cx)
            }
            ShellView::Dashboard
            | ShellView::Placeholder(_)
            | ShellView::Settings(
                SettingsSection::General
                | SettingsSection::Display
                | SettingsSection::Units
                | SettingsSection::Institutions
                | SettingsSection::SyncServer
                | SettingsSection::DataBackup
                | SettingsSection::Tracing
                | SettingsSection::About
                | SettingsSection::Accounts
                | SettingsSection::Categories
                | SettingsSection::Payees
                | SettingsSection::Tags,
            ) => false,
        }
    }

    /// Routes a keystroke while the palette is open (tier 2, "popup-owned keys" -- mirroring
    /// `docs/ux/tui-mockups/navigation.md`): `Backspace` mutates the input buffer, `Up`/`Down` move the
    /// selection, `Tab` completes to the selected result's full name, `Ctrl-r` cycles backward
    /// through previously run commands, `Enter` runs the selected command (see
    /// [`Self::run_command`]), and any other unmodified, printable key is typed into the query.
    /// Everything else is swallowed here rather than falling through to the zone/movement
    /// handling below -- keeping "the popup owns every keystroke" true even for a modified key
    /// (e.g. a bare `Ctrl`) this palette gives no meaning to.
    fn handle_palette_key(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        let Some(palette) = self.chrome.palette.as_mut() else {
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
                self.chrome.palette = None;
                match command {
                    Some(command) => self.run_command(command, &argument, cx),
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
    /// [`Self::handle_palette_key`]'s shape): to the active View's Search. A no-op everywhere
    /// else.
    fn handle_search_key(&mut self, keystroke: &Keystroke, cx: &mut gpui::App) -> bool {
        let view = self.active_view_entity(cx);
        if !matches!(view, ActiveViewEntity::Shell(_)) {
            return view.search_key(keystroke, cx);
        }
        false
    }

    /// Routes a keystroke while `InputMode::Dialog` is active (tier 2, mirroring
    /// [`Self::handle_search_key`]'s shape). A Dialog in the Dialog host goes through
    /// [`chrome::dialog_host::handle_key`]; the dialogs not yet moved there keep their own handlers.
    /// This tier returns before `route_key`'s `Tab` tier is ever checked, so the shell-wide zones
    /// stay untouched while a dialog is up.
    fn handle_dialog_key(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        if let Some(dialog) = self.chrome.dialog.as_mut() {
            return match dialog_host::handle_key(dialog, dialog_key(keystroke)) {
                DialogOutcome::Ignored => false,
                DialogOutcome::Handled => true,
                DialogOutcome::Confirm => {
                    self.confirm_open_dialog(cx);
                    true
                }
            };
        }
        false
    }

    /// Runs what the key handlers left pending for after the `cx.notify()` -- effects that need
    /// the `gpui::App` rather than just `Shell`'s own state.
    pub(super) fn run_pending_effects(&mut self, cx: &mut Context<'_, Self>) {
        if let Some(change) = self.chrome.pending_colour_change.take() {
            change.apply(cx);
        }
    }
}

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

/// The one character an unmodified keystroke types into a text field, or `None` for a modified
/// key (a chord this field gives no meaning to) or anything that isn't a single character.
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
