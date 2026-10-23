//! Keystroke dispatch of `Shell`: the steps of the one key route (`crate::input::route_key`) over
//! the Shell's state, and the handlers they reach. An `impl Shell` block for the concern.

use std::time::Duration;

use gpui::{Context, KeyDownEvent, Keystroke};
use lib_shell::{DialogCommand, ShellCommand, ShellInput, ShellKey};
use lib_toast::ToastKind;

use super::Shell;
use crate::chrome::dialog_entity::DialogHandle;
use crate::input::{self, KeyRoute, Offered};
use crate::navigation::chord::chord;
use crate::{
    chrome::dialog_host::{self, Dialog, DialogKey, DialogOutcome},
    chrome::window_size::WindowSize,
    navigation::{
        explorer::ExplorerMode,
        movement::Movement,
        nav::{FocusZone, InputMode},
    },
    view::{
        active::{ActiveViewEntity, ShellView},
        event::ViewMode,
    },
};

/// The handoff's own "Jumps" timeout: a `g` with no completing key within this window is
/// abandoned rather than left waiting indefinitely.
pub const PENDING_G_TIMEOUT: Duration = Duration::from_millis(1000);

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

    /// Offers a keystroke to each step of the key route in turn (`crate::input`). Returns `false`
    /// for a keystroke that changed nothing (nothing to redraw).
    pub(super) fn handle_key_down(
        &mut self,
        event: &KeyDownEvent,
        window_size: WindowSize,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        let keystroke = &event.keystroke;
        // The top bar's recent-actions list is a mouse affordance; Esc closes it and nothing else.
        if self.chrome.undo_menu_open && keystroke.key == "escape" {
            self.chrome.undo_menu_open = false;
            return true;
        }
        input::route_key(&mut Keys {
            shell: self,
            keystroke,
            window_size,
            cx,
            had_status_message: false,
        })
    }

    /// Acts on a global command, one the Shell keeps or one it fell back on. Returns whether the
    /// window needs redrawing.
    fn run_global(
        &mut self,
        command: ShellCommand,
        window_size: WindowSize,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        match command {
            ShellCommand::Help => self.open_help(cx),
            ShellCommand::Command => self.open_palette(),
            ShellCommand::Search => self.start_search(cx),
            // Mirrors the TopBar's own rail-toggle button (`Shell::handle_toggle_rail`).
            ShellCommand::ToggleRail => self.toggle_rail(window_size, cx),
            ShellCommand::FocusPrevious => self.nav.cycle_focus_backward(),
            ShellCommand::FocusNext => self.nav.cycle_focus_forward(),
            // `Esc` asks the View, since only it knows what of its own is open. With nothing to
            // close it is a true no-op, which leaves a showing status message alone.
            ShellCommand::Back => {
                return (self.view_owns_mode() || self.view_has_keys())
                    && self.active_view_entity(cx).escape(cx);
            }
            ShellCommand::DismissToasts => {
                self.chrome.toasts.dismiss_all();
                self.chrome.status_message = None;
            }
            ShellCommand::ToastHistory => {
                self.open_toast_history();
                self.chrome.status_message = None;
            }
            ShellCommand::Jump(noun) => {
                self.nav.set_noun(noun);
                self.reset_view_scroll(cx);
            }
            ShellCommand::Insert => {
                self.nav.enter_mode(InputMode::Insert);
                self.view_mode = None;
            }
            ShellCommand::Move(_) | ShellCommand::Select => {
                if let Some(movement) = Movement::of(command) {
                    self.apply_movement(movement, cx);
                }
            }
        }
        true
    }

    /// `Esc` in a mode, once the View has kept nothing open: closes the palette, the file explorer
    /// and any Dialog without applying it, and leaves the mode.
    fn close_popups_and_exit_mode(&mut self) {
        self.chrome.palette = None;
        self.file_explorer = None;
        // The README's own Dialog lifecycle table: "`esc` closes any dialog without saving" --
        // discards whatever was typed, same as Cancel.
        self.close_dialog();
        self.nav.exit_mode();
        self.view_mode = None;
    }

    /// Starts the timer that ends a waiting prefix with [`ShellInput::PrefixExpired`]. The
    /// executor's timer, not `Timer::after`, so headless tests can advance it.
    fn start_prefix_timer(&mut self, cx: &mut Context<'_, Self>) {
        self.prefix_timer = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(PENDING_G_TIMEOUT).await;
            // `Err` only once the window, and with it the Shell, has gone.
            this.update(cx, |shell, cx| {
                shell.prefix_timer = None;
                shell.send_shell(ShellInput::PrefixExpired, cx);
            })
            .ok();
        }));
    }

    /// Whether the View zone has keyboard focus in `Normal` mode, so the focused View reads keys.
    fn view_has_keys(&self) -> bool {
        self.nav.focus() == FocusZone::View && self.nav.mode() == InputMode::Normal
    }

    /// Whether a key reaches the active View's own handler: the View zone has focus, in `Normal`
    /// mode, with no `Ctrl`, `Alt` or `Cmd` held (ADR-0037). `Shift` is the View's to read.
    fn view_takes_key(&self, keystroke: &Keystroke) -> bool {
        let modifiers = &keystroke.modifiers;
        self.view_has_keys() && !(modifiers.control || modifiers.alt || modifiers.platform)
    }

    /// Whether a key reaches the active View's own handler ahead of the Shell's fallback: as
    /// [`Self::view_takes_key`], and every key in an `Insert` or `Filter` mode the View entered for
    /// something it opened itself (`ViewEvent::EnterMode`). The Shell swallows every key in those
    /// modes, so only the View can use one; `Esc` comes back to the View as `escape`. A Search the
    /// View entered reads its keys through `search_key` instead.
    fn view_takes_key_ahead(&self, keystroke: &Keystroke) -> bool {
        (self.view_owns_mode() && self.nav.mode() != InputMode::Search)
            || self.view_takes_key(keystroke)
    }

    /// `/` (or the hint strip's `/ search`): the active View's Search, when it has one. A View
    /// with no Search leaves `/` inert rather than opening a mode with no box.
    pub(super) fn start_search(&mut self, cx: &mut Context<'_, Self>) {
        let view = self.active_view_entity(cx);
        if !matches!(view, ActiveViewEntity::Shell(_)) {
            if view.start_search(cx) {
                self.nav.enter_mode(InputMode::Search);
                self.view_mode = Some(ViewMode::Search);
            }
            return;
        }
        self.nav.enter_mode(InputMode::Search);
    }

    /// The active View's own keys, taken ahead of the Shell's fallback.
    fn handle_view_key_ahead(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        match self.active_view_entity(cx) {
            ActiveViewEntity::Shell(ShellView::Placeholder(_)) => false,
            view => {
                if !self.view_takes_key_ahead(keystroke) {
                    return false;
                }
                view.key_ahead(keystroke, cx)
            }
        }
    }

    /// Routes a keystroke while the palette is open (step 2, "popup-owned keys" -- mirroring
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

    /// Routes a keystroke while `InputMode::Search` is active (step 2, mirroring
    /// [`Self::handle_palette_key`]'s shape): to the active View's Search. A no-op everywhere
    /// else.
    fn handle_search_key(&mut self, keystroke: &Keystroke, cx: &mut gpui::App) -> bool {
        let view = self.active_view_entity(cx);
        if !matches!(view, ActiveViewEntity::Shell(_)) {
            return view.search_key(keystroke, cx);
        }
        false
    }

    /// Routes a keystroke while a legacy Dialog is open (step 1). A Dialog in the legacy Dialog host
    /// goes through [`chrome::dialog_host::handle_key`]. The step returns before the global keymap
    /// is asked, so the shell-wide zones stay untouched while a Dialog is up. Each key the Dialog
    /// takes is one input, whose commit saves what its confirm wrote.
    fn handle_dialog_key(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        if self.chrome.dialog.is_none() {
            return false;
        }
        input::send(cx, |cx| {
            let Some(dialog) = self.chrome.dialog.as_mut() else {
                return false;
            };
            match dialog_host::handle_key(dialog, dialog_key(keystroke)) {
                DialogOutcome::Ignored => false,
                DialogOutcome::Handled => true,
                DialogOutcome::Confirm => {
                    self.confirm_open_dialog(cx);
                    true
                }
            }
        })
    }

    /// A key while a module's Dialog is open, as one input: resolved through the Dialog keymap,
    /// with `back` the host's own (an open select closes first, otherwise the Dialog cancels and
    /// closes), and any key the keymap doesn't bind typed into the focused field. Like every key it
    /// clears a status message.
    fn handle_hosted_dialog_key(
        &mut self,
        dialog: &DialogHandle,
        keystroke: &Keystroke,
        cx: &mut Context<'_, Self>,
    ) {
        self.chrome.status_message = None;
        let command = chord(keystroke).and_then(|chord| self.chrome.dialog_keymap.event(&chord));
        input::send(cx, |cx| match command {
            Some(DialogCommand::Back) => {
                if !dialog.close_open_select(cx) {
                    dialog.cancel(cx);
                    self.close_dialog();
                }
            }
            Some(command) => dialog.command(command, cx),
            None => dialog.typed(dialog_key(keystroke), cx),
        });
    }

    /// Runs what the key handlers left pending for after the `cx.notify()` -- effects that need
    /// the `gpui::App` rather than just `Shell`'s own state.
    pub(super) fn run_pending_effects(&mut self, cx: &mut Context<'_, Self>) {
        if let Some(change) = self.chrome.pending_colour_change.take() {
            change.apply(cx);
        }
    }
}

