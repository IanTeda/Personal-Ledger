//! Settings › Display's Presenter: the date style, Colour Theme and Appearance, Configuration in
//! the user's configuration file (ADR-0053) written through [`DeviceConfiguration`]; Row density
//! and Status glyphs, session-only; and Start Sidebar minimised, which the Client remembers in its
//! own state file. Beside them a PREVIEW re-renders three seeded rows under the choices made.
//!
//! The date style, Row density, Status glyphs and Start Sidebar minimised are the Display output,
//! which the Presenter hands the Shell on each change ([`DeviceRouter::display_changed`]) for the
//! Views that draw with it. A saved Colour Theme or Appearance goes to the Shell to apply
//! ([`DeviceRouter::colour_changed`]); how each theme's card looks, and what the Client's own
//! `[theme]` overrides and the system's light or dark setting make of it, is the Client's to draw.

use std::rc::Rc;

use chrono::NaiveDate;
use lib_colour_theme::{ColourAppearance, ColourTheme};
use lib_core::{DateStyle, Money};
use lib_list_cursor::Step;
use lib_locale::format::{AmountStyle, format_amount, format_date};
use lib_viper::display::{RowDensity, StatusGlyphs};
use lib_viper::keymap::{Keymap, MOVE_DOWN, MOVE_UP, SELECT};
use lib_viper::view::{Choice, Hint, SelectView};

use super::device::{ControlView, PageKey, chord, chords, settle, shadowed_note, step_choice};
use crate::keymap::{NEXT_CHOICE, PREVIOUS_CHOICE};
use crate::msg;
use crate::protocols::{ColourChoice, DeviceConfiguration, DeviceKey, DeviceRouter};

/// The Date format control's choices: `None` is "Locale default", the Locale picking the form.
pub const DATE_STYLE_CHOICES: [Option<DateStyle>; 5] = [
    None,
    Some(DateStyle::Short),
    Some(DateStyle::Medium),
    Some(DateStyle::Long),
    Some(DateStyle::Iso),
];

/// The Appearance control's choices, in its order.
pub const APPEARANCES: [ColourAppearance; 3] = [
    ColourAppearance::Light,
    ColourAppearance::Dark,
    ColourAppearance::System,
];

/// The controls above the Colour Theme grid, top to bottom: Date format, Row density, Status
/// glyphs, Start Sidebar minimised. The grid is the stop after the last.
pub const DISPLAY_FIELD_COUNT: usize = 4;

/// The Display field that is a checkbox rather than a segmented or radio control.
pub const DISPLAY_FIELD_SIDEBAR: usize = 3;

/// What the Views and the Shell draw with, from Display: the Shell fans it out on each change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DisplayOutput {
    pub date_style: Option<DateStyle>,
    pub row_density: RowDensity,
    pub status_glyphs: StatusGlyphs,
    /// Applied to the primary rail at launch, so a change waits for the next one.
    pub start_sidebar_minimised: bool,
}

/// What Display is told and the person asks of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayEvent {
    /// A key from [`crate::keymap::device_page_keymap`].
    Key(PageKey),
    /// `back` (`esc`) in the Colour Theme grid: up to the control above it.
    Back,
    /// Any other key in the Colour Theme grid leaves it, as a movement the grid doesn't take does.
    LeaveGrid,
    ChooseDateStyle(Option<DateStyle>),
    ChooseRowDensity(RowDensity),
    ChooseStatusGlyphs(StatusGlyphs),
    ToggleStartSidebarMinimised,
    /// A click on a Colour Theme card, which also takes the keyboard off the grid.
    ClickColourTheme(&'static str),
    /// An Appearance click, or the palette's `theme` or `appearance`.
    ChooseColour(ColourChoice),
}

/// The Colour Theme and Appearance as start-up read them, and the date style.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DisplaySettings {
    pub date_style: Option<DateStyle>,
    /// The `colour_theme` key raw, so an id from a newer release survives; `None` is the default.
    pub colour_theme: Option<String>,
    pub colour_appearance: ColourAppearance,
}

/// What the Client hands Display as it builds it.
pub struct DisplaySetup {
    /// Where the settings are written; the Client swaps in its configuration file's once start-up
    /// has read it ([`DisplayPresenter::configure`]).
    pub device: Rc<dyn DeviceConfiguration>,
    pub keymap: Keymap<PageKey>,
    /// The chord of the Shell's `back`, for the legend.
    pub back_chord: String,
}

/// The Start Sidebar minimised checkbox as drawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckboxView {
    pub label: String,
    pub checked: bool,
    pub focused: bool,
}

