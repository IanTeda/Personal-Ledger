//! Settings › Display's View: `lib-settings`' Display Presenter is the [`PageView`] the generic
//! `ModulePage` hosts (ADR-0056), which the Settings View hosts as any other page. It names the
//! page's Router, tells the Presenter how many Colour Theme cards fit a row ([`PageView::framed`]),
//! and draws the view model (`render.rs`, `colour_theme.rs`); the keymap, the take, send and handle
//! path and the clicks' path are the generic page's. The date style, Colour Theme and Appearance
//! are written through the Presenter's `DeviceConfiguration` (`super::device`); the Display output
//! reaches the Shell as `ViewEvent::DisplayChanged` and a saved colour is applied by the page's
//! Router (`super::router`). Each key and click is one input (`crate::input::send`).

pub mod colour_theme;
pub(crate) mod render;

use std::rc::Rc;

use gpui::{AnyElement, Context, Window};
use lib_database::Domain;
use lib_settings::keymap::display_keymap;
use lib_settings::{
    DeviceConfiguration, DisplayEvent, DisplayPresenter, DisplaySetup, DisplayViewModel,
};
use lib_viper::keymap::KeyBindingConfig;

use crate::settings::SettingsPage;
use crate::settings::pages::PageFrame;
use crate::settings::router::DeviceRouter;
use crate::shell::view::module_page::{ModulePage, PageDraw, PageView, SettingsPageView};
use crate::shell::view::statusline::PageStatus;
use crate::view::help::HelpGroup;

/// The Display page: the Presenter in the generic page Entity. The Presenter holds the Display
/// output, the Colour Theme and Appearance and the keyboard focus on the controls and the Colour
/// Theme grid.
pub type DisplayPage = ModulePage<DisplayPresenter>;

/// Display writing to `device` under `config`'s keys, with the Shell's `back` on `back_chord`.
/// Display reads no Ledger, so it stands before one is open.
pub fn display_page(
    device: Rc<dyn DeviceConfiguration>,
    config: &KeyBindingConfig,
    back_chord: String,
    cx: &mut Context<'_, DisplayPage>,
) -> DisplayPage {
    let presenter = DisplayPresenter::new(DisplaySetup {
        device,
        keymap: display_keymap(config),
        back_chord,
    });
    ModulePage::detached(presenter, (), cx)
}

impl PageView for DisplayPresenter {
    type Setup = ();
    type Widgets = ();
    type Before = ();

    /// Display reads Configuration, not the Ledger.
    const REPAINTS: &'static [Domain] = &[];

    fn with_router<R>(
        presenter: &mut Self,
        _setup: &(),
        cx: &mut Context<'_, DisplayPage>,
        handle: impl FnOnce(&mut Self, &mut Self::Router<'_>) -> R,
    ) -> R {
        handle(presenter, &mut DeviceRouter { cx })
    }

    /// The page body's width sets how many Colour Theme cards fit a row.
    fn framed(frame: PageFrame) -> Option<DisplayEvent> {
        Some(DisplayEvent::SetGridColumns(colour_theme::grid_columns(
            frame.body_width,
        )))
    }

    /// The grid's focus `tab` and half-page moves leave it, then the Shell moves the focus or
    /// scrolls the page.
    fn passes_on(event: &DisplayEvent) -> bool {
        matches!(event, DisplayEvent::LeaveGrid)
    }

    /// `Esc` leaves the Colour Theme grid for the control above it. With the grid unfocused it is
    /// left to Settings, which steps back to the index.
    fn escape(view: &DisplayViewModel) -> Option<DisplayEvent> {
        view.grid.is_some().then_some(DisplayEvent::Back)
    }

    fn render(
        view: &DisplayViewModel,
        page: PageDraw<'_, Self>,
        _window: &mut Window,
        cx: &mut Context<'_, DisplayPage>,
    ) -> AnyElement {
        render::page(view, page.clicks, cx)
    }

    /// The keys the status line shows: the grid's while it has the keyboard, else the controls'.
    fn status(view: &DisplayViewModel) -> PageStatus {
        PageStatus {
            hints: view.hints.clone(),
            right: crate::settings::scope_note(SettingsPage::Display),
        }
    }

    /// The `?` cheat sheet's keys: the controls', then the grid's own, whichever has the keyboard.
    fn help(view: &DisplayViewModel) -> Option<HelpGroup> {
        Some(HelpGroup {
            heading: crate::msg::desktop_help_settings_label(),
            keys: view
                .help
                .iter()
                .map(|hint| (hint.action.clone(), hint.chord.clone()))
                .collect(),
        })
    }

    /// Focus arriving puts the keyboard on the first control; focus leaving, for the index or
    /// another page, takes it off the controls and the grid.
    fn focus_changed(&mut self, focused: bool) -> bool {
        self.set_page_focused(focused)
    }
}

impl SettingsPageView for DisplayPresenter {
    const PAGE: SettingsPage = SettingsPage::Display;

    /// The page has no selection, so a movement its keymap doesn't bind (`Ctrl-d` on a control,
    /// `G`) scrolls the Settings body, the Colour Theme grid keeping the keyboard.
    const FRAME_SCROLLS: bool = true;
}