/// One keystroke on its way through the key route, with the Shell it acts on.
struct Keys<'a, 'b> {
    shell: &'a mut Shell,
    keystroke: &'a Keystroke,
    window_size: WindowSize,
    cx: &'a mut Context<'b, Shell>,
    /// Whether the key cleared a status message, which asks for a redraw on its own.
    had_status_message: bool,
}

impl KeyRoute for Keys<'_, '_> {
    /// A module's Dialog reads its keys through the Dialog keymap; a legacy Dialog through its own
    /// handler, with `Esc` closing its open select first, otherwise the Dialog unapplied.
    fn dialog(&mut self) -> Option<bool> {
        let shell = &mut *self.shell;
        if !shell.presenter.dialog_open() {
            return None;
        }
        if let Some(dialog) = shell.hosted_dialog().cloned() {
            shell.handle_hosted_dialog_key(&dialog, self.keystroke, self.cx);
            return Some(true);
        }
        if self.keystroke.key == "escape" {
            if !shell
                .chrome
                .dialog
                .as_mut()
                .is_some_and(Dialog::close_open_select)
            {
                shell.close_popups_and_exit_mode();
            }
            return Some(true);
        }
        let had_status_message = shell.chrome.status_message.take().is_some();
        Some(shell.handle_dialog_key(self.keystroke, self.cx) || had_status_message)
    }