/// One PREVIEW row, formatted under the choices made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewRowView {
    pub glyph: String,
    /// The Flagged row, whose glyph takes the accent.
    pub flagged: bool,
    pub date: String,
    pub payee: String,
    pub amount: String,
}

/// The PREVIEW table beside the controls.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewView {
    pub heading: String,
    pub date_column: String,
    pub payee_column: String,
    pub amount_column: String,
    pub rows: Vec<PreviewRowView>,
    pub row_density: RowDensity,
    pub note: String,
}

/// One built-in Colour Theme's card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColourCardView {
    pub id: &'static str,
    pub name: String,
    pub chosen: bool,
    pub focused: bool,
}

/// The Colour Theme grid: its label, the note when its key is shadowed, and a card per built-in
/// Colour Theme.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColourThemeView {
    pub label: String,
    pub shadowed: Option<String>,
    pub cards: Vec<ColourCardView>,
}

/// Display as the Client draws it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplayViewModel {
    /// The control the keyboard is on, by its `DISPLAY_FIELD_*` index.
    pub field: Option<usize>,
    /// The Colour Theme card the keyboard is on, while the grid has it.
    pub grid: Option<usize>,
    pub date_style: ControlView<Option<DateStyle>>,
    pub row_density: SelectView<RowDensity>,
    pub status_glyphs: SelectView<StatusGlyphs>,
    pub start_sidebar_minimised: CheckboxView,
    pub preview: PreviewView,
    pub appearance: ControlView<ColourAppearance>,
    pub colour_theme: ColourThemeView,
    pub output: DisplayOutput,
    /// The keys the status line shows: the grid's while it has the keyboard, else the controls'.
    pub hints: Vec<Hint>,
    /// The page's keys for the `?` cheat sheet: the controls', then the grid's own.
    pub help: Vec<Hint>,
}

/// Settings › Display's Presenter.
pub struct DisplayPresenter {
    device: Rc<dyn DeviceConfiguration>,
    keymap: Keymap<PageKey>,
    back_chord: String,
    output: DisplayOutput,
    colour_theme: Option<String>,
    colour_appearance: ColourAppearance,
    field: Option<usize>,
    grid: Option<usize>,
    /// How many cards fit a grid row, as the Client lays them out.
    columns: usize,
    page_focused: bool,
}

impl DisplayPresenter {
    /// Display on the defaults, until the Client tells it what start-up read.
    pub fn new(setup: DisplaySetup) -> Self {
        Self {
            device: setup.device,
            keymap: setup.keymap,
            back_chord: setup.back_chord,
            output: DisplayOutput::default(),
            colour_theme: None,
            colour_appearance: ColourAppearance::default(),
            field: None,
            grid: None,
            columns: 1,
            page_focused: false,
        }
    }

    pub fn keymap(&self) -> &Keymap<PageKey> {
        &self.keymap
    }

    /// Replaces the keymap and the `back` chord, for a `[keybindings]` change.
    pub fn set_keymap(&mut self, keymap: Keymap<PageKey>, back_chord: String) {
        self.keymap = keymap;
        self.back_chord = back_chord;
    }

    /// Writes to `device` from now on, and shows `settings` as start-up read them.
    pub fn configure(&mut self, device: Rc<dyn DeviceConfiguration>, settings: DisplaySettings) {
        self.device = device;
        self.output.date_style = settings.date_style;
        self.colour_theme = settings.colour_theme;
        self.colour_appearance = settings.colour_appearance;
    }

    /// Start Sidebar minimised as the Client remembered it from its last run.
    pub fn set_start_sidebar_minimised(&mut self, minimised: bool) {
        self.output.start_sidebar_minimised = minimised;
    }

    /// How many Colour Theme cards fit one grid row, for `j`/`k`.
    pub fn set_grid_columns(&mut self, columns: usize) {
        self.columns = columns.max(1);
    }

    /// Whether the page, not Settings' index, has the keyboard. Arriving puts it on the first
    /// control, and leaving takes it off the controls and the grid; only a change moves it.
    /// Answers whether it changed, so the View redraws only then: the frame tells the page on
    /// every draw.
    pub fn set_page_focused(&mut self, focused: bool) -> bool {
        if focused == self.page_focused {
            return false;
        }
        self.page_focused = focused;
        self.field = focused.then_some(0);
        self.grid = None;
        true
    }

    pub fn output(&self) -> DisplayOutput {
        self.output
    }

