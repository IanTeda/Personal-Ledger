//! The shell's status line (`docs/ux/desktop-mockups/01-shell/README.md`'s "1a" spec,
//! "Status line" component): mode badge, hint strip. Its own bottom-right breadcrumb (`ledger ·
//! <noun>`) was dropped -- the active View's own heading names the screen, so a breadcrumb here
//! was a plain duplicate (as was the top bar's View suffix, `crate::shell::view::topbar::brand_mark`).

use std::path::{Path, PathBuf};
use std::rc::Rc;

use gpui::{App, FontWeight, Pixels, Window, div, prelude::*, px};

use lib_shell::{Hint, ShellCommand, ShellViewModel};
use lib_toast::ToastKind;

use crate::{
    navigation::nav::InputMode,
    shell::view::{
        text_fit::{hints_that_fit, text_width},
        window_size::{WindowSize, WindowSizeExt},
    },
    theme::color,
};

const PADDING_X: Pixels = px(12.0);
const GAP: Pixels = px(14.0);
const TEXT_SIZE: Pixels = px(11.5);
const BADGE_SIZE: Pixels = px(10.0);
const BADGE_PADDING_X: Pixels = px(7.0);
/// Between a hint's key and its action, and either side of the `·` between hints.
const HINT_GAP: Pixels = px(4.0);

/// Band height: `docs/ux/desktop-mockups/01-shell/README.md`'s "Layout" table.
pub const HEIGHT: Pixels = px(28.0);

/// A page's own status-line content, replacing the shell-wide hint strip and file path while that
/// page is showing: its key legend and its right-aligned text.
#[derive(Debug, Clone)]
pub struct PageStatus {
    pub hints: Vec<Hint>,
    pub right: String,
}

/// The shell-wide hint strip's clickable entries -- each does what its key does in `Normal` mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HintAction {
    Command,
    Search,
    Help,
    ToggleRail,
}

