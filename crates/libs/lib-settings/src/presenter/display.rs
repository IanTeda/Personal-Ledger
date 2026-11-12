//! Settings › Display's Presenter: the date style, Row density, Status glyphs, Colour Theme and
//! Appearance, Configuration in the user's configuration file (ADR-0053) written through
//! [`DeviceConfiguration`]; and Start Sidebar minimised, which the Client remembers in its own
//! state file. Beside them a PREVIEW re-renders three seeded rows under the choices made.
//!
//! The fields are the device settings form's ([`DeviceForm`]), which walks them, steps them and
//! writes each change, showing it only once saved. Display declares them ([`DisplayField`]) and
//! keeps what is its own: the Colour Theme grid after the form, the PREVIEW, the Display output and
//! the colour hand-off.
//!
//! The date style, Row density, Status glyphs and Start Sidebar minimised are the Display output,
//! which the Presenter hands the Shell on each change ([`DeviceRouter::display_changed`]) for the
//! Views that draw with it. A saved Colour Theme or Appearance goes to the Shell to apply
//! ([`DeviceRouter::colour_changed`]); how each theme's card looks, and what the Client's own
//! `[theme]` overrides and the system's light or dark setting make of it, is the Client's to draw.

use std::convert::Infallible;
use std::rc::Rc;

use chrono::NaiveDate;
use lib_colour_theme::{ColourAppearance, ColourTheme};
use lib_core::configuration::{ConfigKey, ConfigValue};
use lib_core::{DateStyle, Money};
use lib_list_cursor::Step;
use lib_locale::format::{AmountStyle, format_amount, format_date};
use lib_viper::display::{PageDisplay, RowDensity, StatusGlyphs};
use lib_viper::keymap::{KeyBindingConfig, Keymap, MOVE_DOWN, MOVE_UP, SELECT};
use lib_viper::presenter::{PagePresenter, Presenter};
use lib_viper::view::{Choice, Hint};

use super::device::shadowed_note;
use super::form::{
    DeviceForm, FieldDecl, FieldValue, FieldViews, FormAnswer, FormInput, Presentation,
};
use crate::keymap::{NEXT_CHOICE, PREVIOUS_CHOICE};
use crate::msg;
use crate::protocols::{ColourChoice, DeviceConfiguration, DeviceRouter, DeviceSnapshot};
use crate::summary::SummaryRow;

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

/// Display's fields, top to bottom. The `j`/`k` walk is the first four: Appearance is reached by a
/// click, and the Colour Theme grid is the stop after Start Sidebar minimised.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayField {
    DateStyle,
    RowDensity,
    StatusGlyphs,
    StartSidebar,
    Appearance,
}

impl DisplayField {
    pub const ALL: [DisplayField; 5] = [
        Self::DateStyle,
        Self::RowDensity,
        Self::StatusGlyphs,
        Self::StartSidebar,
        Self::Appearance,
    ];
}

/// What the Views and the Shell draw with, from Display: the Shell fans it out on each change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DisplayOutput {
    pub date_style: Option<DateStyle>,
    pub row_density: RowDensity,
    pub status_glyphs: StatusGlyphs,
    /// Applied to the primary rail at launch, so a change waits for the next one.
    pub start_sidebar_minimised: bool,
}

impl DisplayOutput {
    /// What every page is handed of it; Start Sidebar minimised is the Shell's alone.
    pub fn page(&self) -> PageDisplay {
        PageDisplay {
            date_style: self.date_style,
            glyphs: self.status_glyphs,
            density: self.row_density,
        }
    }
}

/// What Display is told and the person asks of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayEvent {
    /// `j`/`k`: the field below or above, or the grid's row.
    Move(Step),
    /// `h`/`l`: the focused field's choice after or before, or the grid's card.
    Change(Step),
    /// `enter`: ticks the checkbox, or chooses the Colour Theme the grid's focus is on.
    Select,
    /// `space`: ticks the checkbox.
    Toggle,
    /// `back` (`esc`) in the Colour Theme grid: up to the field above it.
    Back,
    /// Focus `tab` or a half-page move in the Colour Theme grid leaves it, and the key goes on to
    /// the Shell, which does what it does there.
    LeaveGrid,
    /// How many Colour Theme cards fit one grid row as the Client lays them out, for `j`/`k`. The
    /// layout, told outside any input; a count the Presenter already has is not taken.
    SetGridColumns(usize),
    /// A click on a field's choice, by its index.
    Choose { field: DisplayField, index: usize },
    /// A click on a field itself: the Start Sidebar minimised checkbox toggles.
    Press(DisplayField),
    /// A click on a Colour Theme card, which also takes the keyboard off the grid.
    ClickColourTheme(&'static str),
    /// The palette's `theme` or `appearance`.
    ChooseColour(ColourChoice),
}

