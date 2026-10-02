//! Headless harness for the Desktop UI: repeats `lib::run`'s app-level setup, builds `Shell`
//! through the lib's `build_shell` with a fixed date and the deterministic stub seed, and offers
//! key presses, selector clicks and read helpers. Nothing here opens a real window or touches
//! `persistence`, so it runs in a sandbox with no GPU.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "test support: a failed lookup or invalid fixed date should fail the test loudly"
)]

use bin_desktop::{
    ShellBindings, build_shell, colours,
    locale::init_for_tests,
    nav::Noun,
    persistence::PersistedState,
    shell::{
        BillsSnapshot, BudgetsSnapshot, DashboardBillSnapshot, DocumentsSnapshot, ImportSnapshot,
        SettingsSnapshot, Shell, ToastsSnapshot, TransactionsSnapshot,
    },
};
use chrono::NaiveDate;
use gpui::{Entity, Modifiers, TestAppContext, VisualTestContext};
use lib_colour_theme::ThemeOverrides;

/// The date every test is anchored to, so scopes and seeded relative dates never drift.
pub fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 6, 15).expect("fixed test date is valid")
}

/// A `Shell` in a headless window, with helpers to drive and read it.
pub struct Harness<'a> {
    pub shell: Entity<Shell>,
    pub cx: &'a mut VisualTestContext,
}

impl<'a> Harness<'a> {
    /// Boots the app-level globals and a `Shell` with default (nothing persisted) state.
    pub fn new(app: &'a mut TestAppContext) -> Self {
        // Pins the process-wide Locale to en-US (and loads this bin's Messages), so rendered text
        // never depends on the host; `main` is never run in a test.
        init_for_tests();
        app.update(|cx| {
            gpui_component::init(cx);
            colours::init(ThemeOverrides::default(), cx);
        });
        let (shell, cx) = app.add_window_view(|window, cx| {
            let focus_handle = cx.focus_handle();
            window.focus(&focus_handle);
            build_shell(
                PersistedState::default(),
                ShellBindings {
                    dismiss_toasts: "ctrl+l".to_string(),
                    toast_history: None,
                },
                today(),
                focus_handle,
                cx,
            )
        });
        Self { shell, cx }
    }

    /// Presses a space-separated keystroke sequence, e.g. `press("g f")`.
    pub fn press(&mut self, keys: &str) {
        self.cx.simulate_keystrokes(keys);
        self.cx.run_until_parked();
    }

    /// Clicks the centre of the element tagged `debug_selector(selector)`.
    pub fn click(&mut self, selector: &str) {
        // gpui's `debug_bounds` wants a `'static` key and index-tagged selectors are built at run
        // time; leaking a handful of short strings per test process is the cheap way through.
        let key: &'static str = Box::leak(selector.to_string().into_boxed_str());
        let bounds = self
            .cx
            .debug_bounds(key)
            .unwrap_or_else(|| panic!("no element tagged `{selector}` was drawn"));
        self.cx.simulate_click(bounds.center(), Modifiers::none());
        self.cx.run_until_parked();
    }

    /// The Noun the Shell is showing.
    pub fn noun(&mut self) -> Noun {
        self.read(|shell| shell.nav().noun())
    }

    /// The Documents page's state, read back for assertions.
    pub fn documents(&mut self) -> DocumentsSnapshot {
        self.read(Shell::documents_snapshot)
    }

    /// The Bills page's state, read back for assertions.
    pub fn bills(&mut self) -> BillsSnapshot {
        self.read(Shell::bills_snapshot)
    }

    /// The Budgets page's state, read back for assertions.
    pub fn budgets(&mut self) -> BudgetsSnapshot {
        self.read(Shell::budgets_snapshot)
    }

    /// The Settings pages' state, read back for assertions.
    pub fn settings(&mut self) -> SettingsSnapshot {
        self.read(Shell::settings_snapshot)
    }

    /// The Colour Theme id and Appearance now chosen, read from the colours Global.
    pub fn colour_choice(&mut self) -> (Option<String>, Option<String>) {
        self.cx.read(|cx| {
            let state = colours::colours(cx);
            (
                state.colour_theme().map(str::to_string),
                state.colour_appearance().map(|a| format!("{a:?}")),
            )
        })
    }

    /// The Transactions page's state, read back for assertions.
    pub fn transactions(&mut self) -> TransactionsSnapshot {
        self.read(Shell::transactions_snapshot)
    }

    /// The Toast layer and its history, read back for assertions.
    pub fn toasts(&mut self) -> ToastsSnapshot {
        self.read(Shell::toasts_snapshot)
    }

    /// The Import step's state; `None` while it is not showing.
    pub fn import(&mut self) -> Option<ImportSnapshot> {
        self.read(Shell::import_snapshot)
    }

    /// The Dashboard's Needs Attention Bill rows.
    pub fn dashboard_bills(&mut self) -> Vec<DashboardBillSnapshot> {
        self.read(Shell::dashboard_bills_snapshot)
    }

    /// Raises a Toast the way a call site would, then settles.
    pub fn raise_toast(&mut self, kind: lib_toast::ToastKind, text: &str) {
        self.shell.update(self.cx, |shell, cx| {
            shell.raise_toast(kind, text);
            cx.notify();
        });
        self.cx.run_until_parked();
    }

    /// Reads from the Shell without mutating it.
    pub fn read<T>(&mut self, f: impl FnOnce(&Shell) -> T) -> T {
        self.shell.read_with(self.cx, |shell, _| f(shell))
    }
}
