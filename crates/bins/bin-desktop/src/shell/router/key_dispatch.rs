//! Keystroke dispatch of `Root`: the steps of the one key route (`crate::input::route_key`) over
//! the Shell's state, and the handlers they reach. An `impl Root` block for the concern.

use std::time::Duration;

use gpui::{Context, KeyDownEvent, Keystroke};
use lib_shell::{DialogCommand, ExplorerOutcome, ShellCommand, ShellInput, ShellKey};
use lib_toast::ToastKind;

use super::Root;
use crate::input::{self, KeyRoute, Offered};
use crate::navigation::chord::chord;
use crate::shell::view::dialog_entity::DialogHandle;
use crate::{
    navigation::nav::{FocusZone, InputMode},
    shell::view::dialog_entity::DialogKey,
    shell::view::window_size::WindowSize,
    view::active::ActiveViewEntity,
};
use lib_shell::NavInput;

/// The handoff's own "Jumps" timeout: a `g` with no completing key within this window is
/// abandoned rather than left waiting indefinitely.
pub const PENDING_G_TIMEOUT: Duration = Duration::from_millis(1000);

impl Root {
    /// The status line's own COMMAND-mode echo (`crate::shell::view::statusline::StatusLine`'s
    /// `command_echo`): the palette's live input while it's open, or -- once `:open` or `:new` has
    /// been confirmed and the palette has closed in its favour -- the file explorer's frozen
    /// `"open"` or `"new"` echo, each read from its Presenter and paired with its own "esc closes
    /// ..." hint text, under the key `back` is bound to now.
    pub(super) fn command_echo(&self) -> Option<(String, String)> {
        if let Some(palette) = self.presenter.palette() {
            Some((
                palette.input,
                crate::msg::desktop_status_close_command_window(&palette.keys.close),
            ))
        } else if let Some(explorer) = self.presenter.explorer() {
            Some((
                explorer.mode.command().to_string(),
                crate::msg::desktop_status_close_file_explorer(&explorer.close_key),
            ))
        } else {
            None
        }
    }

    /// Offers a keystroke to each step of the key route in turn (`crate::input`). Returns `false`
    /// for a keystroke that changed nothing (nothing to redraw). Every key first clears the status
    /// message, before any step handles it, so a message the key sets answers it alone.
    pub(super) fn handle_key_down(
        &mut self,
        event: &KeyDownEvent,
        window_size: WindowSize,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        let keystroke = &event.keystroke;
        let cleared = self.presenter.keypress();
        // The top bar's recent-actions list is a mouse affordance; Esc closes it and nothing else.
        if self.state.undo_menu_open && keystroke.key == "escape" {
            self.state.undo_menu_open = false;
            return true;
        }
        let redraw = input::route_key(&mut Keys {
            shell: self,
            keystroke,
            window_size,
            cx,
        });
        redraw || cleared
    }

    /// `Esc` in a mode, once the View has kept nothing open: closes the palette, the file explorer
    /// and any Dialog without applying it, and leaves the mode.
    fn close_popups_and_exit_mode(&mut self) {
        self.presenter.close_palette();
        self.presenter.close_explorer();
        // The README's own Dialog lifecycle table: "`esc` closes any dialog without saving" --
        // discards whatever was typed, same as Cancel.
        self.close_dialog();
        self.navigate(NavInput::ExitMode);
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
        self.nav().focus() == FocusZone::View && self.nav().mode() == InputMode::Normal
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
        (self.view_owns_mode() && self.nav().mode() != InputMode::Search)
            || self.view_takes_key(keystroke)
    }

    /// The active View's own keys, taken ahead of the Shell's fallback.
    fn handle_view_key_ahead(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        match self.active_view_entity(cx) {
            ActiveViewEntity::Shell(_) => false,
            view => {
                if !self.view_takes_key_ahead(keystroke) {
                    return false;
                }
                view.key_ahead(keystroke, cx)
            }
        }
    }

    /// A key while the palette is open, as one input: its Presenter resolves it through the
    /// Dialog keymap and its own `history_back`, types any key neither binds, and runs the Command
    /// it closes on through the root Router. The palette takes every key, so a key it gives no
    /// meaning to goes no further. The row list is kept scrolled onto the selected row.
    fn handle_palette_key(
        &mut self,
        keystroke: &Keystroke,
        window_size: WindowSize,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        let chord = chord(keystroke).unwrap_or_default();
        let typed = typed_char(keystroke);
        let changed = self.send_with_router(window_size, cx, |presenter, router| {
            presenter.palette_key(&chord, typed, router)
        });
        if let Some(row) = self
            .presenter
            .palette()
            .and_then(|model| model.selected_row)
        {
            self.state.palette_scroll.scroll_to_item(row);
        }
        changed
    }