impl DisplayEvent {
    /// Whether the event is one of the keymap's keys, which go to the focused field or the grid.
    fn is_key(&self) -> bool {
        matches!(
            self,
            Self::Move(_) | Self::Change(_) | Self::Select | Self::Toggle
        )
    }

    /// The event as the form takes it, `None` for one that is Display's own.
    fn form_input(&self) -> Option<FormInput<DisplayField>> {
        Some(match *self {
            Self::Move(step) => FormInput::Move(step),
            Self::Change(step) => FormInput::Change(step),
            Self::Select => FormInput::Select,
            Self::Toggle => FormInput::Toggle,
            Self::Choose { field, index } => FormInput::Choose { field, index },
            Self::Press(field) => FormInput::Press(field),
            Self::Back
            | Self::LeaveGrid
            | Self::SetGridColumns(_)
            | Self::ClickColourTheme(_)
            | Self::ChooseColour(_) => return None,
        })
    }
}

/// What Display shows of a device's Configuration.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DisplaySettings {
    pub date_style: Option<DateStyle>,
    pub row_density: RowDensity,
    pub status_glyphs: StatusGlyphs,
    /// The `colour_theme` key raw, so an id from a newer release survives; `None` is the default.
    pub colour_theme: Option<String>,
    pub colour_appearance: ColourAppearance,
}

impl DisplaySettings {
    /// What each field shows of `snapshot` ([`DeviceSnapshot::shown`]): the value in force, or
    /// the default.
    pub fn read(snapshot: &DeviceSnapshot) -> Self {
        let mut settings = Self::default();
        for key in [
            ConfigKey::DateStyle,
            ConfigKey::RowDensity,
            ConfigKey::StatusGlyphs,
            ConfigKey::ColourTheme,
            ConfigKey::ColourAppearance,
        ] {
            if let Some(value) = snapshot.shown(key) {
                settings.take(value);
            }
        }
        settings
    }

    /// Takes `value`, as start-up read it or a write saved it.
    fn take(&mut self, value: &ConfigValue) {
        match value {
            ConfigValue::DateStyle(style) => self.date_style = *style,
            ConfigValue::RowDensity(density) => self.row_density = *density,
            ConfigValue::StatusGlyphs(glyphs) => self.status_glyphs = *glyphs,
            ConfigValue::ColourTheme(id) => self.colour_theme = Some(id.clone()),
            ConfigValue::ColourAppearance(appearance) => self.colour_appearance = *appearance,
            _ => {}
        }
    }
}

/// What the Client hands Display as it builds it.
pub struct DisplaySetup {
    /// Where the settings are read from, once, and written: the configuration file start-up read.
    pub device: Rc<dyn DeviceConfiguration>,
    pub keymap: Keymap<DisplayEvent>,
    /// The chord of the Shell's `back`, for the legend.
    pub back_chord: String,
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

/// The PREVIEW table beside the fields.
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
    /// The Colour Theme card the keyboard is on, while the grid has it.
    pub grid: Option<usize>,
    /// Each field as the form draws it, in [`DisplayField::ALL`]'s order.
    pub fields: FieldViews<DisplayField>,
    pub preview: PreviewView,
    pub colour_theme: ColourThemeView,
    pub output: DisplayOutput,
    /// The keys the status line shows: the grid's while it has the keyboard, else the fields'.
    pub hints: Vec<Hint>,
    /// The page's keys for the `?` cheat sheet: the fields', then the grid's own.
    pub help: Vec<Hint>,
}

/// Settings › Display's Presenter.
pub struct DisplayPresenter {
    /// The Colour Theme's stock note while a source above the file sets it, as start-up read it.
    colour_theme_shadowed: Option<String>,
    keymap: Keymap<DisplayEvent>,
    back_chord: String,
    /// The fields and the values they show, as each write's answer keeps them.
    form: DeviceForm<DisplayField>,
    /// The `colour_theme` key raw, which no field shows: the grid's.
    colour_theme: Option<String>,
    grid: Option<usize>,
    /// How many cards fit a grid row, as the Client lays them out.
    columns: usize,
    page_focused: bool,
}