    /// Whether the Colour Theme grid has the keyboard.
    pub fn in_grid(&self) -> bool {
        self.grid.is_some()
    }

    /// Whether Display takes `event` as things stand, so a key it declines goes on to the frame.
    /// The grid declines `h` in its first column, so the frame's `h` steps out to the index, and
    /// `space`, which leaves it.
    pub fn takes(&self, event: &DisplayEvent) -> bool {
        match event {
            DisplayEvent::Key(key) => match (self.grid, self.field) {
                (Some(index), _) => match key {
                    PageKey::Change(Step::Prev) => index % self.columns != 0,
                    PageKey::Toggle => false,
                    _ => true,
                },
                (None, Some(_)) => true,
                (None, None) => false,
            },
            DisplayEvent::Back | DisplayEvent::LeaveGrid => self.grid.is_some(),
            _ => true,
        }
    }

    pub fn handle(&mut self, event: DisplayEvent, router: &mut dyn DeviceRouter) {
        if !self.takes(&event) {
            return;
        }
        match event {
            DisplayEvent::Key(key) => {
                if let Some(index) = self.grid {
                    self.grid_key(index, key, router);
                } else {
                    self.field_key(key, router);
                }
            }
            DisplayEvent::Back => {
                self.grid = None;
                self.field = Some(DISPLAY_FIELD_COUNT - 1);
            }
            DisplayEvent::LeaveGrid => self.grid = None,
            DisplayEvent::ChooseDateStyle(style) => self.choose_date_style(style, router),
            DisplayEvent::ChooseRowDensity(density) => {
                self.change(|output| output.row_density = density, router);
            }
            DisplayEvent::ChooseStatusGlyphs(glyphs) => {
                self.change(|output| output.status_glyphs = glyphs, router);
            }
            DisplayEvent::ToggleStartSidebarMinimised => self.change(
                |output| output.start_sidebar_minimised = !output.start_sidebar_minimised,
                router,
            ),
            DisplayEvent::ClickColourTheme(id) => {
                self.grid = None;
                self.choose_colour(ColourChoice::Theme(id), router);
            }
            DisplayEvent::ChooseColour(colour) => self.choose_colour(colour, router),
        }
    }

    pub fn view_model(&self) -> DisplayViewModel {
        let device = self.device.as_ref();
        let output = self.output;
        let chosen = self.chosen_theme();
        let field_hints = vec![
            Hint::new(
                chords(&self.keymap, &[MOVE_DOWN, MOVE_UP]),
                msg::settings_hint_field(),
            ),
            Hint::new(
                chords(&self.keymap, &[PREVIOUS_CHOICE, NEXT_CHOICE]),
                msg::settings_hint_change(),
            ),
            Hint::new(chord(&self.keymap, SELECT), msg::settings_hint_toggle()),
            Hint::new(self.back_chord.clone(), msg::settings_hint_index()),
        ];
        let grid_hints = vec![
            Hint::new(
                chords(
                    &self.keymap,
                    &[PREVIOUS_CHOICE, MOVE_DOWN, MOVE_UP, NEXT_CHOICE],
                ),
                msg::settings_hint_colour(),
            ),
            Hint::new(chord(&self.keymap, SELECT), msg::settings_hint_choose()),
            Hint::new(self.back_chord.clone(), msg::settings_hint_field()),
        ];
        let help = field_hints
            .iter()
            .chain(grid_hints.iter().take(2))
            .cloned()
            .collect();
        DisplayViewModel {
            field: self.field,
            grid: self.grid,
            date_style: ControlView {
                select: SelectView {
                    label: msg::settings_display_date_style_label(),
                    options: DATE_STYLE_CHOICES
                        .into_iter()
                        .map(|value| Choice {
                            value,
                            label: date_style_label(value),
                        })
                        .collect(),
                    chosen: Some(output.date_style),
                    focused: self.field == Some(0),
                },
                hint: None,
                shadowed: shadowed_note(device, DeviceKey::DateStyle),
            },
            row_density: SelectView {
                label: msg::settings_display_row_density(),
                options: RowDensity::ALL
                    .into_iter()
                    .map(|value| Choice {
                        value,
                        label: row_density_label(value),
                    })
                    .collect(),
                chosen: Some(output.row_density),
                focused: self.field == Some(1),
            },
            status_glyphs: SelectView {
                label: msg::settings_display_status_glyphs(),
                options: StatusGlyphs::ALL
                    .into_iter()
                    .map(|value| Choice {
                        value,
                        label: status_glyphs_label(value),
                    })
                    .collect(),
                chosen: Some(output.status_glyphs),
                focused: self.field == Some(2),
            },
            start_sidebar_minimised: CheckboxView {
                label: msg::settings_display_start_minimised(),
                checked: output.start_sidebar_minimised,
                focused: self.field == Some(DISPLAY_FIELD_SIDEBAR),
            },
            preview: PreviewView {
                heading: msg::settings_display_preview(),
                date_column: lib_locale::msg::column_date(),
                payee_column: lib_locale::msg::column_payee(),
                amount_column: lib_locale::msg::column_amount(),
                rows: PREVIEW_ROWS
                    .iter()
                    .map(|row| PreviewRowView {
                        glyph: row.status.glyph(output.status_glyphs).to_string(),
                        flagged: row.status == PreviewStatus::Flagged,
                        date: format_preview_date(row.year, row.month, row.day, output.date_style),
                        payee: row.payee.to_string(),
                        amount: format_preview_amount(row.amount_cents),
                    })
                    .collect(),
                row_density: output.row_density,
                note: msg::settings_display_note(),
            },
            appearance: ControlView {
                select: SelectView {
                    label: msg::settings_display_colour_appearance(),
                    options: APPEARANCES
                        .into_iter()
                        .map(|value| Choice {
                            value,
                            label: appearance_label(value),
                        })
                        .collect(),
                    chosen: Some(self.colour_appearance),
                    focused: false,
                },
                hint: None,
                shadowed: shadowed_note(device, DeviceKey::ColourAppearance),
            },
            colour_theme: ColourThemeView {
                label: msg::settings_display_colour_theme(),
                shadowed: shadowed_note(device, DeviceKey::ColourTheme),
                cards: ColourTheme::built_in()
                    .iter()
                    .enumerate()
                    .map(|(index, theme)| ColourCardView {
                        id: theme.id,
                        name: colour_theme_name(theme.id),
                        chosen: index == chosen,
                        focused: self.grid == Some(index),
                    })
                    .collect(),
            },
            output,
            hints: if self.grid.is_some() {
                grid_hints
            } else {
                field_hints
            },
            help,
        }
    }
}