pub type OnHint = Rc<dyn Fn(HintAction, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct StatusLine {
    on_hint: Option<OnHint>,
    mode: InputMode,
    /// Replaces the hint strip when `Some` -- the handoff's "Loading and error states" rule
    /// ("the hint strip is replaced by the error ... cleared by any keypress"), reused here
    /// for the `g`-jump prefix's own "flash the hint strip" abort message and the command
    /// palette's own "not yet built" message (the Shell Presenter's status message), not just write
    /// failures. Ignored while `palette_query` is `Some` -- the "1d" spec's own COMMAND-mode
    /// status line has no room for both the live query and a message, and a message can only
    /// be showing here right after the palette that set it has already closed.
    status_message: Option<String>,
    /// The Shell's own hints, keyed as the global keymap binds them now.
    shell_hints: Vec<(ShellCommand, Hint)>,
    /// The active command-mode echo text and its own "esc closes ..." hint, `Some` while
    /// `InputMode::Command` has something other than the ordinary hint strip to show: either
    /// the palette's live input (`docs/ux/desktop-mockups/01-shell/README.md`'s "1d" spec:
    /// "Status line in COMMAND mode: ... the live query echoes ...; right side reads 'esc close
    /// command window'"), or -- once `:open` is confirmed -- the "1e" file explorer's own
    /// frozen `"open"` echo and "esc close file explorer" hint, since `Root` keeps `mode` at
    /// `Command` for as long as that dialog is open (see `Root::run_command`'s own doc).
    command_echo: Option<(String, String)>,
    /// `Some` while a page with its own legend is showing (`docs/ux/desktop-mockups/17-accounts/README.md`'s
    /// 3a status bar). Ignored in command mode, which owns the whole line.
    page: Option<PageStatus>,
    /// The status-line echo (ADR-0027): the Toast carried here while Toasts are off. Below
    /// `status_message`, above the page legend and hint strip.
    toast_echo: Option<(ToastKind, String)>,
    /// Replaces the mode badge's text: a page's own mode (6e's `IMPORT`), drawn like `Normal`.
    mode_label: Option<String>,
    /// Whether a Ledger is open: the right-hand side names its file, or says none is open.
    ledger_open: bool,
    /// The Open Ledger's file, `None` on a test's in-memory stores, which show the stub's.
    ledger_file: Option<PathBuf>,
}

impl StatusLine {
    /// The status line over the Shell's view model: its mode, status message and hints.
    pub fn new(shell: ShellViewModel, command_echo: Option<(String, String)>) -> Self {
        Self {
            on_hint: None,
            mode: shell.mode,
            status_message: shell.status,
            shell_hints: shell.hints,
            command_echo,
            page: None,
            toast_echo: None,
            mode_label: None,
            ledger_open: true,
            ledger_file: None,
        }
    }

    /// Says whether a Ledger is open, and its file, for the right-hand side's path or "No ledger
    /// open…".
    pub fn ledger(mut self, open: bool, file: Option<PathBuf>) -> Self {
        self.ledger_open = open;
        self.ledger_file = file;
        self
    }

    pub fn toast_echo(mut self, echo: Option<(ToastKind, String)>) -> Self {
        self.toast_echo = echo;
        self
    }

    /// Makes the shell-wide hint strip's entries clickable.
    pub fn on_hint(mut self, on_hint: OnHint) -> Self {
        self.on_hint = Some(on_hint);
        self
    }

    pub fn mode_label(mut self, label: Option<String>) -> Self {
        self.mode_label = label;
        self
    }

    pub fn page(mut self, page: Option<PageStatus>) -> Self {
        self.page = page;
        self
    }
}

impl RenderOnce for StatusLine {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let window_size = WindowSize::of(window);
        let right = right_side(
            self.command_echo.as_ref(),
            self.page.as_ref(),
            ledger_label(
                self.ledger_open,
                self.ledger_file.as_deref(),
                window_size,
                HOME.as_deref(),
            ),
        );
        let badge_label = lib_locale::format::upper(
            &self
                .mode_label
                .clone()
                .unwrap_or_else(|| mode_label(self.mode)),
        );
        // What the mode chip and the context note leave the hints; they never shrink.
        let room = window.viewport_size().width
            - PADDING_X * 2.0
            - GAP * 3.0
            - text_width(window, &badge_label, BADGE_SIZE, FontWeight::EXTRA_BOLD)
            - BADGE_PADDING_X * 2.0
            - text_width(window, &right, TEXT_SIZE, FontWeight::NORMAL);
        let fit = |hints: &[Hint]| -> usize {
            let widths: Vec<Pixels> = hints
                .iter()
                .map(|hint| {
                    text_width(window, &hint.chord, TEXT_SIZE, FontWeight::EXTRA_BOLD)
                        + HINT_GAP
                        + text_width(window, &hint.action, TEXT_SIZE, FontWeight::NORMAL)
                })
                .collect();
            let separator =
                HINT_GAP * 2.0 + text_width(window, "\u{b7}", TEXT_SIZE, FontWeight::NORMAL);
            hints_that_fit(&widths, separator, room)
        };
        div()
            .h(HEIGHT)
            .flex_none()
            .flex()
            .items_center()
            .gap(GAP)
            .px(PADDING_X)
            .bg(color::chrome(cx))
            .border_t(px(2.0))
            .border_color(color::structural_rule(cx))
            .text_size(TEXT_SIZE)
            .text_color(color::muted(cx))
            .whitespace_nowrap()
            .child(mode_badge(self.mode, badge_label, cx))
            .child(
                match left(
                    self.command_echo.as_ref().map(|(query, _)| query.as_str()),
                    self.status_message,
                    self.toast_echo,
                    self.page.as_ref(),
                ) {
                    Left::CommandEcho(query) => command_query_echo(query, cx).into_any_element(),
                    Left::StatusMessage(message) => div()
                        .font_weight(FontWeight::EXTRA_BOLD)
                        .text_color(color::accent_text(cx))
                        .child(message)
                        .into_any_element(),
                    Left::ToastEcho(kind, text) => toast_echo(kind, text, cx).into_any_element(),
                    Left::PageHints(hints) => {
                        let shown = fit(hints);
                        page_hint_strip(&hints[..shown], cx).into_any_element()
                    }
                    Left::HintStrip => {
                        let entries = shell_hint_entries(&self.shell_hints);
                        let shown = fit(&shell_hints(&entries));
                        hint_strip(self.on_hint.clone(), entries, shown, cx).into_any_element()
                    }
                },
            )
            .child(div().flex_1())
            .child(
                div()
                    .debug_selector(|| "status-right".to_string())
                    .flex_none()
                    .child(right),
            )
    }
}