impl DisplayPresenter {
    /// Display on what `setup`'s device reads.
    pub fn new(setup: DisplaySetup) -> Self {
        let snapshot = setup.device.read();
        let fields = DisplayField::ALL.into_iter().map(declare).collect();
        Self {
            form: DeviceForm::new(setup.device, &snapshot, fields),
            colour_theme_shadowed: shadowed_note(
                snapshot.get(ConfigKey::ColourTheme).shadow.as_deref(),
            ),
            keymap: setup.keymap,
            back_chord: setup.back_chord,
            colour_theme: DisplaySettings::read(&snapshot).colour_theme,
            grid: None,
            columns: 1,
            page_focused: false,
        }
    }

    /// Start Sidebar minimised as the Client remembered it from its last run.
    pub fn set_start_sidebar_minimised(&mut self, minimised: bool) {
        self.form
            .set_value(DisplayField::StartSidebar, FieldValue::Flag(minimised));
    }

    /// Whether the page, not Settings' index, has the keyboard. Arriving puts it on the first
    /// field, and leaving takes it off the fields and the grid; only a change moves it. Answers
    /// whether it changed, so the View redraws only then: the frame tells the page on every draw.
    pub fn set_page_focused(&mut self, focused: bool) -> bool {
        if focused == self.page_focused {
            return false;
        }
        self.page_focused = focused;
        self.form.set_focused(focused);
        self.grid = None;
        true
    }

    /// The Display output the fields show.
    pub fn output(&self) -> DisplayOutput {
        let mut output = DisplayOutput::default();
        for field in DisplayField::ALL {
            match self.form.value(field) {
                Some(FieldValue::Set(ConfigValue::DateStyle(style))) => output.date_style = *style,
                Some(FieldValue::Set(ConfigValue::RowDensity(density))) => {
                    output.row_density = *density;
                }
                Some(FieldValue::Set(ConfigValue::StatusGlyphs(glyphs))) => {
                    output.status_glyphs = *glyphs;
                }
                Some(FieldValue::Flag(minimised)) => output.start_sidebar_minimised = *minimised,
                _ => {}
            }
        }
        output
    }

    /// The settings as they stand now, as start-up read them or as the page last wrote them.
    pub fn settings(&self) -> DisplaySettings {
        let mut settings = DisplaySettings {
            colour_theme: self.colour_theme.clone(),
            ..DisplaySettings::default()
        };
        for field in DisplayField::ALL {
            if let Some(FieldValue::Set(value)) = self.form.value(field) {
                settings.take(value);
            }
        }
        settings
    }

    /// What the Settings index's detail pane quotes of Display: each field as it stands.
    pub fn summary_rows(&self) -> Vec<SummaryRow> {
        let settings = self.settings();
        vec![
            SummaryRow::new(
                msg::settings_display_date_style_label(),
                date_style_label(settings.date_style),
            ),
            SummaryRow::new(
                msg::settings_display_row_density(),
                row_density_label(settings.row_density),
            ),
            SummaryRow::new(
                msg::settings_display_status_glyphs(),
                status_glyphs_label(settings.status_glyphs),
            ),
            SummaryRow::new(
                msg::settings_display_colour_theme(),
                colour_theme_name(settings.colour_theme.as_deref().unwrap_or_default()),
            ),
            SummaryRow::new(
                msg::settings_summary_appearance(),
                appearance_label(settings.colour_appearance),
            ),
        ]
    }
}

impl Presenter for DisplayPresenter {
    type Event = DisplayEvent;
    type ViewModel = DisplayViewModel;
    type Router<'r> = dyn DeviceRouter + 'r;

    fn handle(&mut self, event: DisplayEvent, router: &mut Self::Router<'_>) {
        if !self.takes(&event) {
            return;
        }
        if let Some(index) = self.grid
            && event.is_key()
        {
            self.grid_key(index, event, router);
            return;
        }
        let answer = match event {
            DisplayEvent::Back => {
                self.leave_grid_up();
                None
            }
            DisplayEvent::LeaveGrid => {
                self.grid = None;
                None
            }
            DisplayEvent::SetGridColumns(columns) => {
                self.columns = columns.max(1);
                None
            }
            DisplayEvent::ClickColourTheme(id) => {
                self.grid = None;
                self.choose_theme(id, router);
                None
            }
            DisplayEvent::ChooseColour(ColourChoice::Theme(id)) => {
                self.choose_theme(id, router);
                None
            }
            // A Command writes and reports even the value shown, as `:toasts` and a theme do.
            DisplayEvent::ChooseColour(ColourChoice::Appearance(appearance)) => self.form.write(
                DisplayField::Appearance,
                FieldValue::Set(ConfigValue::ColourAppearance(appearance)),
                router,
            ),
            event => event
                .form_input()
                .and_then(|input| self.form.handle(input, router)),
        };
        match answer {
            Some(FormAnswer::Saved(_, FieldValue::Set(value))) => self.saved(&value, router),
            Some(FormAnswer::Changed(DisplayField::StartSidebar, FieldValue::Flag(_))) => {
                router.display_changed(self.output());
            }
            // `j` past the last field enters the grid, on the Colour Theme drawn now.
            Some(FormAnswer::LeftEnd(Step::Next)) => {
                self.form.set_focused(false);
                self.grid = Some(self.chosen_theme());
            }
            // The walk stops at the top, and Display has no action.
            Some(
                FormAnswer::Saved(..)
                | FormAnswer::Changed(..)
                | FormAnswer::LeftEnd(_)
                | FormAnswer::Activated(_),
            )
            | None => {}
        }
    }

