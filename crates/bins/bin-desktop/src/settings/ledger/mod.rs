//! The **Ledger** page (`docs/ux/desktop-mockups/16-settings/README.md`'s 16a), first in the THIS
//! LEDGER group: `lib-settings`' Ledger Presenter in a gpui Entity, which the Settings View hosts
//! as any other page. The identity fields and the summary are the mockup's placeholder values; the
//! *Base unit* select is real, read from the Units store through `lib-database`'s
//! `StoredSettingsReads` and set through its `LedgerPreferencesAdapter`, so the page holds no
//! Units and reads none from the `Ledger`. It looks up each key the Shell passes on in the page's
//! keymap, turns clicks into events through `PageClicks` and draws the view model (`render.rs`).
//! Each key and click is one input (`crate::input::send`).

mod render;

use std::rc::Rc;

use gpui::{App, Context, Entity, EventEmitter, Subscription};
use lib_database::{LedgerPreferencesAdapter, SharedUnitsStore, StoredSettingsReads};
use lib_ledger::Domain;
use lib_settings::keymap::ledger_keymap;
use lib_settings::{LedgerEvent, LedgerPresenter, LedgerSetup, LedgerViewModel};
use lib_shell::Hint;
use lib_viper::keymap::{KeyBindingConfig, Keymap};

use crate::settings::snapshot::PageSnapshot;
use crate::{
    ledger_store::{LedgerStore, notify_on},
    settings::{
        SettingsPage,
        pages::{Page, PageRowClicked},
        router::LedgerRouter,
    },
    shell::view::page_clicks::PresenterPage,
    units::DesktopPorts,
    view::event::ViewEvent,
};

impl EventEmitter<ViewEvent> for LedgerPage {}

/// The page has no rows, so it never emits a row click.
impl EventEmitter<PageRowClicked> for LedgerPage {}

/// What `shell::router::build` hands the Ledger page: its keymap and the `back` chord, under
/// `[keybindings]`.
pub struct LedgerKeys {
    pub keymap: Keymap<LedgerEvent>,
    pub back_chord: String,
}

impl LedgerKeys {
    pub fn new(config: &KeyBindingConfig) -> Self {
        Self {
            keymap: ledger_keymap(config),
            back_chord: crate::settings::state::back_chord(config),
        }
    }
}

/// The Ledger page: its Presenter, and the Units ports and read copy a base Unit change brings in
/// line (`crate::units::ports::refresh`). The page itself reads neither.
pub struct LedgerPage {
    presenter: LedgerPresenter,
    ports: DesktopPorts,
    ledger: Entity<LedgerStore>,
    _units_changed: Subscription,
}

impl LedgerPage {
    pub fn new(
        ledger: Entity<LedgerStore>,
        ports: DesktopPorts,
        keys: LedgerKeys,
        cx: &mut Context<'_, Self>,
    ) -> Self {
        // The Presenter reads the Units on each draw, so a write anywhere repaints the page.
        let units_changed = notify_on(&ledger, Domain::Units, cx);
        let store: SharedUnitsStore = ports.clone();
        let presenter = LedgerPresenter::new(LedgerSetup {
            units: Rc::new(StoredSettingsReads {
                units: store.clone(),
            }),
            preferences: Rc::new(LedgerPreferencesAdapter::new(store)),
            keymap: keys.keymap,
            back_chord: keys.back_chord,
        });
        Self {
            presenter,
            ports,
            ledger,
            _units_changed: units_changed,
        }
    }

    /// The page's keys under `config`'s overrides.
    pub fn set_keymaps(&mut self, config: &KeyBindingConfig, back_chord: String) {
        self.presenter.set_keymap(ledger_keymap(config), back_chord);
    }

    pub fn view_model(&self) -> LedgerViewModel {
        self.presenter.view_model()
    }

    /// A key or click as one input, committed once it is handled.
    fn send(&mut self, event: LedgerEvent, cx: &mut Context<'_, Self>) {
        if crate::input::sending(cx) {
            self.handle(event, cx);
        } else {
            crate::input::send(cx, |cx| self.handle(event, cx));
        }
    }

    fn handle(&mut self, event: LedgerEvent, cx: &mut Context<'_, Self>) {
        let mut router = LedgerRouter {
            ports: &self.ports,
            ledger: &self.ledger,
            cx,
        };
        self.presenter.handle(event, &mut router);
        cx.notify();
    }
}

/// The page's clicks on the Base unit select's options.
impl PresenterPage<LedgerEvent> for LedgerPage {
    fn send(&mut self, event: LedgerEvent, cx: &mut Context<'_, Self>) {
        LedgerPage::send(self, event, cx);
    }
}

impl PageSnapshot for SettingsLedgerSnapshot {
    type Page = LedgerPage;

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

impl Page for LedgerPage {
    const PAGE: SettingsPage = SettingsPage::Ledger;

    /// The keymap's steps while the page has the keyboard, ahead of the frame's; the rest go on to
    /// the frame.
    fn module_key(&mut self, chord: &str, cx: &mut Context<'_, Self>) -> bool {
        let Some(event) = self.presenter.keymap().event(chord) else {
            return false;
        };
        if !self.presenter.takes(&event) {
            return false;
        }
        self.send(event, cx);
        true
    }

    fn set_page_focused(&mut self, focused: bool, cx: &mut Context<'_, Self>) {
        if self.presenter.set_page_focused(focused) {
            cx.notify();
        }
    }

    /// The Presenter's legend, rebound keys included, which the `?` cheat sheet reads.
    fn hints(&self, _cx: &App) -> Vec<Hint> {
        self.presenter.view_model().hints
    }
}
