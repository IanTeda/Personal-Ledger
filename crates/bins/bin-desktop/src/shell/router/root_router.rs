//! The root Router: what `lib-shell`'s Presenter asks of the Desktop, and the one place one module
//! is called from another (#820). [`Root::with_router`] lends it every field of `Root` but the
//! Presenter for one call, so the Presenter acts on its own state and calls the Router for what
//! reaches a View or the frame, inside the input it is handling ([`Root::send_with_router`]): one
//! commit covers whatever the key ran. A module call it reaches joins that input, through the
//! page's `handle` or `join_or_send`, never a `send` of its own (#929). An `impl Root` block of the
//! root Router, which is `Root` itself, and the Router struct it lends.

use std::path::Path;

use chrono::NaiveDate;
use gpui::{App, AppContext as _, Context, Entity, ScrollHandle};
use lib_core::Clock;
use lib_database::{Storage, WriteFailure};
use lib_shell::{
    HelpGroup, LedgerFolders, Movement, PaletteRun, ShellDialog, ShellInput, ShellKey, ShellRouter,
};
use lib_toast::{Notice, Notify, ToastKind};

use super::{LedgerFile, OpenLedger, Root, ShellPresenter, explorer::file_name};
use crate::documents::{DocumentsMode, LibraryScope, LibrarySort};
use crate::navigation::active_view::ActiveView;
use crate::navigation::command::{CommandEffect, ModulePage};
use crate::navigation::explorer::FsLedgerFolders;
use crate::navigation::nav::Noun;
use crate::settings::{SettingsPage, state::SettingsView};
use crate::shell::state::ShellState;
use crate::shell::view::dialog_entity::{DialogHandle, ModuleDialog};
use crate::shell::view::page_handle::PageHandle;
use crate::shell::view::rail as rail_view;
use crate::shell::view::window_size::WindowSize;
use crate::view::active;
use lib_settings::DisplayOutput;

/// The Documents mode, Library scope and sort a Ledger's Documents page opens on.
type DocumentsState = (DocumentsMode, LibraryScope, LibrarySort);

/// Every part of `Root` but the Shell's Presenter, borrowed for one call of it.
pub(super) struct RootRouter<'a, 'b> {
    /// The Open Ledger, `None` with no Ledger open; `:open`, `:new` and `:close` replace it.
    pub(super) open: &'a mut Option<OpenLedger>,
    settings: &'a Entity<SettingsView>,
    state: &'a mut ShellState,
    view_x_scroll_handle: &'a ScrollHandle,
    documents_state: &'a mut Option<DocumentsState>,
    last_ledger_forgotten: &'a mut bool,
    keybindings: &'a lib_config::KeyBindingConfig,
    /// The Shell's Clock, which a Ledger opened now is built on.
    clock: Clock,
    /// The Window Size the window is drawn at, which the rail toggle flips the rail for.
    window_size: WindowSize,
    pub(super) cx: &'a mut Context<'b, Root>,
}

impl Notify for RootRouter<'_, '_> {
    /// Queues `notice`, for the input's `send` to drain into the Shell.
    fn notify(&mut self, notice: Notice) {
        crate::notices::Queue.notify(notice);
    }
}

impl ShellRouter for RootRouter<'_, '_> {
    type Failure = WriteFailure;
    type Effect = CommandEffect;

    /// Generates the new month's Bill Schedule, committed as the day poll's one write. The Dashboard
    /// keeps its view model by the day, so it rebuilds on its own. The Budgets page moves a month on
    /// its old one on to the new, and the Documents page bands its Key Dates against the new day and
    /// drops a Financial Year scope that has left the rail. The Bills page catches up on its own next
    /// event too, which finds nothing left to generate. With no Ledger open there is nothing to move.
    fn month_turned(&mut self, today: NaiveDate) {
        let Some(open) = self.open.as_ref() else {
            return;
        };
        crate::bills::new_month(&open.stores.ports.bills, today);
        open.views.budgets.update(self.cx, |budgets, cx| {
            budgets.send(lib_budgets::presenter::BudgetsPageEvent::MonthTurned, cx);
        });
        open.views.documents.update(self.cx, |documents, cx| {
            documents.send(
                lib_documents::presenter::DocumentsPageEvent::MonthTurned,
                cx,
            );
        });
    }