impl DisplayPresenter {
    /// The index of the Colour Theme drawn now, where the grid's focus starts: the configured one,
    /// or the default for none or an id this release doesn't know, as the colours resolve it.
    fn chosen_theme(&self) -> usize {
        let theme = self
            .colour_theme
            .as_deref()
            .and_then(ColourTheme::by_id)
            .unwrap_or_else(|| ColourTheme::default_theme());
        ColourTheme::built_in()
            .iter()
            .position(|built_in| built_in.id == theme.id)
            .unwrap_or(0)
    }

    /// The controls' keys: `j`/`k` walk the controls and on past the last into the Colour Theme
    /// grid, `h`/`l` change a segmented or radio control (or clear or tick the checkbox) in place,
    /// and `enter`/`space` toggle the checkbox.
    fn field_key(&mut self, key: PageKey, router: &mut dyn DeviceRouter) {
        let Some(field) = self.field else {
            return;
        };
        match key {
            PageKey::Move(Step::Prev) => self.field = Some(field.saturating_sub(1)),
            PageKey::Move(_) => {
                if field + 1 >= DISPLAY_FIELD_COUNT {
                    self.field = None;
                    self.grid = Some(self.chosen_theme());
                } else {
                    self.field = Some(field + 1);
                }
            }
            PageKey::Change(step) => match field {
                0 => {
                    let current = self.output.date_style;
                    if let Some(style) = step_choice(&DATE_STYLE_CHOICES, &current, &current, step)
                    {
                        self.choose_date_style(style, router);
                    }
                }
                1 => {
                    let current = self.output.row_density;
                    let density =
                        step_choice(&RowDensity::ALL, &current, &current, step).unwrap_or(current);
                    self.change(|output| output.row_density = density, router);
                }
                2 => {
                    let current = self.output.status_glyphs;
                    let glyphs = step_choice(&StatusGlyphs::ALL, &current, &current, step)
                        .unwrap_or(current);
                    self.change(|output| output.status_glyphs = glyphs, router);
                }
                _ => self.change(
                    |output| output.start_sidebar_minimised = step == Step::Next,
                    router,
                ),
            },
            PageKey::Select | PageKey::Toggle => {
                if field == DISPLAY_FIELD_SIDEBAR {
                    self.change(
                        |output| output.start_sidebar_minimised = !output.start_sidebar_minimised,
                        router,
                    );
                }
            }
        }
    }