/// What the left side of the status line shows, highest precedence first (ADR-0027): command
/// echo > status-line message > Toast echo > page legend / shell hint strip.
#[derive(Debug, PartialEq, Eq)]
enum Left<'a> {
    CommandEcho(&'a str),
    StatusMessage(String),
    ToastEcho(ToastKind, String),
    PageHints(&'a [Hint]),
    HintStrip,
}

fn left<'a>(
    command_echo: Option<&'a str>,
    status_message: Option<String>,
    toast_echo: Option<(ToastKind, String)>,
    page: Option<&'a PageStatus>,
) -> Left<'a> {
    if let Some(query) = command_echo {
        Left::CommandEcho(query)
    } else if let Some(message) = status_message {
        Left::StatusMessage(message)
    } else if let Some((kind, text)) = toast_echo {
        Left::ToastEcho(kind, text)
    } else if let Some(page) = page {
        Left::PageHints(&page.hints)
    } else {
        Left::HintStrip
    }
}

/// The Kind glyph in its mark colour, then the Message in `foreground`, as a Toast reads.
fn toast_echo(kind: ToastKind, text: String, cx: &App) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(px(6.0))
        .text_color(color::foreground(cx))
        .child(
            div()
                .font_weight(FontWeight::EXTRA_BOLD)
                .text_color(color::toast_mark(kind, cx))
                .child(kind.glyph().to_string()),
        )
        .child(text)
}

/// The mode chip's own label, before a View's override.
fn mode_label(mode: InputMode) -> String {
    match mode {
        InputMode::Normal => crate::msg::desktop_mode_normal(),
        InputMode::Insert => crate::msg::desktop_mode_insert(),
        InputMode::Search => crate::msg::desktop_mode_search(),
        InputMode::Command => crate::msg::desktop_mode_command(),
        InputMode::Dialog => crate::msg::desktop_mode_dialog(),
        InputMode::Filter => crate::msg::desktop_mode_filter(),
    }
}

fn mode_badge(mode: InputMode, label: String, cx: &App) -> impl IntoElement {
    let bg = match mode {
        InputMode::Normal | InputMode::Insert | InputMode::Search => color::foreground(cx),
        InputMode::Command | InputMode::Dialog | InputMode::Filter => color::accent(cx),
    };

    div()
        .flex_none()
        .bg(bg)
        .text_color(color::background(cx))
        .font_weight(FontWeight::EXTRA_BOLD)
        .text_size(BADGE_SIZE)
        .py(px(2.0))
        .px(BADGE_PADDING_X)
        .child(label)
}