    /// Puts each domain `failure` names back as stored, with an Error Toast for each. A failure
    /// arriving with no Ledger open has nothing left to put back.
    fn write_failed(&mut self, failure: WriteFailure) {
        let Some(open) = self.open.as_ref() else {
            return;
        };
        for notice in super::write_failures::roll_back(&failure, open, self.cx) {
            self.notify(notice);
        }
    }

    /// The handoff: "`g` + an unbound key is a no-op: clear the pending prefix and flash the hint
    /// strip."
    fn not_a_jump(&mut self, sequence: String) {
        self.notify(Notice::Status(crate::msg::desktop_status_not_a_jump(
            &sequence,
        )));
    }

    fn ledger_open(&self) -> bool {
        self.open.is_some()
    }

    fn refused_without_ledger(&mut self) {
        self.notify(Notice::Status(
            crate::msg::desktop_status_refused_no_ledger(":open", ":new"),
        ));
    }

    /// Drops a settled collapsed-rail tooltip, meaningless once the rail that anchors it changes
    /// shape.
    fn toggle_rail(&mut self) -> (WindowSize, bool) {
        self.state.collapsed_rail_tooltip = None;
        let start_hidden = self.display_output().start_sidebar_minimised;
        (self.window_size, start_hidden)
    }

    /// A View the Shell still draws (a placeholder, or the no Ledger open content) has no Search
    /// box, but `/` enters Search mode on it as it always has, for `esc` to leave.
    fn start_search(&mut self, noun: Noun) -> bool {
        match active_page(noun, self.settings, self.open.as_ref(), self.cx) {
            None => true,
            Some(page) => page.start_search(self.cx),
        }
    }

    fn view_back(&mut self, noun: Noun) -> bool {
        active_page(noun, self.settings, self.open.as_ref(), self.cx)
            .is_some_and(|page| page.escape(self.cx))
    }

    /// The Settings frame's own: it scrolls the body or steps its index. Every other View binds
    /// the movements it takes in its keymap, and a placeholder, or the no Ledger open content, has
    /// nothing to move through.
    fn move_selection(&mut self, noun: Noun, movement: Movement) {
        if let ActiveView::Settings(_) =
            active_view(noun, self.settings, self.open.as_ref(), self.cx)
        {
            self.settings
                .update(self.cx, |view, cx| view.apply_movement(movement, cx));
        }
    }

    fn context_rail_count(&self, noun: Noun) -> usize {
        rail_view::context::entity_count(noun)
    }

    fn reset_view_scroll(&mut self, noun: Noun) {
        let view = active_view(noun, self.settings, self.open.as_ref(), self.cx);
        reset_view_scroll(
            self.view_x_scroll_handle,
            view,
            self.settings,
            self.open.as_ref(),
            self.cx,
        );
    }

    fn help_group(&mut self, noun: Noun) -> Option<HelpGroup> {
        active_page(noun, self.settings, self.open.as_ref(), self.cx)
            .and_then(|page| page.help_group(self.cx))
    }

    fn host_dialog(&mut self, dialog: ShellDialog) {
        let handle = match dialog {
            ShellDialog::Help(presenter) => {
                DialogHandle::new(self.cx.new(|_| ModuleDialog::new(presenter, |(), _| {})))
            }
            ShellDialog::ToastHistory(presenter) => {
                DialogHandle::new(self.cx.new(|_| ModuleDialog::new(presenter, |(), _| {})))
            }
        };
        super::dialogs::host(self.state, handle, self.cx);
    }

    /// Flushes the Open Ledger's writes and drops everything built on it. A write that failed
    /// since the last poll raises one Error Toast, since there is nothing left to put back. The
    /// remembered last Ledger is cleared, so the next run starts with none.
    fn close_ledger(&mut self) {
        let Some(open) = self.open.take() else {
            return;
        };
        *self.documents_state = Some(open.views.documents.read(self.cx).presenter().persisted());
        self.report_failures_on_close(&open);
        crate::input::close_ledger(self.cx);
        // Dropped here, once the failures are read: the writer flushes again, which finds nothing.
        drop(open);
        *self.last_ledger_forgotten = true;
        self.settings
            .update(self.cx, |view, cx| view.close_ledger(cx));
        self.set_last_ledger(None);
    }