    fn view_model(&self) -> DisplayViewModel {
        let output = self.output();
        let chosen = self.chosen_theme();
        let grid_hints = vec![
            Hint::new(
                self.keymap
                    .hint_chords(&[PREVIOUS_CHOICE, MOVE_DOWN, MOVE_UP, NEXT_CHOICE]),
                msg::settings_hint_colour(),
            ),
            Hint::new(self.keymap.hint_chord(SELECT), msg::settings_hint_choose()),
            Hint::new(self.back_chord.clone(), msg::settings_hint_field()),
        ];
        let index = Hint::new(self.back_chord.clone(), msg::settings_hint_index());
        // The cheat sheet holds every field's keys, whichever field has the keyboard: the
        // checkbox's are the segmented fields' and its own `enter`.
        let help = self
            .form
            .field_hints(DisplayField::StartSidebar, &self.keymap)
            .into_iter()
            .chain([index.clone()])
            .chain(grid_hints.iter().take(2).cloned())
            .collect();
        DisplayViewModel {
            grid: self.grid,
            fields: self.form.views(DisplayField::ALL),
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
            colour_theme: ColourThemeView {
                label: msg::settings_display_colour_theme(),
                shadowed: self.colour_theme_shadowed.clone(),
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
                let mut hints = self.form.hints(&self.keymap, &self.back_chord);
                hints.push(index);
                hints
            },
            help,
        }
    }

    /// Whether Display takes `event` as things stand, so a key it declines goes on to the frame.
    /// The grid declines `h` in its first column, so the frame's `h` steps out to the index, and
    /// `space`, which it gives no meaning to; the fields' keys are as the form says. A grid column
    /// count it already has is declined, so the Client, which tells it on every draw, redraws only
    /// on a change.
    fn takes(&self, event: &DisplayEvent) -> bool {
        if let Some(index) = self.grid
            && event.is_key()
        {
            return match event {
                DisplayEvent::Change(Step::Prev) => index % self.columns != 0,
                DisplayEvent::Toggle => false,
                _ => true,
            };
        }
        match event {
            DisplayEvent::Back | DisplayEvent::LeaveGrid => self.grid.is_some(),
            DisplayEvent::SetGridColumns(columns) => (*columns).max(1) != self.columns,
            event => event
                .form_input()
                .is_none_or(|input| self.form.takes(&input)),
        }
    }
}

impl PagePresenter for DisplayPresenter {
    /// Display opens no Dialog.
    type Outcome = Infallible;

    /// Display opens no Dialog, so there is nothing to tell it.
    fn dialog_closed(_outcome: Option<Infallible>) -> Option<DisplayEvent> {
        None
    }

    fn keymap(&self) -> &Keymap<DisplayEvent> {
        &self.keymap
    }