    /// The Colour Theme grid (`docs/colour-themes-design.md` "Settings"): the moves step the focus,
    /// stopping at the edges, `k` on the top row climbs back to the last control, and `enter`
    /// chooses the card. Moving the focus never previews.
    fn grid_key(&mut self, index: usize, key: PageKey, router: &mut dyn DeviceRouter) {
        let last = ColourTheme::built_in().len().saturating_sub(1);
        match key {
            PageKey::Select => {
                if let Some(theme) = ColourTheme::built_in().get(index) {
                    self.choose_colour(ColourChoice::Theme(theme.id), router);
                }
            }
            PageKey::Move(Step::Prev) if index < self.columns => {
                self.grid = None;
                self.field = Some(DISPLAY_FIELD_COUNT - 1);
            }
            PageKey::Move(Step::Prev) => self.grid = Some(index - self.columns),
            PageKey::Move(_) => {
                if index + self.columns <= last {
                    self.grid = Some(index + self.columns);
                }
            }
            PageKey::Change(Step::Prev) => self.grid = Some(index.saturating_sub(1)),
            PageKey::Change(_) => self.grid = Some((index + 1).min(last)),
            PageKey::Toggle => {}
        }
    }

    /// Saves the date style, which shows, and reaches the Views, only once saved.
    fn choose_date_style(&mut self, style: Option<DateStyle>, router: &mut dyn DeviceRouter) {
        if style == self.output.date_style {
            return;
        }
        let answer = self.device.set_date_style(style);
        if let Some(saved) = settle(answer, DeviceKey::DateStyle, router) {
            self.change(|output| output.date_style = saved, router);
        }
    }

    /// Saves a Colour Theme or Appearance, which the Shell applies only once saved.
    fn choose_colour(&mut self, colour: ColourChoice, router: &mut dyn DeviceRouter) {
        let saved = match colour {
            ColourChoice::Theme(id) => {
                let answer = self.device.set_colour_theme(id);
                settle(answer, DeviceKey::ColourTheme, router).map(|id| {
                    self.colour_theme = Some(id.to_string());
                    ColourChoice::Theme(id)
                })
            }
            ColourChoice::Appearance(appearance) => {
                let answer = self.device.set_colour_appearance(appearance);
                settle(answer, DeviceKey::ColourAppearance, router).map(|appearance| {
                    self.colour_appearance = appearance;
                    ColourChoice::Appearance(appearance)
                })
            }
        };
        if let Some(colour) = saved {
            router.colour_changed(colour);
        }
    }

    /// Applies a change to the Display output and hands the Shell the result.
    fn change(&mut self, change: impl FnOnce(&mut DisplayOutput), router: &mut dyn DeviceRouter) {
        change(&mut self.output);
        router.display_changed(self.output);
    }
}

/// The Date format control's label for a choice.
pub fn date_style_label(choice: Option<DateStyle>) -> String {
    match choice {
        None => msg::settings_display_date_style_default(),
        Some(DateStyle::Short) => msg::settings_display_date_style_short(),
        Some(DateStyle::Medium) => msg::settings_display_date_style_medium(),
        Some(DateStyle::Long) => msg::settings_display_date_style_long(),
        Some(DateStyle::Iso) => msg::settings_display_date_style_iso(),
    }
}

fn row_density_label(density: RowDensity) -> String {
    match density {
        RowDensity::Compact => msg::settings_display_density_compact(),
        RowDensity::Regular => msg::settings_display_density_regular(),
        RowDensity::Roomy => msg::settings_display_density_roomy(),
    }
}

fn status_glyphs_label(glyphs: StatusGlyphs) -> String {
    match glyphs {
        StatusGlyphs::Unicode => {
            msg::settings_display_glyphs_unicode("\u{25cb} \u{25d0} \u{25cf} \u{2691}")
        }
        StatusGlyphs::AsciiFallback => msg::settings_display_glyphs_ascii("o / x !"),
    }
}

/// The Colour Theme's name, a shared Message.
pub fn colour_theme_name(id: &str) -> String {
    match id {
        "high_contrast" => lib_locale::msg::colour_theme_high_contrast(),
        "catppuccin" => lib_locale::msg::colour_theme_catppuccin(),
        "gruvbox" => lib_locale::msg::colour_theme_gruvbox(),
        "nord" => lib_locale::msg::colour_theme_nord(),
        _ => lib_locale::msg::colour_theme_modernist(),
    }
}