    /// Runs `run`'s effect on its destination, which the Shell has shown: one exhaustive match over
    /// [`CommandEffect`], the one source of truth for what a Command does. The Shell runs its own
    /// effects ([`CommandEffect::palette_effect`]), so they never reach here.
    fn run_effect(&mut self, run: PaletteRun<CommandEffect>) {
        let cx = &mut *self.cx;
        match run.effect {
            CommandEffect::OpenSettingsPage(section) => {
                self.settings
                    .update(cx, |view, cx| view.open_page(section, cx));
            }
            CommandEffect::Accounts(verb) => {
                self.settings
                    .update(cx, |view, cx| view.open_page(SettingsPage::Accounts, cx));
                let page = self
                    .settings
                    .read(cx)
                    .page::<lib_accounts::presenter::AccountsPagePresenter>();
                crate::accounts::router::run_command(&page, run.name, verb, &run.argument, cx);
            }
            CommandEffect::Colour(colour) => {
                self.settings
                    .update(cx, |view, cx| view.choose_colour(colour, cx));
            }
            CommandEffect::SetToasts(on) => {
                self.settings.update(cx, |view, cx| view.set_toasts(on, cx));
            }
            CommandEffect::Module { page, command } => {
                if !self.run_module_command(page, command) {
                    self.notify(Notice::Status(crate::msg::desktop_status_command_refused(
                        run.name,
                    )));
                }
            }
            // The step opens over the list, from its top.
            CommandEffect::Import => {
                self.reset_view_scroll(Noun::Transactions);
                if let Some(open) = self.open.as_ref() {
                    open.views.import.update(self.cx, |view, cx| {
                        view.send(lib_transaction_import::presenter::ImportEvent::OpenStep, cx);
                    });
                }
            }
            CommandEffect::NotYetBuilt => {
                self.notify(Notice::Status(
                    crate::msg::desktop_status_command_not_yet_built(run.name),
                ));
            }
            CommandEffect::OpenDialog(_)
            | CommandEffect::CloseLedger
            | CommandEffect::DismissToast
            | CommandEffect::Shell(_) => {}
        }
    }

    fn folders(&self) -> &dyn LedgerFolders {
        &FsLedgerFolders
    }

    /// Opens the Ledger file at `path` in place of the Open Ledger, if any, saying so. The Open
    /// Ledger's writes land first, so reopening its own file reads them back; it is swapped out
    /// only once the new file has opened, so a file that won't open leaves it as it was, or none
    /// open, and an Error Toast says why. Never creates a file.
    fn open_ledger(&mut self, path: &Path) -> bool {
        self.flush();
        match crate::open_ledger_file(path, self.clock.today()) {
            Ok(storage) => {
                self.swap_in_ledger(path, storage);
                self.notify(Notice::Toast(
                    ToastKind::Success,
                    lib_locale::msg::toast_ledger_opened(&file_name(path)),
                ));
                true
            }
            Err(failure) => {
                self.notify(ledger_open_failed(&failure));
                false
            }
        }
    }

    /// Opens the Create ledger Dialog on `path`, which writes nothing until Create. Returns
    /// `false` either way: the explorer stays up under the Dialog, for another pick if Esc
    /// cancels it, and [`Root::ledger_created`] finishes the job once Create has written the file.
    fn create_ledger(&mut self, path: &Path) -> bool {
        let root = self.cx.weak_entity();
        let dialog = crate::ledger_creation::open(
            path.to_path_buf(),
            move |created, cx| {
                // The Dialog is mid-input, inside the Shell's own update: the Shell opens the new
                // Ledger once that update ends.
                let root = root.clone();
                cx.defer(move |cx| {
                    // A Shell gone with its window has no Ledger to open.
                    let _ = root.update(cx, |root, cx| root.ledger_created(&created.path, cx));
                });
            },
            self.cx,
        );
        super::dialogs::host(self.state, DialogHandle::new(dialog), self.cx);
        false
    }
}

impl RootRouter<'_, '_> {
    /// Opens the Ledger file the Create ledger Dialog has just written, in place of the Open
    /// Ledger, if any, as [`ShellRouter::open_ledger`] does, but saying it was created. The Open
    /// Ledger's writes land first. Whether it opened; one that didn't says why and is removed, so
    /// the failed create leaves no file.
    pub(super) fn open_created(&mut self, path: &Path) -> bool {
        self.flush();
        match crate::open_created_ledger_file(path, self.clock.today()) {
            Ok(storage) => {
                self.swap_in_ledger(path, storage);
                self.notify(Notice::Toast(
                    ToastKind::Success,
                    lib_locale::msg::toast_ledger_created(&file_name(path)),
                ));
                true
            }
            Err(failure) => {
                self.notify(ledger_open_failed(&failure));
                false
            }
        }
    }