/// A shell-wide hint as the strip draws it: `(id, action, hint)`.
pub(crate) type HintEntry = (&'static str, HintAction, Hint);

/// The Shell's hints, in the order they drop from the end, each with its tag and click action.
pub(crate) fn shell_hint_entries(hints: &[(ShellCommand, Hint)]) -> Vec<HintEntry> {
    hints
        .iter()
        .filter_map(|(command, hint)| {
            let (id, action) = match command {
                ShellCommand::Command => ("hint-command", HintAction::Command),
                ShellCommand::Search => ("hint-search", HintAction::Search),
                ShellCommand::Help => ("hint-help", HintAction::Help),
                ShellCommand::ToggleRail => ("hint-rail", HintAction::ToggleRail),
                _ => return None,
            };
            Some((id, action, hint.clone()))
        })
        .collect()
}

/// The shell-wide hints alone, for measuring.
pub(crate) fn shell_hints(entries: &[HintEntry]) -> Vec<Hint> {
    entries.iter().map(|(_, _, hint)| hint.clone()).collect()
}

/// The shell-wide hints, the first `shown` of them, each a click target for its key's action. The
/// strip is tagged with `shown`: gpui keeps an element's bounds after it stops drawing, so a test
/// can't see a hint drop by its own tag, only a count that was never drawn before.
fn hint_strip(
    on_hint: Option<OnHint>,
    entries: Vec<HintEntry>,
    shown: usize,
    cx: &App,
) -> impl IntoElement {
    let entry = |id: &'static str, action: HintAction, hint: Hint| {
        let on_hint = on_hint.clone();
        div()
            .id(id)
            .debug_selector(|| id.to_string())
            .flex()
            .items_center()
            .gap(HINT_GAP)
            .when(on_hint.is_some(), |this| {
                this.cursor_pointer()
                    .hover(|style| style.bg(color::hover(cx)))
            })
            .when_some(on_hint, |this, on_hint| {
                this.on_click(move |_event, window, cx| on_hint(action, window, cx))
            })
            .child(
                div()
                    .font_weight(FontWeight::EXTRA_BOLD)
                    .text_color(color::foreground(cx))
                    .child(hint.chord),
            )
            .child(hint.action)
            .into_any_element()
    };

    div()
        .debug_selector(move || format!("status-hints-shown-{shown}"))
        .flex()
        .items_center()
        .gap(HINT_GAP)
        .children(entries.into_iter().take(shown).enumerate().flat_map(
            |(index, (id, action, hint))| {
                let separator = (index > 0).then(|| div().child("\u{b7}").into_any_element());
                separator.into_iter().chain([entry(id, action, hint)])
            },
        ))
}

/// A page's key legend: `j/k row · enter open ledger · ...`, each key at weight 800 like the
/// shell-wide hint strip's own. Tagged with how many hints it shows, as [`hint_strip`] is.
fn page_hint_strip(hints: &[Hint], cx: &App) -> impl IntoElement {
    let shown = hints.len();
    div()
        .debug_selector(move || format!("status-page-hints-shown-{shown}"))
        .flex()
        .items_center()
        .gap(HINT_GAP)
        .children(hints.iter().enumerate().flat_map(|(index, hint)| {
            let separator = (index > 0).then(|| div().child("\u{b7}").into_any_element());
            separator.into_iter().chain([div()
                .debug_selector(move || format!("status-hint-{index}"))
                .flex()
                .items_center()
                .gap(HINT_GAP)
                .child(
                    div()
                        .font_weight(FontWeight::EXTRA_BOLD)
                        .text_color(color::foreground(cx))
                        .child(hint.chord.clone()),
                )
                .child(hint.action.clone())
                .into_any_element()])
        }))
}

/// The "1d" spec's COMMAND-mode echo: `:{query}` at weight 800, then the block caret --
/// the status line's own smaller echo of the palette's input row, kept in sync by `Root`
/// passing the same `Palette::input()` string to both.
fn command_query_echo(query: &str, cx: &App) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(px(4.0))
        .font_weight(FontWeight::EXTRA_BOLD)
        .text_color(color::foreground(cx))
        .child(format!(":{query}"))
        .child(div().w(px(2.0)).h(px(13.0)).bg(color::cursor(cx)))
}

/// The status line's right-hand side: the command surface's "esc closes" hint, else the page's
/// note, else `ledger`, the Open Ledger's file or "No ledger open…" ([`ledger_label`]).
fn right_side(
    command_echo: Option<&(String, String)>,
    page: Option<&PageStatus>,
    ledger: String,
) -> String {
    match (command_echo, page) {
        (Some((_, hint)), _) => hint.clone(),
        (None, Some(page)) => page.right.clone(),
        (None, None) => ledger,
    }
}