    /// The palette, Search, a filter, the file explorer or Insert takes every key. A mode the View
    /// entered for something of its own reads them first, `Esc` included.
    fn mode(&mut self) -> Option<bool> {
        let shell = &mut *self.shell;
        let mode = shell.nav.mode();
        if mode == InputMode::Normal {
            return None;
        }
        if self.keystroke.key == "escape" {
            if !(shell.view_owns_mode() && shell.active_view_entity(self.cx).escape(self.cx)) {
                shell.close_popups_and_exit_mode();
            }
            return Some(true);
        }
        if shell.view_owns_mode()
            && mode != InputMode::Search
            && shell.handle_view_key_ahead(self.keystroke, self.cx)
        {
            shell.chrome.status_message = None;
            return Some(true);
        }
        // The handoff's own precedent for hint-strip messages (`docs/ux/desktop-mockups/README.md`'s
        // "Loading and error states"): any keypress clears one, not just a timer.
        let had_status_message = shell.chrome.status_message.take().is_some();
        let taken = match mode {
            InputMode::Command => shell.handle_palette_key(self.keystroke, self.cx),
            InputMode::Search => shell.handle_search_key(self.keystroke, self.cx),
            // Only a View opens a filter popover, and it reads the popover's keys above; what it
            // leaves is swallowed, as is every key in Insert, which has no input surface yet.
            InputMode::Filter | InputMode::Insert | InputMode::Dialog | InputMode::Normal => false,
        };
        Some(taken || had_status_message)
    }

    /// The Shell's Presenter resolves the key through the global keymap, as one input; the root
    /// Router hands back the commands it kept, which the Shell then acts on.
    fn global(&mut self) -> Offered {
        let shell = &mut *self.shell;
        // Debug builds only: F9 raises each Kind in turn, to eyeball the layer before real call
        // sites exist (#346).
        #[cfg(debug_assertions)]
        if self.keystroke.key == "f9" {
            let kind = ToastKind::ALL[shell.chrome.debug_toast_kind % ToastKind::ALL.len()];
            shell.chrome.debug_toast_kind += 1;
            shell.raise_toast(kind, format!("{kind:?} Toast raised from F9"));
            return Offered::Took(true);
        }
        let Some(chord) = chord(self.keystroke) else {
            return Offered::Passed {
                chord: String::new(),
                fallback: None,
            };
        };
        // Dropping the timer cancels it, so it can't end a prefix begun by this key.
        shell.prefix_timer = None;
        // Every key clears a status message but `back`, which leaves one showing when it has
        // nothing to close.
        if shell.presenter.keymap().event(&chord) != Some(ShellCommand::Back) {
            self.had_status_message = shell.chrome.status_message.take().is_some();
        }
        let (answer, ran) = shell.resolve_key(&chord, self.cx);
        match answer {
            ShellKey::Taken => {
                let mut redraw = true;
                for command in ran {
                    redraw = shell.run_global(command, self.window_size, self.cx);
                }
                Offered::Took(redraw)
            }
            ShellKey::Prefixed => {
                shell.start_prefix_timer(self.cx);
                Offered::Took(self.had_status_message)
            }
            ShellKey::Passed { chord, fallback } => Offered::Passed { chord, fallback },
        }
    }

    fn module(&mut self, chord: &str) -> bool {
        let shell = &mut *self.shell;
        shell.view_has_keys() && shell.active_view_entity(self.cx).module_key(chord, self.cx)
    }

    /// The View's keys that sit ahead of the Shell's fallback (Budgets' `tab`, the Import step,
    /// the Documents keys, the Settings pages' own keys).
    fn view(&mut self) -> bool {
        self.shell.handle_view_key_ahead(self.keystroke, self.cx)
    }

    fn fall_back(&mut self, fallback: Option<ShellCommand>, sequence: bool) -> bool {
        let shell = &mut *self.shell;
        if let Some(command) = fallback {
            return shell.run_global(command, self.window_size, self.cx);
        }
        let handled = !sequence
            && match shell.active_view_entity(self.cx) {
                ActiveViewEntity::Shell(ShellView::Placeholder(_)) => false,
                view => {
                    shell.view_takes_key(self.keystroke) && view.handle_key(self.keystroke, self.cx)
                }
            };
        handled || self.had_status_message
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
fn typed_char(keystroke: &Keystroke) -> Option<char> {
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
