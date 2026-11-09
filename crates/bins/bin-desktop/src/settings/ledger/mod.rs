//! The **Ledger** page (`docs/ux/desktop-mockups/16-settings/README.md`'s 16a), first in the THIS
//! LEDGER group: `lib-settings`' Ledger Presenter is the [`PageView`] the generic `ModulePage`
//! hosts (ADR-0056), which the Settings View hosts as any other page. The identity fields and the
//! summary are the mockup's placeholder values; the *Base unit* select is real, read from the
//! Units store through `lib-database`'s `StoredSettingsReads` and set through its
//! `LedgerPreferencesAdapter`, so the page holds no Units. Clicks are Presenter events through
//! `PageClicks`, and the view model is drawn in `render.rs`. Each key and click is one input
//! (`crate::input::send`).

mod render;

use std::rc::Rc;

use gpui::{AnyElement, App, Context, Entity, Window};
use lib_database::Domain;
use lib_database::{LedgerPorts, LedgerPreferencesAdapter, StoredSettingsReads};
use lib_settings::keymap::ledger_keymap;
use lib_settings::{LedgerEvent, LedgerPresenter, LedgerSetup, LedgerViewModel};
use lib_viper::keymap::{KeyBindingConfig, Keymap};

use crate::announcer::Announcer;
use crate::settings::SettingsPage;
use crate::settings::router::LedgerRouter;
use crate::settings::snapshot::PageSnapshot;
use crate::shell::view::module_page::{ModulePage, PageDraw, PageView, SettingsPageView};
use crate::shell::view::statusline::PageStatus;

/// The Ledger page: the Presenter, over the Units store, in the generic page Entity.
pub type LedgerPage = ModulePage<LedgerPresenter>;

/// What `shell::router::build` hands the Ledger page: its keymap and the `back` chord, under
/// `[keybindings]`.
pub struct LedgerKeys {
    pub keymap: Keymap<LedgerEvent>,
    pub back_chord: String,
    /// Settings' reads `LedgerPorts` built (the Units and the index pane's counts) and the base
    /// Unit write.
    pub settings_reads: Rc<StoredSettingsReads>,
    pub preferences: Rc<LedgerPreferencesAdapter>,
}

impl LedgerKeys {
    pub fn new(config: &KeyBindingConfig, ports: &LedgerPorts) -> Self {
        Self {
            settings_reads: ports.settings_reads.clone(),
            preferences: ports.ledger_preferences.clone(),
            keymap: ledger_keymap(config),
            back_chord: crate::settings::state::back_chord(config),
        }
    }
}

/// The Ledger page on `keys`, repainting when the Units change.
pub fn ledger_page(
    keys: LedgerKeys,
    announcer: &Entity<Announcer>,
    cx: &mut Context<'_, LedgerPage>,
) -> LedgerPage {
    let presenter = LedgerPresenter::new(LedgerSetup {
        units: keys.settings_reads,
        preferences: keys.preferences,
        keymap: keys.keymap,
        back_chord: keys.back_chord,
    });
    ModulePage::new(presenter, (), announcer, cx)
}

impl PageView for LedgerPresenter {
    type Setup = ();
    type Widgets = ();
    type Before = ();

    /// The Presenter reads the Units on each draw, so a write anywhere repaints the page.
    const REPAINTS: &'static [Domain] = &[Domain::Units];

    fn with_router<R>(
        presenter: &mut Self,
        _setup: &(),
        _cx: &mut Context<'_, LedgerPage>,
        handle: impl FnOnce(&mut Self, &mut Self::Router<'_>) -> R,
    ) -> R {
        handle(presenter, &mut LedgerRouter)
    }

    fn render(
        view: &LedgerViewModel,
        page: PageDraw<'_, Self>,
        _window: &mut Window,
        cx: &mut Context<'_, LedgerPage>,
    ) -> AnyElement {
        render::page(view, page.clicks, cx)
    }

    /// The Presenter's legend, rebound keys included, which the `?` cheat sheet reads.
    fn status(view: &LedgerViewModel) -> PageStatus {
        PageStatus {
            hints: view.hints.clone(),
            right: crate::settings::scope_note(SettingsPage::Ledger),
        }
    }

    fn focus_changed(&mut self, focused: bool) -> bool {
        self.set_page_focused(focused)
    }
}

impl SettingsPageView for LedgerPresenter {
    const PAGE: SettingsPage = SettingsPage::Ledger;

    /// With the keyboard off the select, the frame scrolls the page.
    const FRAME_SCROLLS: bool = true;
}

impl PageSnapshot for SettingsLedgerSnapshot {
    type Presenter = LedgerPresenter;

    /// The select's state for a test: the base Unit's code and the options' codes in order.
    fn read(page: &LedgerPage, _cx: &App) -> Self {
        let select = page.view_model().base_unit;
        SettingsLedgerSnapshot {
            base_unit: select.chosen.and_then(|chosen| {
                select
                    .options
                    .iter()
                    .find(|option| option.id == chosen)
                    .map(|option| option.code.clone())
            }),
            options: select
                .options
                .into_iter()
                .map(|option| option.code)
                .collect(),
        }
    }
}

/// The Ledger page for a test: the base Unit's code and the select's option codes in order.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsLedgerSnapshot {
    pub base_unit: Option<String>,
    pub options: Vec<String>,
}
