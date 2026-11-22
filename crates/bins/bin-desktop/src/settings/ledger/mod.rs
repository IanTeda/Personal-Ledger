//! The **Ledger** page (`docs/ux/desktop-mockups/16-settings/README.md`'s 16a), first in the THIS
//! LEDGER group: `lib-settings`' Ledger Presenter is the [`PageView`] the generic `ModulePage`
//! hosts (ADR-0056), which the Settings View hosts as any other page. The Ledger name and Owner
//! are edited in place (the Presenter's Insert mode) and Financial year starts and Base unit are
//! selects, all read through `lib-settings`' `LedgerReads` and written through its
//! `LedgerPreferences`, so the page holds none of them. The THIS LEDGER summary is the mockup's
//! placeholder figures. Clicks are Presenter events through `PageClicks`, and the view model is
//! drawn in `render.rs`. Each key and click is one input (`crate::input::send`).

mod render;

use gpui::{AnyElement, App, Context, Entity, Window};
use lib_database::Domain;
use lib_settings::{LedgerEvent, LedgerPresenter, LedgerSetup, LedgerViewModel, SettingsLedger};
use lib_viper::keymap::KeyBindingConfig;

use crate::announcer::Announcer;
use crate::settings::SettingsPage;
use crate::settings::snapshot::PageSnapshot;
use crate::shell::view::module_page::{
    ModulePage, PageCue, PageDraw, PageLayout, PageView, SettingsPageView,
};
use crate::shell::view::page_router::page_router;
use crate::shell::view::statusline::{PageStatus, StatusRight};

/// The Ledger page: the Presenter, over the Ledger and Units stores, in the generic page Entity.
pub type LedgerPage = ModulePage<LedgerPresenter>;

/// The Ledger page on `ledger`, the reads and write `LedgerPorts::settings_ledger` hands out, its
/// keys from `config`, repainting when the Units or the Ledger's Preferences change.
pub fn ledger_page(
    ledger: SettingsLedger,
    config: &KeyBindingConfig,
    announcer: &Entity<Announcer>,
    cx: &mut Context<'_, LedgerPage>,
) -> LedgerPage {
    let presenter = LedgerPresenter::new(
        LedgerSetup {
            units: ledger.units,
            summaries: ledger.summaries,
            preferences: ledger.preferences,
        },
        config,
    );
    ModulePage::new(presenter, (), announcer, cx)
}

/// What the View keeps between draws: each select's open list's scroll, as widget mechanics.
#[derive(Default)]
pub struct LedgerWidgets {
    financial_year_scroll: usize,
    base_unit_scroll: usize,
}

impl PageView for LedgerPresenter {
    type Setup = ();
    type Widgets = LedgerWidgets;
    type Before = ();

    /// The Presenter reads the Units and the Ledger's Preferences on each draw, so a write
    /// anywhere, an Undo among them, repaints the page.
    const REPAINTS: &'static [Domain] = &[
        Domain::Units,
        Domain::Institutions,
        Domain::Accounts,
        Domain::Transactions,
        Domain::Ledger,
    ];

    /// With the keyboard off the select, the frame scrolls the page.
    fn layout() -> PageLayout {
        PageLayout {
            frame_scrolls: true,
            ..PageLayout::default()
        }
    }

    page_router!();

    fn render(
        view: &LedgerViewModel,
        page: PageDraw<'_, Self>,
        _window: &mut Window,
        cx: &mut Context<'_, LedgerPage>,
    ) -> AnyElement {
        render::page(view, page.clicks, page.widgets, cx)
    }

    /// `Esc` discards a draft or collapses an open list; with nothing open it steps back to the
    /// index.
    fn escape(view: &LedgerViewModel) -> Option<LedgerEvent> {
        view.open().then_some(LedgerEvent::Cancel)
    }

    /// The Presenter's legend, rebound keys included, which the `?` cheat sheet reads.
    fn status(view: &LedgerViewModel) -> PageStatus {
        PageStatus {
            hints: view.hints.clone(),
            right: StatusRight::of_settings_page(SettingsPage::Ledger),
        }
    }

    /// Focus arriving puts the keyboard on Ledger name; focus leaving saves what is open.
    fn cued(cue: PageCue) -> Option<LedgerEvent> {
        match cue {
            PageCue::Focused(focused) => Some(LedgerEvent::Focused(focused)),
            _ => None,
        }
    }
}

impl SettingsPageView for LedgerPresenter {
    const PAGE: SettingsPage = SettingsPage::Ledger;
}

impl PageSnapshot for SettingsLedgerSnapshot {
    type Presenter = LedgerPresenter;

    /// The fields' state for a test: what each shows, which has the keyboard, and the selects.
    fn read(page: &LedgerPage, _cx: &App) -> Self {
        let view = page.view_model();
        let focused = [
            ("name", view.name.focused),
            ("owner", view.owner.focused),
            ("financial_year", view.financial_year.focused),
            ("base_unit", view.base_unit.focused),
        ]
        .into_iter()
        .find(|(_, focused)| *focused)
        .map(|(field, _)| field);
        let select = view.base_unit;
        let summary = view.summary;
        SettingsLedgerSnapshot {
            name: view.name.text,
            owner: view.owner.text,
            editing: view.name.editing || view.owner.editing,
            refused: view.name.refused || view.owner.refused,
            focused,
            financial_year: view
                .financial_year
                .options
                .get(view.financial_year.chosen)
                .cloned(),
            list_open: view.financial_year.highlight.is_some() || select.highlight.is_some(),
            base_unit: select.chosen.and_then(|chosen| {
                select
                    .options
                    .iter()
                    .find(|option| option.id == chosen)
                    .map(|option| option.code.clone())
            }),
            summary,
            options: select
                .options
                .into_iter()
                .map(|option| option.code)
                .collect(),
        }
    }
}

/// The Ledger page for a test: each field as drawn, the field with the keyboard, the base Unit's
/// code and the select's option codes in order.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsLedgerSnapshot {
    /// The Ledger name as shown: the draft while editing it.
    pub name: String,
    /// The Owner as shown, empty for none.
    pub owner: String,
    /// Whether a text field is being edited.
    pub editing: bool,
    /// Whether the draft's last save was refused.
    pub refused: bool,
    /// The field with the keyboard: `name`, `owner`, `financial_year` or `base_unit`.
    pub focused: Option<&'static str>,
    /// The chosen Financial year start's month as shown.
    pub financial_year: Option<String>,
    /// Whether a select's list is open.
    pub list_open: bool,
    pub base_unit: Option<String>,
    pub options: Vec<String>,
    /// THIS LEDGER's label and figure pairs, as drawn.
    pub summary: Vec<(String, String)>,
}
