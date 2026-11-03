//! The Switcher (11b) and Manage budgets (11f) as one hosted Dialog Entity: each holds its
//! `lib-budgets` Presenter. Every Dialog command and typed key reaches it as a `lib-viper`
//! `DialogInput`, which it reads itself, every click becomes a Presenter event, and the Dialog
//! draws its view model (`switcher.rs`, `manage.rs`).
//!
//! A Presenter closes its Dialog through the host's [`HostRouter`] with an outcome; the Dialog hands it to the
//! page that opened it ([`Landed`]), which opens what follows (another Budget, the Budget form,
//! Manage budgets), then emits [`DialogClosed`] for the Shell to close the host.

use std::rc::Rc;

use gpui::{App, Context, Entity, EventEmitter, Pixels, Window, px};
use lib_budgets::BudgetsOutcome;
use lib_budgets::ManageAction;
use lib_budgets::presenter::{ManageEvent, ManagePresenter, SwitcherEvent, SwitcherPresenter};
use lib_shell::{DialogCommand, Hint};
use lib_viper::dialog::{DialogInput, DialogRouter};
use lib_viper::keymap::Keymap;

use crate::budgets::ports;
use crate::budgets::view::snapshot::ManageRowSnapshot;
use crate::dialog::OnClick;
use crate::ledger_store::LedgerStore;
use crate::shell::view::dialog_entity::{DialogClosed, HostRouter, HostedDialog};
use crate::shell::view::dialog_host::DialogReadback;

/// What a closed Switcher or Manage budgets hands the page: its outcome, or `None` on a cancel.
pub type Landed = Box<dyn Fn(Option<BudgetsOutcome>, &mut App)>;

/// The Switcher card's top below the window's: under the top bar, the page's top padding and
/// the title line.
pub const SWITCHER_TOP: Pixels = px(122.0);

pub(crate) enum List {
    Switcher(SwitcherPresenter),
    Manage(ManagePresenter),
}

pub struct ListDialog {
    pub(crate) list: List,
    ledger: Entity<LedgerStore>,
    landed: Landed,
    /// The Switcher card's left edge: the page's, which the Shell sets as it draws the rails.
    pub(in crate::budgets) left: Pixels,
}

impl EventEmitter<DialogClosed> for ListDialog {}

/// The host's Router, as a Switcher or Manage budgets Presenter closes through it.
type Router = HostRouter<BudgetsOutcome>;

impl ListDialog {
    pub fn switcher(
        presenter: SwitcherPresenter,
        ledger: &Entity<LedgerStore>,
        landed: Landed,
    ) -> Self {
        Self::new(List::Switcher(presenter), ledger, landed)
    }

    pub fn manage(
        presenter: ManagePresenter,
        ledger: &Entity<LedgerStore>,
        landed: Landed,
    ) -> Self {
        Self::new(List::Manage(presenter), ledger, landed)
    }

    fn new(list: List, ledger: &Entity<LedgerStore>, landed: Landed) -> Self {
        Self {
            list,
            ledger: ledger.clone(),
            landed,
            left: px(0.0),
        }
    }

    /// Sets where the Switcher card starts, as the Shell lays the rails out.
    pub fn set_left(&mut self, left: Pixels) {
        self.left = left;
    }

    pub(in crate::budgets) fn switcher_event(
        &mut self,
        event: SwitcherEvent,
        cx: &mut Context<'_, Self>,
    ) {
        self.run(cx, |list, router| {
            if let List::Switcher(presenter) = list {
                presenter.handle(event, router);
            }
        });
    }

    pub(in crate::budgets) fn manage_event(
        &mut self,
        event: ManageEvent,
        cx: &mut Context<'_, Self>,
    ) {
        self.run(cx, |list, router| {
            if let List::Manage(presenter) = list {
                presenter.handle(event, router);
            }
        });
    }

    /// Hands an event to the Presenter, brings the Ledger read copy in line with what it wrote,
    /// and, once the Presenter has closed the Dialog, hands its outcome to the page.
    fn run(
        &mut self,
        cx: &mut Context<'_, Self>,
        handle: impl FnOnce(&mut List, &mut dyn DialogRouter<BudgetsOutcome>),
    ) {
        let mut router = Router::default();
        handle(&mut self.list, &mut router);
        ports::refresh(&self.ledger, cx);
        if let Some(outcome) = router.closed() {
            (self.landed)(outcome, cx);
            cx.emit(DialogClosed);
        }
        cx.notify();
    }

    /// A click that sends the Switcher `event`, as one input.
    pub(super) fn on_switcher(cx: &Context<'_, Self>, event: SwitcherEvent) -> OnClick {
        let this = cx.weak_entity();
        Rc::new(move |_window: &mut Window, cx: &mut App| {
            crate::input::send(cx, |cx| {
                // A Dialog that has closed takes no click.
                let _ = this.update(cx, |dialog, cx| dialog.switcher_event(event, cx));
            });
        })
    }