    /// A key while the file explorer is open, as one input: its Presenter resolves it through the
    /// Dialog keymap, `back` closing it, and in `:new` types any key the keymap doesn't bind into
    /// the File name, select creating the file through the root Router, as Create does. `:open`'s
    /// explorer picks a file by mouse alone. The explorer takes every key, so a key it gives no
    /// meaning to goes no further.
    fn handle_explorer_key(
        &mut self,
        keystroke: &Keystroke,
        window_size: WindowSize,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        let chord = chord(keystroke).unwrap_or_default();
        let typed = typed_char(keystroke);
        let outcome = self.send_with_router(window_size, cx, |presenter, router| {
            presenter.explorer_key(&chord, typed, router)
        });
        outcome != ExplorerOutcome::Ignored
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

    /// A key while a module's Dialog is open, as one input: resolved through the Dialog keymap,
    /// with `back` the host's own (an open select closes first, otherwise the Dialog cancels and
    /// closes), and any key the keymap doesn't bind typed into the focused field.
    fn handle_hosted_dialog_key(
        &mut self,
        dialog: &DialogHandle,
        keystroke: &Keystroke,
        cx: &mut Context<'_, Self>,
    ) {
        let command =
            chord(keystroke).and_then(|chord| self.presenter.dialog_keymap().event(&chord));
        input::send(cx, |cx| match command {
            Some(DialogCommand::Back) => {
                if !dialog.close_open_select(cx) {
                    dialog.cancel(cx);
                    self.close_dialog();
                }
            }
            // The list step is a letter: a step only in a Dialog stepping a list, else typing.
            Some(command) if command.is_list_step() && !dialog.steps_list(cx) => {
                dialog.typed(dialog_key(keystroke), cx);
            }
            Some(command) => dialog.command(command, cx),
            None => dialog.typed(dialog_key(keystroke), cx),
        });
    }
}

/// One keystroke on its way through the key route, with the Shell it acts on.
struct Keys<'a, 'b> {
    shell: &'a mut Root,
    keystroke: &'a Keystroke,
    window_size: WindowSize,
    cx: &'a mut Context<'b, Root>,
}

impl KeyRoute for Keys<'_, '_> {
    /// A module's Dialog reads its keys through the Dialog keymap.
    fn dialog(&mut self) -> Option<bool> {
        let shell = &mut *self.shell;
        let dialog = shell.hosted_dialog().cloned()?;
        shell.handle_hosted_dialog_key(&dialog, self.keystroke, self.cx);
        Some(true)
    }

    /// The palette, Search, a filter, the file explorer or Insert takes every key. The palette and
    /// the file explorer are the Shell's own, and read their keys, `back` included, through its
    /// Presenter. A mode the View entered for something of its own reads them first, `Esc`
    /// included.
    fn mode(&mut self) -> Option<bool> {
        let shell = &mut *self.shell;
        let mode = shell.nav().mode();
        if mode == InputMode::Normal {
            return None;
        }
        if shell.presenter.palette_open() {
            return Some(shell.handle_palette_key(self.keystroke, self.window_size, self.cx));
        }
        if shell.presenter.explorer_open() {
            return Some(shell.handle_explorer_key(self.keystroke, self.window_size, self.cx));
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
            return Some(true);
        }
        let taken = match mode {
            InputMode::Search => shell.handle_search_key(self.keystroke, self.cx),
            // Only a View opens a filter popover, and it reads the popover's keys above; what it
            // leaves is swallowed, as is every key in Insert, which has no input surface yet.
            // Command mode with neither the palette nor the explorer open has nothing to type into.
            InputMode::Command
            | InputMode::Filter
            | InputMode::Insert
            | InputMode::Dialog
            | InputMode::Normal => false,
        };
        Some(taken)
    }

    /// The Shell's Presenter resolves the key through the global keymap and runs a command it
    /// keeps through the root Router, inside the key's one input.
    fn global(&mut self) -> Offered {
        let shell = &mut *self.shell;
        // Debug builds only: F9 raises each Kind in turn, to eyeball the layer before real call
        // sites exist (#346).
        #[cfg(debug_assertions)]
        if self.keystroke.key == "f9" {
            let kind = ToastKind::ALL[shell.debug_toast_kind % ToastKind::ALL.len()];
            shell.debug_toast_kind += 1;
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
        match shell.resolve_key(&chord, self.window_size, self.cx) {
            ShellKey::Taken => Offered::Took(true),
            ShellKey::Prefixed => {
                shell.start_prefix_timer(self.cx);
                Offered::Took(false)
            }
            ShellKey::Passed { chord, fallback } => Offered::Passed { chord, fallback },
        }
    }

    fn module(&mut self, chord: &str) -> bool {
        let shell = &mut *self.shell;
        shell.view_has_keys()
            && shell
                .active_view_entity(self.cx)
                .module_key(chord, self.keystroke, self.cx)
    }

    /// The View's keys that sit ahead of the Shell's fallback (a Budgets Plan cell's typing, the
    /// Documents keys, the Settings pages' own keys).
    fn view(&mut self) -> bool {
        self.shell.handle_view_key_ahead(self.keystroke, self.cx)
    }

    fn fall_back(&mut self, fallback: Option<ShellCommand>, sequence: bool) -> bool {
        let shell = &mut *self.shell;
        // The Shell's Presenter runs the command it falls back on, as one input.
        if let Some(command) = fallback {
            return shell.send_with_router(self.window_size, self.cx, |presenter, router| {
                presenter.command(command, router)
            });
        }
        !sequence
            && match shell.active_view_entity(self.cx) {
                ActiveViewEntity::Shell(_) => false,
                view => {
                    shell.view_takes_key(self.keystroke) && view.handle_key(self.keystroke, self.cx)
                }
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