/// The Appearance's label, a shared Message.
pub fn appearance_label(appearance: ColourAppearance) -> String {
    match appearance {
        ColourAppearance::Light => lib_locale::msg::colour_appearance_light(),
        ColourAppearance::Dark => lib_locale::msg::colour_appearance_dark(),
        ColourAppearance::System => lib_locale::msg::colour_appearance_system(),
    }
}

/// One PREVIEW row's transaction status (the mockup's three drawn rows: cleared, pending,
/// flagged), distinct from [`StatusGlyphs`], which picks how a status is marked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PreviewStatus {
    Cleared,
    Pending,
    Flagged,
}

impl PreviewStatus {
    /// The glyph in the PREVIEW's leftmost column. The ascii fallback's symbols keep the mockup's
    /// left-to-right order ("o / x !"): pending the open `o`, cleared the `x`, flagged `!`.
    fn glyph(self, style: StatusGlyphs) -> &'static str {
        match (self, style) {
            (Self::Cleared, StatusGlyphs::Unicode) => "\u{25cf}",
            (Self::Pending, StatusGlyphs::Unicode) => "\u{25d0}",
            (Self::Flagged, StatusGlyphs::Unicode) => "\u{2691}",
            (Self::Cleared, StatusGlyphs::AsciiFallback) => "x",
            (Self::Pending, StatusGlyphs::AsciiFallback) => "o",
            (Self::Flagged, StatusGlyphs::AsciiFallback) => "!",
        }
    }
}

/// One PREVIEW row as seeded, by a real date and integer cents so it re-renders under any date
/// style.
struct PreviewRow {
    status: PreviewStatus,
    year: i32,
    month: u32,
    day: u32,
    payee: &'static str,
    amount_cents: i64,
}

/// The mockup's own three seeded rows, in its own order.
const PREVIEW_ROWS: &[PreviewRow] = &[
    PreviewRow {
        status: PreviewStatus::Cleared,
        year: 2026,
        month: 9,
        day: 12,
        payee: "Woolworths Metro",
        amount_cents: -8640,
    },
    PreviewRow {
        status: PreviewStatus::Pending,
        year: 2026,
        month: 9,
        day: 10,
        payee: "Telstra",
        amount_cents: -9900,
    },
    PreviewRow {
        status: PreviewStatus::Flagged,
        year: 2026,
        month: 9,
        day: 9,
        payee: "Dept of Education",
        amount_cents: 421_000,
    },
];

/// A day in the chosen date style, through the Locale formatting every View uses, so the PREVIEW
/// can't disagree with them.
pub fn format_preview_date(year: i32, month: u32, day: u32, style: Option<DateStyle>) -> String {
    NaiveDate::from_ymd_opt(year, month, day)
        .map(|date| format_date(date, style))
        .unwrap_or_else(|| "???".to_string())
}