    /// The Display output, which Settings › Display's Presenter holds.
    fn display_output(&self) -> DisplayOutput {
        self.settings.read(self.cx).display_output(self.cx)
    }

    /// Waits for the Open Ledger's writes to land in its file, before another file is opened in
    /// its place.
    fn flush(&self) {
        if let Some(open) = self.open.as_ref() {
            open.flush();
        }
    }

    /// Makes the Ledger file at `path`, open on `storage`, the Open Ledger, remembering it for the
    /// next start. The Shell lands on its Dashboard.
    fn swap_in_ledger(&mut self, path: &Path, storage: Storage) {
        self.install_ledger(LedgerFile::opened(path.to_path_buf(), storage));
        self.set_last_ledger(Some(path.to_path_buf()));
    }

    /// Opens `ledger` as the Open Ledger, building everything on it under the `[keybindings]` in
    /// effect, in place of any Ledger open before, which is dropped once the new one is built.
    pub(super) fn install_ledger(&mut self, ledger: LedgerFile) {
        let opened = OpenLedger::build(
            ledger,
            self.keybindings,
            &self.clock,
            self.settings,
            self.cx,
        );
        if let Some(open) = self.open.as_ref() {
            *self.documents_state =
                Some(open.views.documents.read(self.cx).presenter().persisted());
        }
        if let Some(replaced) = self.open.replace(opened) {
            // Reported before it drops, as `:close` does, so a write it lost is never silent.
            self.report_failures_on_close(&replaced);
        }
        if let Some((mode, scope, sort)) = *self.documents_state
            && let Some(open) = self.open.as_ref()
        {
            open.views.documents.update(self.cx, |page, cx| {
                page.send(
                    lib_documents::presenter::DocumentsPageEvent::Restore { mode, scope, sort },
                    cx,
                );
            });
        }
        *self.last_ledger_forgotten = false;
        // The Views that read their keys and Display Preferences after construction take
        // `[keybindings]` and the Preferences here, so the Open Ledger comes up under them whoever
        // opens it.
        set_view_keymaps(self.settings, self.open.as_ref(), self.keybindings, self.cx);
        let display = self.display_output();
        push_display_preferences(self.open.as_ref(), display, self.cx);
    }

    /// Flushes the closing Ledger `open` and reports a write that failed since the last poll: each
    /// is logged, and one Error Toast names the file, since there is nothing left to put back.
    fn report_failures_on_close(&mut self, open: &OpenLedger) {
        let failures = open.flush_and_take_failures();
        for failure in &failures {
            tracing::error!(
                error = %failure.error,
                domains = ?failure.domains,
                "A write failed as the Ledger closed"
            );
        }
        tracing::info!(path = ?open.path(), "Closed the Ledger file");
        if !failures.is_empty() {
            let name = open.path().map(file_name).unwrap_or_default();
            self.notify(Notice::Toast(
                ToastKind::Error,
                crate::msg::desktop_toast_write_failed_on_close(&name),
            ));
        }
    }

    /// Tells the General page which Ledger file the state file will remember at quit.
    fn set_last_ledger(&mut self, last_ledger: Option<std::path::PathBuf>) {
        self.settings
            .update(self.cx, |view, cx| view.set_last_ledger(last_ledger, cx));
    }

    /// A module Command on its page, which the Shell has shown (Settings' page, opened here with
    /// focus in it): the page's Router takes the event its keymap binds `command` to, inside the
    /// input, as the key's is. Whether the page took it: a This Ledger page Settings refused to
    /// open with no Ledger open takes nothing.
    fn run_module_command(&mut self, page: ModulePage, command: &'static str) -> bool {
        let cx = &mut *self.cx;
        if let ModulePage::Settings(section) = page
            && !self
                .settings
                .update(cx, |view, cx| view.open_page(section, cx))
        {
            return false;
        }
        let views = self.open.as_ref().map(|open| &open.views);
        active::page(page.view(), views, self.settings)
            .is_some_and(|handle| handle.module_command(command, cx))
    }
}