/// What the status line says of the Open Ledger: its file's path with `home` as `~`, shortened to
/// its file name at Medium (DESKTOP-116), or "No ledger open…" with none open. A test's in-memory
/// stores have no file, so show the stub's.
pub fn ledger_label(
    open: bool,
    file: Option<&Path>,
    window_size: WindowSize,
    home: Option<&Path>,
) -> String {
    if !open {
        return crate::msg::desktop_status_no_ledger_file();
    }
    let name = file
        .and_then(Path::file_name)
        .map_or(STUB_LEDGER_FILE.into(), |name| name.to_string_lossy());
    match (window_size, file) {
        (WindowSize::Medium, _) => format!("\u{2026}/{name}"),
        (WindowSize::Large | WindowSize::XLarge, None) => format!("~/Documents/{name}"),
        (WindowSize::Large | WindowSize::XLarge, Some(file)) => {
            match home.and_then(|home| file.strip_prefix(home).ok()) {
                Some(relative) => format!("~/{}", relative.display()),
                None => file.display().to_string(),
            }
        }
    }
}

/// The home directory the Ledger's path is shown under, read once rather than on every frame.
static HOME: std::sync::LazyLock<Option<PathBuf>> = std::sync::LazyLock::new(dirs::home_dir);

/// The stub Ledger's file name, shared with the Documents status line's "stored beside" note until
/// a real open-file path exists.
pub const STUB_LEDGER_FILE: &str = "My-Personal-Ledger.pldb";

#[cfg(test)]
mod tests {
    use super::*;

    fn echo() -> Option<(ToastKind, String)> {
        Some((ToastKind::Success, "Deleted tag Food".to_string()))
    }

    #[test]
    fn with_no_ledger_open_the_right_side_says_so_unless_a_page_or_command_has_it() {
        crate::locale::init_for_tests();
        assert_eq!(
            ledger_label(false, None, WindowSize::Large, None),
            "No ledger open…"
        );
        let page = PageStatus {
            hints: Vec::new(),
            right: "this device".to_string(),
        };
        assert_eq!(
            right_side(None, Some(&page), "No ledger open…".to_string()),
            "this device"
        );
        let echo = (String::new(), "esc close command window".to_string());
        assert_eq!(
            right_side(Some(&echo), None, "No ledger open…".to_string()),
            "esc close command window"
        );
        assert_eq!(
            right_side(None, None, "No ledger open…".to_string()),
            "No ledger open…"
        );
    }

    #[test]
    fn the_open_ledger_shows_its_path_with_the_home_directory_as_a_tilde() {
        let home = Path::new("/home/someone");
        let file = Path::new("/home/someone/ledgers/household.pldb");
        assert_eq!(
            ledger_label(true, Some(file), WindowSize::Large, Some(home)),
            "~/ledgers/household.pldb"
        );
        assert_eq!(
            ledger_label(true, Some(file), WindowSize::Medium, Some(home)),
            "\u{2026}/household.pldb"
        );
        let elsewhere = Path::new("/srv/ledgers/shared.pldb");
        assert_eq!(
            ledger_label(true, Some(elsewhere), WindowSize::XLarge, Some(home)),
            "/srv/ledgers/shared.pldb"
        );
    }

    #[test]
    fn the_toast_echo_replaces_the_hint_strip_and_the_page_legend() {
        let page = PageStatus {
            hints: vec![Hint::new("j/k", "row")],
            right: String::new(),
        };
        assert_eq!(left(None, None, None, None), Left::HintStrip);
        assert_eq!(
            left(None, None, None, Some(&page)),
            Left::PageHints(&page.hints)
        );
        let echoed = Left::ToastEcho(ToastKind::Success, "Deleted tag Food".to_string());
        assert_eq!(left(None, None, echo(), None), echoed);
        assert_eq!(left(None, None, echo(), Some(&page)), echoed);
    }

    #[test]
    fn a_status_line_message_and_the_command_echo_outrank_the_toast_echo() {
        let message = Some("not yet built".to_string());
        assert_eq!(
            left(None, message.clone(), echo(), None),
            Left::StatusMessage("not yet built".to_string())
        );
        assert_eq!(
            left(Some("acc"), message, echo(), None),
            Left::CommandEcho("acc")
        );
    }
}