/// `amount_cents` as the PREVIEW shows it (`+4,210.00`, `−86.40`): an explicit `+` on a positive
/// amount and the Locale's grouping, as the Transactions AMOUNT column signs it.
pub fn format_preview_amount(amount_cents: i64) -> String {
    let sign = if amount_cents < 0 { "-" } else { "" };
    let text = format!(
        "{sign}{}.{:02}",
        amount_cents.unsigned_abs() / 100,
        amount_cents.unsigned_abs() % 100
    );
    text.parse::<Money>()
        .map(|money| format_amount(&money, AmountStyle::own().with_plus()).text)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use lib_viper::keymap::KeyBindingConfig;

    use super::*;
    use crate::fake::{InMemoryDeviceConfiguration, RecordingRouter, Routed};
    use crate::keymap::device_page_keymap;

    fn display_on(device: Rc<InMemoryDeviceConfiguration>) -> DisplayPresenter {
        lib_locale::init_with_layers("en-US", &[msg::LAYER]);
        let mut display = DisplayPresenter::new(DisplaySetup {
            device: device.clone(),
            keymap: device_page_keymap(&KeyBindingConfig::default()),
            back_chord: "esc".to_string(),
        });
        display.configure(device, DisplaySettings::default());
        display.set_grid_columns(3);
        display
    }

    fn writable() -> (Rc<InMemoryDeviceConfiguration>, DisplayPresenter) {
        let device = Rc::new(InMemoryDeviceConfiguration::writable());
        (device.clone(), display_on(device))
    }

    fn send(
        display: &mut DisplayPresenter,
        events: impl IntoIterator<Item = DisplayEvent>,
    ) -> RecordingRouter {
        let mut router = RecordingRouter::default();
        for event in events {
            display.handle(event, &mut router);
        }
        router
    }

    fn key(key: PageKey) -> DisplayEvent {
        DisplayEvent::Key(key)
    }

    const DOWN: DisplayEvent = DisplayEvent::Key(PageKey::Move(Step::Next));
    const UP: DisplayEvent = DisplayEvent::Key(PageKey::Move(Step::Prev));
    const LEFT: DisplayEvent = DisplayEvent::Key(PageKey::Change(Step::Prev));
    const RIGHT: DisplayEvent = DisplayEvent::Key(PageKey::Change(Step::Next));

    #[test]
    fn the_date_style_is_saved_then_reaches_the_views() {
        let (device, mut display) = writable();
        display.set_page_focused(true);

        let router = send(&mut display, [RIGHT]);

        assert_eq!(display.output().date_style, Some(DateStyle::Short));
        assert_eq!(device.stored().date_style, Some(Some(DateStyle::Short)));
        assert_eq!(
            router.statuses(),
            ["date_style saved to personal-ledger.conf"]
        );
        assert_eq!(router.displays(), [display.output()]);
    }

    #[test]
    fn a_shadowed_date_style_is_noted_and_reaches_no_view() {
        let device = Rc::new(InMemoryDeviceConfiguration::shadowing(
            DeviceKey::DateStyle,
            "PERSONAL_LEDGER_PERSONAL_LEDGER__DATE_STYLE",
        ));
        let mut display = display_on(device.clone());
        assert!(display.view_model().date_style.shadowed.is_some());

        let router = send(
            &mut display,
            [DisplayEvent::ChooseDateStyle(Some(DateStyle::Iso))],
        );

        assert_eq!(display.output().date_style, None);
        assert!(router.displays().is_empty());
        assert!(router.statuses()[0].contains("PERSONAL_LEDGER_PERSONAL_LEDGER__DATE_STYLE"));
    }

    #[test]
    fn a_refused_colour_is_not_applied() {
        let mut display = display_on(Rc::new(InMemoryDeviceConfiguration::default()));

        let router = send(
            &mut display,
            [
                DisplayEvent::ClickColourTheme("nord"),
                DisplayEvent::ChooseColour(ColourChoice::Appearance(ColourAppearance::Dark)),
            ],
        );

        assert!(
            !router
                .routed
                .iter()
                .any(|routed| matches!(routed, Routed::ColourChanged(_)))
        );
        assert_eq!(router.statuses().len(), 2);
        assert!(router.statuses()[0].contains("colour_theme can't be saved"));
    }

    #[test]
    fn a_saved_colour_is_applied_and_chosen() {
        let (device, mut display) = writable();

        let router = send(
            &mut display,
            [
                DisplayEvent::ClickColourTheme("nord"),
                DisplayEvent::ChooseColour(ColourChoice::Appearance(ColourAppearance::Light)),
            ],
        );

        assert!(
            router
                .routed
                .contains(&Routed::ColourChanged(ColourChoice::Theme("nord")))
        );
        assert!(
            router
                .routed
                .contains(&Routed::ColourChanged(ColourChoice::Appearance(
                    ColourAppearance::Light
                )))
        );
        let view = display.view_model();
        assert!(
            view.colour_theme
                .cards
                .iter()
                .any(|card| card.chosen && card.id == "nord")
        );
        assert_eq!(view.appearance.select.chosen, Some(ColourAppearance::Light));
        assert_eq!(
            device.stored().colour_appearance,
            Some(ColourAppearance::Light)
        );
    }

    #[test]
    fn row_density_glyphs_and_the_sidebar_are_the_display_output_unwritten() {
        let (device, mut display) = writable();

        let router = send(
            &mut display,
            [
                DisplayEvent::ChooseRowDensity(RowDensity::Roomy),
                DisplayEvent::ChooseStatusGlyphs(StatusGlyphs::AsciiFallback),
                DisplayEvent::ToggleStartSidebarMinimised,
            ],
        );

        assert_eq!(
            router.displays().last(),
            Some(&DisplayOutput {
                date_style: None,
                row_density: RowDensity::Roomy,
                status_glyphs: StatusGlyphs::AsciiFallback,
                start_sidebar_minimised: true,
            })
        );
        assert!(router.statuses().is_empty());
        assert_eq!(device.stored(), Default::default());
        let preview = display.view_model().preview;
        assert_eq!(preview.rows[2].glyph, "!");
        assert!(preview.rows[2].flagged);
    }

    #[test]
    fn the_keys_step_each_control_and_toggle_the_checkbox() {
        let (_, mut display) = writable();
        display.set_page_focused(true);

        send(&mut display, [DOWN, RIGHT, RIGHT, DOWN, RIGHT, DOWN]);
        assert_eq!(display.output().row_density, RowDensity::Roomy);
        assert_eq!(display.output().status_glyphs, StatusGlyphs::AsciiFallback);
        assert_eq!(display.view_model().field, Some(DISPLAY_FIELD_SIDEBAR));
        send(&mut display, [key(PageKey::Toggle)]);
        assert!(display.output().start_sidebar_minimised);
        send(&mut display, [LEFT]);
        assert!(!display.output().start_sidebar_minimised);
        send(&mut display, [key(PageKey::Select)]);
        assert!(display.output().start_sidebar_minimised);
    }

    #[test]
    fn j_past_the_last_control_enters_the_grid_on_the_chosen_card_and_k_climbs_back() {
        let (device, mut display) = writable();
        display.configure(
            device,
            DisplaySettings {
                colour_theme: Some("nord".to_string()),
                ..DisplaySettings::default()
            },
        );
        display.set_page_focused(true);
        let nord = ColourTheme::built_in()
            .iter()
            .position(|theme| theme.id == "nord");

        send(&mut display, [DOWN; 4]);

        let view = display.view_model();
        assert_eq!((view.field, view.grid), (None, nord));
        assert_eq!(view.hints[0].chord, "h/j/k/l");
        let up = if nord.unwrap_or(0) < 3 { 1 } else { 2 };
        send(&mut display, std::iter::repeat_n(UP, up));
        assert_eq!(display.view_model().field, Some(DISPLAY_FIELD_SIDEBAR));
    }

    #[test]
    fn the_grid_declines_h_in_its_first_column_and_space_and_esc_climbs_out() {
        let (_, mut display) = writable();
        display.set_page_focused(true);
        send(&mut display, [DOWN; 4]);
        assert_eq!(display.view_model().grid, Some(0));

        assert!(
            !display.takes(&LEFT),
            "the frame's h steps out to the index"
        );
        assert!(!display.takes(&key(PageKey::Toggle)));
        send(&mut display, [RIGHT]);
        assert_eq!(display.view_model().grid, Some(1));
        let router = send(&mut display, [key(PageKey::Select)]);
        assert!(
            router
                .routed
                .contains(&Routed::ColourChanged(ColourChoice::Theme(
                    ColourTheme::built_in()[1].id
                )))
        );

        send(&mut display, [DisplayEvent::Back]);
        let view = display.view_model();
        assert_eq!((view.field, view.grid), (Some(DISPLAY_FIELD_SIDEBAR), None));
        assert!(!display.takes(&DisplayEvent::LeaveGrid));
    }

    #[test]
    fn the_legend_follows_the_focus_and_the_cheat_sheet_holds_both() {
        let (_, mut display) = writable();
        display.set_page_focused(true);
        let chords = |hints: Vec<Hint>| -> Vec<String> {
            hints.into_iter().map(|hint| hint.chord).collect()
        };

        let view = display.view_model();
        assert_eq!(chords(view.hints), ["j/k", "h/l", "enter", "esc"]);
        assert_eq!(
            chords(view.help),
            ["j/k", "h/l", "enter", "esc", "h/j/k/l", "enter"]
        );
        send(&mut display, [DOWN; 4]);
        assert_eq!(
            chords(display.view_model().hints),
            ["h/j/k/l", "enter", "esc"]
        );
    }

    #[test]
    fn the_preview_formats_under_the_date_style() {
        lib_locale::with_locale(lib_locale::Locale::EnAu, || {
            assert_eq!(format_preview_date(2026, 9, 12, None), "12 Sept 2026");
            assert_eq!(
                format_preview_date(2026, 9, 12, Some(DateStyle::Iso)),
                "2026-09-12"
            );
        });
        assert_eq!(format_preview_date(2026, 13, 40, None), "???");
        assert_eq!(format_preview_amount(-8640), "\u{2212}86.40");
        assert_eq!(format_preview_amount(421_000), "+4,210.00");
    }

    #[test]
    fn the_labels_are_messages() {
        let (_, display) = writable();
        let view = display.view_model();
        let labels: Vec<String> = view
            .date_style
            .select
            .options
            .into_iter()
            .map(|choice| choice.label)
            .collect();
        assert_eq!(labels, ["Locale", "Short", "Medium", "Long", "ISO"]);
        assert_eq!(view.row_density.options[2].label, "roomy");
        assert_eq!(
            view.status_glyphs.options[1].label,
            "ascii fallback \u{2014} o / x !"
        );
    }
}
