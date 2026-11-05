//! Settings › Display's View: `lib-settings`' Display Presenter in a gpui Entity, which the
//! Settings View hosts as any other page. It looks up each key the Shell passes on in the This
//! device pages' keymap, turns clicks into events through `PageClicks`, tells the Presenter how
//! many Colour Theme cards fit a row, and draws the view model (`render.rs`, `colour_theme.rs`).
//! The date style, Colour Theme and Appearance are written through the Presenter's
//! `DeviceConfiguration` (`super::device`); the Display output reaches the Shell as
//! `ViewEvent::DisplayChanged` and a saved colour is applied by the page's Router
//! (`super::router`). Each key and click is one input (`crate::input::send`).

pub mod colour_theme;
pub(crate) mod render;

use std::rc::Rc;

use gpui::{App, Context, EventEmitter, Keystroke};
use lib_settings::keymap::device_page_keymap;
use lib_settings::{
    ColourChoice, DeviceConfiguration, DisplayEvent, DisplayOutput, DisplayPresenter,
    DisplaySettings, DisplaySetup, DisplayViewModel,
};
use lib_shell::Hint;
use lib_viper::keymap::KeyBindingConfig;

use crate::{
    navigation::movement::Movement,
    settings::{
        SettingsPage,
        pages::{Page, PageFrame, PageRowClicked},
        router::DeviceRouter,
    },
    shell::view::{page_clicks::PresenterPage, statusline::PageStatus},
    view::event::ViewEvent,
};

impl EventEmitter<ViewEvent> for DisplayPage {}

/// The page has no rows, so it never emits a row click.
impl EventEmitter<PageRowClicked> for DisplayPage {}

/// The Display page: its Presenter, which holds the Display output, the Colour Theme and
/// Appearance and the keyboard focus on the controls and the Colour Theme grid.
pub struct DisplayPage {
    presenter: DisplayPresenter,
}

impl DisplayPage {
    /// Display writing to `device` under `config`'s keys, with the Shell's `back` on
    /// `back_chord`.
    pub fn new(
        device: Rc<dyn DeviceConfiguration>,
        config: &KeyBindingConfig,
        back_chord: String,
    ) -> Self {
        Self {
            presenter: DisplayPresenter::new(DisplaySetup {
                device,
                keymap: device_page_keymap(config),
                back_chord,
            }),
        }
    }

    /// The page's keys under `config`'s overrides.
    pub fn set_keymaps(&mut self, config: &KeyBindingConfig, back_chord: String) {
        self.presenter
            .set_keymap(device_page_keymap(config), back_chord);
    }

    /// Writes to `device` from now on, showing `settings` as start-up read them.
    pub fn configure(
        &mut self,
        device: Rc<dyn DeviceConfiguration>,
        settings: DisplaySettings,
        cx: &mut Context<'_, Self>,
    ) {
        self.presenter.configure(device, settings);
        cx.notify();
    }

    /// Start Sidebar minimised as the state file remembered it.
    pub fn set_start_sidebar_minimised(&mut self, minimised: bool, cx: &mut Context<'_, Self>) {
        self.presenter.set_start_sidebar_minimised(minimised);
        cx.notify();
    }

    /// `theme` or `appearance` from the palette, written as Display writes it, inside the
    /// command's input.
    pub fn choose_colour(&mut self, colour: ColourChoice, cx: &mut Context<'_, Self>) {
        self.send(DisplayEvent::ChooseColour(colour), cx);
    }

    pub fn output(&self) -> DisplayOutput {
        self.presenter.output()
    }

    pub fn view_model(&self) -> DisplayViewModel {
        self.presenter.view_model()
    }

    /// A key, click or command as one input, committed once it is handled; one sent inside
    /// another input (the palette's) runs in it.
    fn send(&mut self, event: DisplayEvent, cx: &mut Context<'_, Self>) {
        if crate::input::sending(cx) {
            self.handle(event, cx);
        } else {
            crate::input::send(cx, |cx| self.handle(event, cx));
        }
    }

    fn handle(&mut self, event: DisplayEvent, cx: &mut Context<'_, Self>) {
        self.presenter.handle(event, &mut DeviceRouter { cx });
        cx.notify();
    }

    /// Takes the keyboard off the Colour Theme grid, if it has it.
    fn leave_grid(&mut self, cx: &mut Context<'_, Self>) {
        if self.presenter.in_grid() {
            self.send(DisplayEvent::LeaveGrid, cx);
        }
    }
}

/// The page's clicks on its options, its checkbox and its Colour Theme cards.
impl PresenterPage<DisplayEvent> for DisplayPage {
    fn send(&mut self, event: DisplayEvent, cx: &mut Context<'_, Self>) {
        DisplayPage::send(self, event, cx);
    }
}

impl Page for DisplayPage {
    const PAGE: SettingsPage = SettingsPage::Display;

    /// The keymap's chords while a control or the grid has the keyboard, ahead of the frame's, so
    /// the page's `h`/`l` win. The grid declines `h` in its first column, which the frame's `h`
    /// then takes back to the index.
    fn module_key(&mut self, chord: &str, cx: &mut Context<'_, Self>) -> bool {
        let Some(key) = self.presenter.keymap().event(chord) else {
            return false;
        };
        let event = DisplayEvent::Key(key);
        if !self.presenter.takes(&event) {
            return false;
        }
        self.send(event, cx);
        true
    }

    /// Any other plain key the grid doesn't take leaves it, and goes on (`tab` to the next zone).
    fn key_ahead(&mut self, _keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        self.leave_grid(cx);
        false
    }

    /// A movement the grid did not take (`Ctrl-d`/`Ctrl-u`) leaves it, as any other key does; the
    /// page has no selection, so Settings scrolls.
    fn apply_movement(&mut self, _movement: Movement, cx: &mut Context<'_, Self>) -> bool {
        self.leave_grid(cx);
        false
    }

    /// `Esc` leaves the Colour Theme grid for the control above it. `false` when the grid has no
    /// focus, so the page is left to Settings.
    fn escape(&mut self, cx: &mut Context<'_, Self>) -> bool {
        if !self.presenter.takes(&DisplayEvent::Back) {
            return false;
        }
        cx.emit(ViewEvent::SetStatus(None));
        self.send(DisplayEvent::Back, cx);
        true
    }

    /// Focus arriving puts the keyboard on the first control; focus leaving, for the index or
    /// another page, takes it off the controls and the grid.
    fn set_page_focused(&mut self, focused: bool, cx: &mut Context<'_, Self>) {
        if self.presenter.set_page_focused(focused) {
            cx.notify();
        }
    }

    /// The page body's width sets how many Colour Theme cards fit a row.
    fn set_frame(&mut self, frame: PageFrame, _cx: &mut Context<'_, Self>) {
        self.presenter
            .set_grid_columns(colour_theme::grid_columns(frame.body_width));
    }

    /// The `?` cheat sheet's keys: the controls', then the grid's own.
    fn hints(&self, _cx: &App) -> Vec<Hint> {
        self.presenter.view_model().help
    }

    fn page_status(&self, _cx: &App) -> Option<PageStatus> {
        Some(PageStatus {
            hints: self.presenter.view_model().hints,
            right: crate::settings::scope_note(Self::PAGE),
        })
    }
}