/// The View showing `noun`: Settings' page, or the Transactions destination's Import step while
/// it is open.
fn active_view(
    noun: Noun,
    settings: &Entity<SettingsView>,
    open: Option<&OpenLedger>,
    cx: &App,
) -> ActiveView {
    ActiveView::derive(
        noun,
        settings.read(cx).selected_section(),
        open.is_some_and(|open| open.views.import.read(cx).presenter().is_open()),
    )
}

/// The View showing `noun` through the one hosting trait, `None` for a View `Root` still draws (a
/// placeholder, or with no Ledger open the no Ledger open content).
fn active_page<'a>(
    noun: Noun,
    settings: &'a Entity<SettingsView>,
    open: Option<&'a OpenLedger>,
    cx: &App,
) -> Option<&'a dyn PageHandle> {
    active::page(
        active_view(noun, settings, open, cx),
        open.map(|open| &open.views),
        settings,
    )
}

/// A new noun's view is a different (usually much shorter) length -- carrying over the old scroll
/// offset could leave it scrolled past all its content, rendering blank. Every fresh noun starts
/// scrolled to the top, and the View that is now the destination is told (`on_enter`), so it can
/// reset its own scroll and focus. The Import step is the Transactions destination's second View:
/// entering it closes the step, so the list it uncovers is told too.
pub(super) fn reset_view_scroll(
    view_x_scroll_handle: &ScrollHandle,
    view: ActiveView,
    settings: &Entity<SettingsView>,
    open: Option<&OpenLedger>,
    cx: &mut App,
) {
    view_x_scroll_handle.set_offset(gpui::Point::default());
    let views = open.map(|open| &open.views);
    if let Some(page) = active::page(view, views, settings) {
        page.on_enter(cx);
    }
    if let (ActiveView::Import, Some(views)) = (view, views) {
        views.transactions.on_enter(cx);
    }
}

/// The keys of Settings and the Open Ledger's Views, under `config`'s overrides: one input, so the
/// rebind costs one commit however many pages hear it.
pub(super) fn set_view_keymaps(
    settings: &Entity<SettingsView>,
    open: Option<&OpenLedger>,
    config: &lib_config::KeyBindingConfig,
    cx: &mut App,
) {
    crate::input::join_or_send(cx, |cx| {
        settings.update(cx, |view, cx| view.set_keymaps(config, cx));
        if let Some(open) = open {
            set_ledger_keymaps(open, config, cx);
        }
    });
}

/// The keys of the Open Ledger's Views and the Dialogs they open, under `config`'s overrides.
fn set_ledger_keymaps(open: &OpenLedger, config: &lib_config::KeyBindingConfig, cx: &mut App) {
    open.dialog_keys.set(lib_viper::dialog::DialogKeys::of(
        &lib_shell::dialog_keymap(config),
    ));
    open.views.bills.update(cx, |page, cx| {
        page.update_presenter(cx, |presenter| {
            presenter.set_keymap(lib_bills::keymap::bills_page_keymap(config));
        });
    });
    open.views.transactions.update(cx, |page, cx| {
        page.send(
            lib_transactions::presenter::TransactionsPageEvent::KeysChanged(config.clone()),
            cx,
        );
    });
    open.views.import.update(cx, |page, cx| {
        page.send(
            lib_transaction_import::presenter::ImportEvent::KeysChanged(config.clone()),
            cx,
        );
    });
    open.views.dashboard.update(cx, |page, cx| {
        page.send(
            lib_dashboard::DashboardEvent::KeysChanged(config.clone()),
            cx,
        );
    });
    open.views.budgets.update(cx, |page, cx| {
        page.send(
            lib_budgets::presenter::BudgetsPageEvent::KeysChanged(config.clone()),
            cx,
        );
    });
    open.views.documents.update(cx, |page, cx| {
        page.send(
            lib_documents::presenter::DocumentsPageEvent::KeysChanged(config.clone()),
            cx,
        );
    });
}

/// Hands the Display output to the Views that read it, after a Settings change; with no Ledger open
/// there are none, and opening one hands it over. One input, so one change costs one commit.
pub(super) fn push_display_preferences(
    open: Option<&OpenLedger>,
    display: DisplayOutput,
    cx: &mut App,
) {
    let Some(open) = open else {
        return;
    };
    crate::input::join_or_send(cx, |cx| push_ledger_display(open, display, cx));
}