    /// The keymap and the `back` chord its legend shows.
    fn rebind(&mut self, keys: &KeyBindingConfig) {
        self.keymap = crate::keymap::display_keymap(keys);
        self.back_chord = crate::keymap::back_chord(keys);
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

    /// The Colour Theme grid (`docs/colour-themes-design.md` "Settings"): the moves step the focus,
    /// stopping at the edges, `k` on the top row climbs back to the last field, and `enter`
    /// chooses the card. Moving the focus never previews.
    fn grid_key(&mut self, index: usize, key: DisplayEvent, router: &mut dyn DeviceRouter) {
        let last = ColourTheme::built_in().len().saturating_sub(1);
        match key {
            DisplayEvent::Select => {
                if let Some(theme) = ColourTheme::built_in().get(index) {
                    self.choose_theme(theme.id, router);
                }
            }
            DisplayEvent::Move(Step::Prev) if index < self.columns => self.leave_grid_up(),
            DisplayEvent::Move(Step::Prev) => self.grid = Some(index - self.columns),
            DisplayEvent::Move(_) => {
                if index + self.columns <= last {
                    self.grid = Some(index + self.columns);
                }
            }
            DisplayEvent::Change(Step::Prev) => self.grid = Some(index.saturating_sub(1)),
            DisplayEvent::Change(_) => self.grid = Some((index + 1).min(last)),
            _ => {}
        }
    }

    /// Hands the keyboard from the grid back to the form, on the field above the grid.
    fn leave_grid_up(&mut self) {
        self.grid = None;
        self.form.focus(DisplayField::StartSidebar);
    }

    /// Saves a Colour Theme, which the Shell applies only once saved. The built-in id chosen goes
    /// to the Shell, not the saved copy, since the Shell applies a built-in.
    fn choose_theme(&mut self, id: &'static str, router: &mut dyn DeviceRouter) {
        if let Some(ConfigValue::ColourTheme(saved)) = self
            .form
            .write_unfielded(ConfigValue::ColourTheme(id.to_string()), router)
        {
            self.colour_theme = Some(saved);
            router.colour_changed(ColourChoice::Theme(id));
        }
    }

    /// A field's value saved: the Display output reaches the Views, and the Appearance the Shell.
    fn saved(&self, value: &ConfigValue, router: &mut dyn DeviceRouter) {
        match *value {
            ConfigValue::DateStyle(_)
            | ConfigValue::RowDensity(_)
            | ConfigValue::StatusGlyphs(_) => router.display_changed(self.output()),
            ConfigValue::ColourAppearance(appearance) => {
                router.colour_changed(ColourChoice::Appearance(appearance));
            }
            _ => {}
        }
    }
}

/// `field` as the form takes it: its key, label, choices and how it is drawn.
fn declare(field: DisplayField) -> FieldDecl<DisplayField> {
    let set = |value| FieldValue::Set(value);
    let defaults = DisplaySettings::default();
    let segmented = |key, label, choices: Vec<Choice<FieldValue>>, default| {
        FieldDecl::configuration(field, key, label, Presentation::Segmented, choices)
            .when_unset(set(default))
    };
    match field {
        DisplayField::DateStyle => segmented(
            ConfigKey::DateStyle,
            msg::settings_display_date_style_label(),
            DATE_STYLE_CHOICES
                .into_iter()
                .map(|value| Choice {
                    value: set(ConfigValue::DateStyle(value)),
                    label: date_style_label(value),
                })
                .collect(),
            ConfigValue::DateStyle(defaults.date_style),
        ),
        DisplayField::RowDensity => segmented(
            ConfigKey::RowDensity,
            msg::settings_display_row_density(),
            RowDensity::ALL
                .into_iter()
                .map(|value| Choice {
                    value: set(ConfigValue::RowDensity(value)),
                    label: row_density_label(value),
                })
                .collect(),
            ConfigValue::RowDensity(defaults.row_density),
        ),
        DisplayField::StatusGlyphs => segmented(
            ConfigKey::StatusGlyphs,
            msg::settings_display_status_glyphs(),
            StatusGlyphs::ALL
                .into_iter()
                .map(|value| Choice {
                    value: set(ConfigValue::StatusGlyphs(value)),
                    label: status_glyphs_label(value),
                })
                .collect(),
            ConfigValue::StatusGlyphs(defaults.status_glyphs),
        ),
        // The Client remembers it in its state file and hands it back at start-up. A checkbox
        // draws its own label, not its choices'.
        DisplayField::StartSidebar => FieldDecl::remembered(
            field,
            msg::settings_display_start_minimised(),
            Presentation::Checkbox,
            [false, true]
                .into_iter()
                .map(|minimised| Choice {
                    value: FieldValue::Flag(minimised),
                    label: String::new(),
                })
                .collect(),
            FieldValue::Flag(false),
        ),
        DisplayField::Appearance => segmented(
            ConfigKey::ColourAppearance,
            msg::settings_display_colour_appearance(),
            APPEARANCES
                .into_iter()
                .map(|value| Choice {
                    value: set(ConfigValue::ColourAppearance(value)),
                    label: appearance_label(value),
                })
                .collect(),
            ConfigValue::ColourAppearance(defaults.colour_appearance),
        )
        .not_in_walk(),
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

pub(crate) fn row_density_label(density: RowDensity) -> String {
    match density {
        RowDensity::Compact => msg::settings_display_density_compact(),
        RowDensity::Regular => msg::settings_display_density_regular(),
        RowDensity::Roomy => msg::settings_display_density_roomy(),
    }
}

pub(crate) fn status_glyphs_label(glyphs: StatusGlyphs) -> String {
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
    use crate::keymap::display_keymap;

    fn display_on(device: Rc<InMemoryDeviceConfiguration>) -> DisplayPresenter {
        lib_locale::init_with_layers("en-US", &[msg::LAYER]);
        let mut display = DisplayPresenter::new(DisplaySetup {
            device: device.clone(),
            keymap: display_keymap(&KeyBindingConfig::default()),
            back_chord: "esc".to_string(),
        });
        send(&mut display, [DisplayEvent::SetGridColumns(3)]);
        display
    }

    /// Display on a writable device whose start-up read each of `values`.
    fn reading(values: impl IntoIterator<Item = ConfigValue>) -> DisplayPresenter {
        display_on(Rc::new(
            InMemoryDeviceConfiguration::writable().reading(values),
        ))
    }

    #[test]
    fn a_page_is_handed_the_display_output_but_the_sidebar() {
        let output = DisplayOutput {
            date_style: Some(DateStyle::Iso),
            row_density: RowDensity::Compact,
            status_glyphs: StatusGlyphs::AsciiFallback,
            start_sidebar_minimised: true,
        };

        assert_eq!(
            output.page(),
            PageDisplay {
                date_style: Some(DateStyle::Iso),
                glyphs: StatusGlyphs::AsciiFallback,
                density: RowDensity::Compact,
            }
        );
    }

    #[test]
    fn display_summarises_its_five_fields() {
        let rows = reading([]).summary_rows();

        let labels: Vec<_> = rows.iter().map(|row| row.label.as_str()).collect();
        assert_eq!(
            labels,
            [
                "Date format",
                "Row density",
                "Status glyphs",
                "Colour theme",
                "Appearance"
            ]
        );
        assert_eq!(rows[0].value, "Locale");
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

    fn chosen(display: &DisplayPresenter, field: DisplayField) -> Option<usize> {
        display
            .view_model()
            .fields
            .view(field)
            .and_then(|view| view.chosen)
    }

    const DOWN: DisplayEvent = DisplayEvent::Move(Step::Next);
    const UP: DisplayEvent = DisplayEvent::Move(Step::Prev);
    const LEFT: DisplayEvent = DisplayEvent::Change(Step::Prev);
    const RIGHT: DisplayEvent = DisplayEvent::Change(Step::Next);

    #[test]
    fn each_saved_field_reaches_the_views_as_the_display_output() {
        let (_, mut display) = writable();
        display.set_page_focused(true);

        let router = send(
            &mut display,
            [
                RIGHT,
                DisplayEvent::Choose {
                    field: DisplayField::RowDensity,
                    index: 2,
                },
                DisplayEvent::Choose {
                    field: DisplayField::StatusGlyphs,
                    index: 1,
                },
            ],
        );

        let output = DisplayOutput {
            date_style: Some(DateStyle::Short),
            row_density: RowDensity::Roomy,
            status_glyphs: StatusGlyphs::AsciiFallback,
            start_sidebar_minimised: false,
        };
        assert_eq!(display.output(), output);
        assert_eq!(router.displays().len(), 3);
        assert_eq!(router.displays().last(), Some(&output));
        let preview = display.view_model().preview;
        assert_eq!(preview.rows[2].glyph, "!");
        assert!(preview.rows[2].flagged);
        assert_eq!(preview.row_density, RowDensity::Roomy);
    }

    #[test]
    fn a_field_not_saved_reaches_no_view() {
        let device = Rc::new(InMemoryDeviceConfiguration::shadowing(
            ConfigKey::DateStyle,
            "PERSONAL_LEDGER_DISPLAY__DATE_STYLE",
        ));
        let mut display = display_on(device);

        let router = send(
            &mut display,
            [DisplayEvent::Choose {
                field: DisplayField::DateStyle,
                index: 4,
            }],
        );

        assert_eq!(display.output(), DisplayOutput::default());
        assert!(router.displays().is_empty());
    }

    #[test]
    fn the_sidebar_is_the_display_output_unwritten() {
        let (device, mut display) = writable();

        let router = send(
            &mut display,
            [DisplayEvent::Press(DisplayField::StartSidebar)],
        );

        assert!(display.output().start_sidebar_minimised);
        assert_eq!(router.displays(), [display.output()]);
        assert!(router.statuses().is_empty());
        assert_eq!(device.stored(), Default::default());
        assert_eq!(chosen(&display, DisplayField::StartSidebar), Some(1));
    }

    #[test]
    fn the_sidebar_shows_what_the_client_remembered() {
        let (_, mut display) = writable();

        display.set_start_sidebar_minimised(true);

        assert!(display.output().start_sidebar_minimised);
        assert_eq!(chosen(&display, DisplayField::StartSidebar), Some(1));
        let router = send(
            &mut display,
            [DisplayEvent::Press(DisplayField::StartSidebar)],
        );
        assert!(!display.output().start_sidebar_minimised);
        assert_eq!(router.displays(), [display.output()]);
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
    fn the_palettes_appearance_writes_and_applies_the_appearance_already_shown() {
        let (device, mut display) = writable();
        let shown = ColourAppearance::Light;
        send(
            &mut display,
            [DisplayEvent::ChooseColour(ColourChoice::Appearance(shown))],
        );

        let router = send(
            &mut display,
            [DisplayEvent::ChooseColour(ColourChoice::Appearance(shown))],
        );

        assert_eq!(router.statuses().len(), 1, "the write is reported");
        assert!(
            router
                .routed
                .contains(&Routed::ColourChanged(ColourChoice::Appearance(shown)))
        );
        assert_eq!(
            device.read().get(ConfigKey::ColourAppearance).stored,
            Some(ConfigValue::ColourAppearance(shown))
        );
    }

    #[test]
    fn a_saved_colour_is_applied_and_chosen() {
        let (device, mut display) = writable();

        let router = send(
            &mut display,
            [
                DisplayEvent::ClickColourTheme("nord"),
                DisplayEvent::ChooseColour(ColourChoice::Appearance(ColourAppearance::Light)),
                DisplayEvent::Choose {
                    field: DisplayField::Appearance,
                    index: 1,
                },
            ],
        );

        assert_eq!(
            router
                .routed
                .iter()
                .filter(|routed| matches!(routed, Routed::ColourChanged(_)))
                .collect::<Vec<_>>(),
            [
                &Routed::ColourChanged(ColourChoice::Theme("nord")),
                &Routed::ColourChanged(ColourChoice::Appearance(ColourAppearance::Light)),
                &Routed::ColourChanged(ColourChoice::Appearance(ColourAppearance::Dark)),
            ]
        );
        assert!(router.displays().is_empty(), "colours aren't the output");
        let view = display.view_model();
        assert!(
            view.colour_theme
                .cards
                .iter()
                .any(|card| card.chosen && card.id == "nord")
        );
        assert_eq!(chosen(&display, DisplayField::Appearance), Some(1));
        assert_eq!(
            device.stored().get(ConfigKey::ColourAppearance),
            Some(&ConfigValue::ColourAppearance(ColourAppearance::Dark))
        );
        assert_eq!(
            device.stored().get(ConfigKey::ColourTheme),
            Some(&ConfigValue::ColourTheme("nord".to_string()))
        );
    }

    #[test]
    fn display_starts_on_what_start_up_read() {
        let display = reading([
            ConfigValue::DateStyle(Some(DateStyle::Iso)),
            ConfigValue::RowDensity(RowDensity::Compact),
            ConfigValue::StatusGlyphs(StatusGlyphs::AsciiFallback),
            ConfigValue::ColourAppearance(ColourAppearance::Dark),
        ]);

        assert_eq!(
            display.output(),
            DisplayOutput {
                date_style: Some(DateStyle::Iso),
                row_density: RowDensity::Compact,
                status_glyphs: StatusGlyphs::AsciiFallback,
                start_sidebar_minimised: false,
            }
        );
        assert_eq!(chosen(&display, DisplayField::DateStyle), Some(4));
        assert_eq!(chosen(&display, DisplayField::RowDensity), Some(0));
        assert_eq!(chosen(&display, DisplayField::StatusGlyphs), Some(1));
        assert_eq!(chosen(&display, DisplayField::Appearance), Some(1));
        assert_eq!(
            display.view_model().preview.row_density,
            RowDensity::Compact
        );
    }

    #[test]
    fn settings_reads_back_what_start_up_read_and_what_the_page_wrote() {
        let mut display = reading([
            ConfigValue::RowDensity(RowDensity::Compact),
            ConfigValue::ColourTheme("not-a-theme-yet".to_string()),
            ConfigValue::ColourAppearance(ColourAppearance::Dark),
        ]);
        let read = DisplaySettings {
            row_density: RowDensity::Compact,
            colour_theme: Some("not-a-theme-yet".to_string()),
            colour_appearance: ColourAppearance::Dark,
            ..DisplaySettings::default()
        };
        assert_eq!(display.settings(), read, "as start-up read them");

        send(
            &mut display,
            [DisplayEvent::Choose {
                field: DisplayField::RowDensity,
                index: 1,
            }],
        );

        assert_eq!(display.settings().row_density, RowDensity::Regular);
        assert_eq!(display.settings().colour_theme, read.colour_theme);
    }

    #[test]
    fn j_walks_the_fields_past_appearance_into_the_grid_on_the_chosen_card_and_k_climbs_back() {
        let mut display = reading([ConfigValue::ColourTheme("nord".to_string())]);
        display.set_page_focused(true);
        let nord = ColourTheme::built_in()
            .iter()
            .position(|theme| theme.id == "nord");

        send(&mut display, [DOWN; 3]);
        assert_eq!(
            display.view_model().fields.focused(),
            Some(DisplayField::StartSidebar)
        );
        send(&mut display, [DOWN]);

        let view = display.view_model();
        assert_eq!((view.fields.focused(), view.grid), (None, nord));
        assert_eq!(view.hints[0].chord, "h/j/k/l");
        let up = if nord.unwrap_or(0) < 3 { 1 } else { 2 };
        send(&mut display, std::iter::repeat_n(UP, up));
        let view = display.view_model();
        assert_eq!(
            (view.fields.focused(), view.grid),
            (Some(DisplayField::StartSidebar), None)
        );
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
        assert!(!display.takes(&DisplayEvent::Toggle));
        send(&mut display, [RIGHT]);
        assert_eq!(display.view_model().grid, Some(1));
        let router = send(&mut display, [DisplayEvent::Select]);
        assert!(
            router
                .routed
                .contains(&Routed::ColourChanged(ColourChoice::Theme(
                    ColourTheme::built_in()[1].id
                )))
        );

        send(&mut display, [DisplayEvent::Back]);
        let view = display.view_model();
        assert_eq!(
            (view.fields.focused(), view.grid),
            (Some(DisplayField::StartSidebar), None)
        );
        assert!(!display.takes(&DisplayEvent::LeaveGrid));
    }

    #[test]
    fn tab_and_the_half_page_moves_leave_the_grid_only_while_it_has_the_keyboard() {
        let (_, mut display) = writable();
        display.set_page_focused(true);
        let keymap = display_keymap(&KeyBindingConfig::default());
        let leaving = ["tab", "ctrl+d", "ctrl+u"].map(|chord| keymap.event(chord));
        assert!(
            leaving
                .iter()
                .all(|event| *event == Some(DisplayEvent::LeaveGrid)),
            "{leaving:?}"
        );
        assert!(
            !display.takes(&DisplayEvent::LeaveGrid),
            "on a field the key is the Shell's alone"
        );

        send(&mut display, [DOWN; 4]);
        assert!(display.takes(&DisplayEvent::LeaveGrid));
        send(&mut display, [DisplayEvent::LeaveGrid]);
        let view = display.view_model();
        assert_eq!((view.fields.focused(), view.grid), (None, None));
    }

    #[test]
    fn a_grid_column_count_it_already_has_is_not_taken() {
        let (_, mut display) = writable();

        assert!(!display.takes(&DisplayEvent::SetGridColumns(3)));
        assert!(display.takes(&DisplayEvent::SetGridColumns(1)));
        send(&mut display, [DisplayEvent::SetGridColumns(1)]);
        assert!(
            !display.takes(&DisplayEvent::SetGridColumns(0)),
            "no room for a card still lays out one column"
        );
    }

    #[test]
    fn the_legend_follows_the_focus_and_the_cheat_sheet_holds_both() {
        let (_, mut display) = writable();
        display.set_page_focused(true);
        let chords = |hints: Vec<Hint>| -> Vec<String> {
            hints.into_iter().map(|hint| hint.chord).collect()
        };

        let view = display.view_model();
        assert_eq!(chords(view.hints), ["j/k", "h/l", "esc"]);
        assert_eq!(
            chords(view.help),
            ["j/k", "h/l", "enter", "esc", "h/j/k/l", "enter"]
        );
        send(&mut display, [DOWN; 3]);
        assert_eq!(
            chords(display.view_model().hints),
            ["j/k", "h/l", "enter", "esc"],
            "the checkbox's enter toggles it"
        );
        send(&mut display, [DOWN]);
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
        let options = |field| view.fields.view(field).map(|view| view.options.clone());
        assert_eq!(
            options(DisplayField::DateStyle),
            Some(
                ["Locale", "Short", "Medium", "Long", "ISO"]
                    .map(str::to_string)
                    .to_vec()
            )
        );
        assert_eq!(
            options(DisplayField::RowDensity).map(|options| options[2].clone()),
            Some("roomy".to_string())
        );
        assert_eq!(
            options(DisplayField::StatusGlyphs).map(|options| options[1].clone()),
            Some("ascii fallback \u{2014} o / x !".to_string())
        );
    }
}