    /// A click that sends Manage budgets `event`, as one input.
    pub(super) fn on_manage(cx: &Context<'_, Self>, event: ManageEvent) -> OnClick {
        let this = cx.weak_entity();
        Rc::new(move |_window: &mut Window, cx: &mut App| {
            crate::input::send(cx, |cx| {
                // A Dialog that has closed takes no click.
                let _ = this.update(cx, |dialog, cx| dialog.manage_event(event, cx));
            });
        })
    }

    /// The Budget names the Switcher lists, in order, and the highlighted row; `None` for Manage.
    pub fn switcher_rows(&self) -> Option<(Vec<String>, usize)> {
        let List::Switcher(presenter) = &self.list else {
            return None;
        };
        let view = presenter.view_model();
        let names = view.rows.into_iter().map(|row| row.name).collect();
        Some((names, view.selected.unwrap_or(0)))
    }

    /// Manage budgets' rows as `(name, is default, is archived)`, its cursor and its refusal;
    /// `None` for the Switcher.
    pub fn manage_rows(&self) -> Option<(Vec<ManageRowSnapshot>, usize, Option<String>)> {
        let List::Manage(presenter) = &self.list else {
            return None;
        };
        let view = presenter.view_model();
        let rows = view
            .rows
            .into_iter()
            .map(|row| (row.name, row.default.is_some(), row.archived))
            .collect();
        Some((rows, view.selected, view.error))
    }
}

impl HostedDialog for ListDialog {
    /// Every input is the Presenter's to read: the list's own letters, or the Switcher's search
    /// text while it has the keys.
    fn input(&mut self, input: DialogInput, cx: &mut Context<'_, Self>) {
        self.run(cx, |list, router| match list {
            List::Switcher(presenter) => presenter.handle(SwitcherEvent::Input(input), router),
            List::Manage(presenter) => presenter.handle(ManageEvent::Input(input), router),
        });
    }

    fn can_confirm(&self) -> bool {
        true
    }

    /// `submit` opens the highlighted Budget, as `enter` does.
    fn confirm(&mut self, cx: &mut Context<'_, Self>) {
        match self.list {
            List::Switcher(_) => self.switcher_event(SwitcherEvent::Choose, cx),
            List::Manage(_) => self.manage_event(ManageEvent::Act(ManageAction::Open), cx),
        }
    }

    fn cancel(&mut self, cx: &mut Context<'_, Self>) {
        match self.list {
            List::Switcher(_) => self.switcher_event(SwitcherEvent::Cancel, cx),
            List::Manage(_) => self.manage_event(ManageEvent::Cancel, cx),
        }
    }

    /// The Switcher's first `esc` while searching gives the keys back to the list.
    fn close_open_select(&mut self) -> bool {
        let List::Switcher(presenter) = &mut self.list else {
            return false;
        };
        if !presenter.searching() {
            return false;
        }
        presenter.handle(SwitcherEvent::LeaveSearch, &mut Router::default());
        true
    }

    fn hints(&self, keymap: &Keymap<DialogCommand>) -> Option<Vec<Hint>> {
        Some(match &self.list {
            List::Switcher(presenter) => presenter.legend(keymap),
            List::Manage(presenter) => presenter.legend(keymap),
        })
    }

    fn readback(&self, _cx: &App) -> DialogReadback {
        match &self.list {
            List::Switcher(presenter) => {
                let view = presenter.view_model();
                let selected = view
                    .selected
                    .and_then(|index| view.rows.get(index))
                    .map(|row| row.name.clone())
                    .unwrap_or_default();
                let names = view
                    .rows
                    .iter()
                    .map(|row| row.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                DialogReadback {
                    kind: "Switcher",
                    values: vec![("Budgets", names), ("Selected", selected)],
                }
            }
            List::Manage(presenter) => {
                let view = presenter.view_model();
                let selected = view
                    .rows
                    .get(view.selected)
                    .map(|row| row.name.clone())
                    .unwrap_or_default();
                let names = view
                    .rows
                    .iter()
                    .map(|row| row.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                DialogReadback {
                    kind: "Manage",
                    values: vec![
                        ("Budgets", names),
                        ("Selected", selected),
                        ("Error", view.error.unwrap_or_default()),
                    ],
                }
            }
        }
    }
}

impl gpui::Render for ListDialog {
    fn render(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> impl gpui::IntoElement {
        match &self.list {
            List::Switcher(presenter) => {
                super::switcher::render(&presenter.view_model(), self.left, cx)
            }
            List::Manage(presenter) => super::manage::render(&presenter.view_model(), cx),
        }
    }
}