/// Hands the Display output to the Open Ledger's Views that read it.
fn push_ledger_display(open: &OpenLedger, display: DisplayOutput, cx: &mut App) {
    let (date_style, glyphs, density) = (
        display.date_style,
        display.status_glyphs,
        display.row_density,
    );
    open.views.dashboard.update(cx, |page, cx| {
        page.send(
            lib_dashboard::DashboardEvent::DisplayChanged { date_style, glyphs },
            cx,
        );
    });
    open.views.bills.update(cx, |page, cx| {
        page.send(
            lib_bills::presenter::BillsPageEvent::DisplayChanged { date_style, glyphs },
            cx,
        );
    });
    open.views.transactions.update(cx, |page, cx| {
        use lib_transactions::presenter::TransactionsPageEvent;
        page.send(TransactionsPageEvent::SetDateStyle(date_style), cx);
        page.send(
            TransactionsPageEvent::DisplayChanged { glyphs, density },
            cx,
        );
    });
    open.views.documents.update(cx, |page, cx| {
        page.send(
            lib_documents::presenter::DocumentsPageEvent::SetDateStyle(date_style),
            cx,
        );
    });
    open.views.budgets.update(cx, |page, cx| {
        page.send(
            lib_budgets::presenter::BudgetsPageEvent::SetDateStyle(date_style),
            cx,
        );
    });
}

/// An Error Toast saying a Ledger file couldn't be opened, naming it and why (#977).
pub(super) fn ledger_open_failed(failure: &crate::OpenFailure) -> Notice {
    Notice::Toast(
        ToastKind::Error,
        lib_locale::msg::toast_ledger_open_failed(&file_name(&failure.path), &failure.reason),
    )
}

impl Root {
    /// Lends `act` the Shell's Presenter and the root Router over every other part of `Root`, the
    /// window drawn at `window_size`. A palette it opened starts at its top, and the file
    /// explorer's row list follows the folder it is on.
    pub(super) fn with_router<T>(
        &mut self,
        window_size: WindowSize,
        cx: &mut Context<'_, Self>,
        act: impl FnOnce(&mut ShellPresenter, &mut RootRouter<'_, '_>) -> T,
    ) -> T {
        let palette_was_open = self.presenter.palette_open();
        let Root {
            presenter,
            open,
            settings,
            state,
            view_x_scroll_handle,
            documents_state,
            last_ledger_forgotten,
            keybindings,
            ..
        } = self;
        let mut router = RootRouter {
            open,
            settings,
            state,
            view_x_scroll_handle,
            documents_state,
            last_ledger_forgotten,
            keybindings,
            clock: presenter.clock().clone(),
            window_size,
            cx,
        };
        let acted = act(presenter, &mut router);
        if !palette_was_open && self.presenter.palette_open() {
            self.state.palette_scroll = ScrollHandle::new();
        }
        self.keep_explorer_scroll();
        acted
    }

    /// [`Self::with_router`] as one input, committed once with its Notices drained.
    pub(super) fn send_with_router<T>(
        &mut self,
        window_size: WindowSize,
        cx: &mut Context<'_, Self>,
        act: impl FnOnce(&mut ShellPresenter, &mut RootRouter<'_, '_>) -> T,
    ) -> T {
        crate::input::send(cx, |cx| self.with_router(window_size, cx, act))
    }

    /// Delivers `input` to the Shell's Presenter as one input, with the root Router. Neither a
    /// tick nor a failure reads the Window Size.
    pub(super) fn send_shell(
        &mut self,
        input: ShellInput<WriteFailure>,
        cx: &mut Context<'_, Self>,
    ) {
        self.send_with_router(WindowSize::Large, cx, |presenter, router| {
            presenter.handle(input, router);
        });
    }

    /// Offers `chord` to the Shell's Presenter, which runs a command it keeps inside this one
    /// input, through the root Router. A key it passes on is no input of its own: nothing commits,
    /// and the next step that takes it is the input.
    pub(super) fn resolve_key(
        &mut self,
        chord: &str,
        window_size: WindowSize,
        cx: &mut Context<'_, Self>,
    ) -> ShellKey {
        crate::input::offer(cx, |cx| {
            let answer = self.with_router(window_size, cx, |presenter, router| {
                presenter.key(chord, router)
            });
            let taken = !matches!(answer, ShellKey::Passed { .. });
            (answer, taken)
        })
    }
}
