//! The Budget form (11c), Edit budget (9e), Stop budgeting (9g), Fill (9f) and the Category detail
//! (9d) as one hosted Dialog Entity: each holds its `lib-budgets` Presenter and the widget state
//! that is gpui's to keep (whether a select's list is open and its highlight), turns the keys the
//! Dialog host passes on and its clicks into the Presenter's events, and draws its view model
//! (`form_dialog/`).
//!
//! A Presenter closes its Dialog through the Router with an outcome; the Dialog hands it to the
//! page that opened it ([`Landed`]), which opens what follows (Edit budget from the detail, Stop
//! from Edit budget, the new Budget on show), then emits [`DialogClosed`] for the Shell to close
//! the host. A Toast the Presenter raises goes onto the bin's queue, which the input's `send`
//! drains.

mod budget;
mod detail;
mod fill;
mod limit;
mod stop;

use std::rc::Rc;

use gpui::{
    AnyElement, App, Context, EventEmitter, IntoElement, Render, SharedString, Window, div,
    prelude::*, px,
};
use lib_budgets::presenter::{
    BudgetFormEvent, BudgetFormPresenter, DetailEvent, DetailPresenter, FillEvent, FillPresenter,
    LimitFormEvent, LimitFormPresenter, StopEvent, StopPresenter,
};
use lib_shell::{DialogCommand, Hint};
use lib_viper::keymap::Keymap;

use super::dialog::{DialogRouter, Landed};
use crate::budgets::ports;
use crate::dialog;
use crate::form::select::SelectState;
use crate::ledger_store::LedgerStore;
use crate::shell::view::dialog_entity::{DialogClicks, DialogClosed, HostedDialog};
use crate::shell::view::dialog_host::{DialogKey, DialogOutcome, DialogReadback};
use crate::theme::color;

/// The open Dialog's clicks.
type Clicks = DialogClicks<FormDialog>;

/// The open Dialog: its Presenter and the widget state of its selects.
pub(crate) enum Form {
    /// New budget or its edit mode, with the Unit select's open state.
    Budget(BudgetFormPresenter, SelectState),
    /// Edit budget, with the Category and Starting selects' open state.
    Limit(LimitFormPresenter, SelectState, SelectState),
    /// Stop budgeting, with the From select's open state.
    Stop(StopPresenter, SelectState),
    Fill(FillPresenter),
    Detail(DetailPresenter),
}

pub struct FormDialog {
    ledger: gpui::Entity<LedgerStore>,
    view: Form,
    landed: Landed,
}

impl EventEmitter<DialogClosed> for FormDialog {}

impl FormDialog {
    pub(crate) fn new(view: Form, ledger: &gpui::Entity<LedgerStore>, landed: Landed) -> Self {
        Self {
            ledger: ledger.clone(),
            view,
            landed,
        }
    }

    /// Which Dialog is open, as the UI tests name it.
    pub fn kind(&self) -> &'static str {
        match self.view {
            Form::Budget(..) => "Budget",
            Form::Limit(..) => "EditLimit",
            Form::Stop(..) => "Stop",
            Form::Fill(_) => "Fill",
            Form::Detail(_) => "Detail",
        }
    }

    /// The open Edit budget form's Amount draft.
    pub fn limit_amount(&self) -> Option<String> {
        match &self.view {
            Form::Limit(presenter, ..) => Some(presenter.view_model().amount.text),
            _ => None,
        }
    }

    /// The open New budget form's Name draft.
    pub fn budget_name(&self) -> Option<String> {
        match &self.view {
            Form::Budget(presenter, _) => Some(presenter.view_model().name.text),
            _ => None,
        }
    }

    /// The Fill dialog's chosen source.
    pub fn fill_source(&self) -> Option<String> {
        match &self.view {
            Form::Fill(presenter) => Some(format!("{:?}", presenter.source())),
            _ => None,
        }
    }

    /// The Category the open Stop dialog is for.
    pub fn stop_category(&self) -> Option<lib_core::CategoryId> {
        match &self.view {
            Form::Stop(presenter, _) => Some(presenter.category()),
            _ => None,
        }
    }

    /// Runs `handle` on the open Presenter, brings the Ledger read copy in line with what it wrote
    /// and, once the Presenter has closed the Dialog, hands its outcome to the page.
    fn run(
        &mut self,
        cx: &mut Context<'_, Self>,
        handle: impl FnOnce(&mut Form, &mut DialogRouter),
    ) {
        let mut router = DialogRouter::default();
        handle(&mut self.view, &mut router);
        ports::refresh(&self.ledger, cx);
        if let Some(outcome) = router.closed {
            (self.landed)(outcome, cx);
            cx.emit(DialogClosed);
        }
        cx.notify();
    }

    /// The Detail's own events, which can close it.
    pub(in crate::budgets) fn detail_event(
        &mut self,
        event: DetailEvent,
        cx: &mut Context<'_, Self>,
    ) {
        self.run(cx, |view, router| {
            if let Form::Detail(presenter) = view {
                presenter.handle(event, router);
            }
        });
    }
}

impl HostedDialog for FormDialog {
    fn key(&mut self, key: DialogKey, cx: &mut Context<'_, Self>) -> DialogOutcome {
        match &mut self.view {
            Form::Budget(presenter, select) => budget::key(presenter, select, key),
            Form::Limit(presenter, category, starting) => {
                limit::key(presenter, category, starting, key)
            }
            Form::Stop(presenter, select) => stop::key(presenter, select, key),
            Form::Fill(presenter) => fill::key(presenter, key),
            Form::Detail(_) => match detail::key(key) {
                Some(Ok(event)) => {
                    self.detail_event(event, cx);
                    DialogOutcome::Handled
                }
                Some(Err(outcome)) => outcome,
                None => DialogOutcome::Ignored,
            },
        }
    }

    fn can_confirm(&self) -> bool {
        match &self.view {
            Form::Budget(presenter, _) => presenter.view_model().frame.can_confirm,
            Form::Limit(presenter, ..) => presenter.view_model().frame.can_confirm,
            Form::Stop(presenter, _) => presenter.view_model().frame.can_confirm,
            Form::Fill(presenter) => presenter.view_model().frame.can_confirm,
            Form::Detail(_) => true,
        }
    }

    /// Sends the Presenter its Confirm. A refusal stays on its view model and keeps the Dialog
    /// open; the detail's confirm is its hand-off to Transactions.
    fn confirm(&mut self, cx: &mut Context<'_, Self>) {
        self.run(cx, |view, router| match view {
            Form::Budget(presenter, _) => presenter.confirm(router),
            Form::Limit(presenter, ..) => presenter.confirm(router),
            Form::Stop(presenter, _) => presenter.confirm(router),
            Form::Fill(presenter) => presenter.confirm(router),
            Form::Detail(presenter) => presenter.confirm(router),
        });
    }

    /// A cancelled Dialog has written nothing.
    fn cancel(&mut self, cx: &mut Context<'_, Self>) {
        self.run(cx, |view, router| match view {
            Form::Budget(presenter, _) => presenter.handle(BudgetFormEvent::Cancel, router),
            Form::Limit(presenter, ..) => presenter.handle(LimitFormEvent::Cancel, router),
            Form::Stop(presenter, _) => presenter.handle(StopEvent::Cancel, router),
            Form::Fill(presenter) => presenter.handle(FillEvent::Cancel, router),
            Form::Detail(presenter) => presenter.handle(DetailEvent::Close, router),
        });
    }

    fn close_open_select(&mut self) -> bool {
        let close = |state: &mut SelectState| {
            let open = state.is_open();
            state.cancel();
            open
        };
        match &mut self.view {
            Form::Budget(_, select) | Form::Stop(_, select) => close(select),
            Form::Limit(_, category, starting) => close(category) | close(starting),
            Form::Fill(_) | Form::Detail(_) => false,
        }
    }

    fn hints(&self, keymap: &Keymap<DialogCommand>) -> Option<Vec<Hint>> {
        Some(match &self.view {
            Form::Budget(presenter, _) => presenter.legend(keymap),
            Form::Limit(presenter, ..) => presenter.legend(keymap),
            Form::Stop(presenter, _) => presenter.legend(keymap),
            Form::Fill(presenter) => presenter.legend(keymap),
            Form::Detail(presenter) => presenter.legend(keymap),
        })
    }

    /// The open Dialog's title, what a form shows and whether it would confirm, for a test.
    fn readback(&self, _cx: &App) -> DialogReadback {
        let mut values: Vec<(&'static str, String)> = Vec::new();
        match &self.view {
            Form::Budget(presenter, _) => {
                let view = presenter.view_model();
                values.push(("title", view.frame.title));
                values.push(("name", view.name.text));
                values.push(("can_confirm", view.frame.can_confirm.to_string()));
            }
            Form::Limit(presenter, ..) => {
                let view = presenter.view_model();
                values.push(("title", view.frame.title));
                values.push(("amount", view.amount.text));
                values.push(("can_confirm", view.frame.can_confirm.to_string()));
            }
            Form::Stop(presenter, _) => {
                let view = presenter.view_model();
                values.push(("title", view.frame.title));
                values.push(("can_confirm", view.frame.can_confirm.to_string()));
            }
            Form::Fill(presenter) => {
                let view = presenter.view_model();
                values.push(("title", view.frame.title));
                values.push(("can_confirm", view.frame.can_confirm.to_string()));
            }
            Form::Detail(presenter) => {
                if let Some(view) = presenter.view_model() {
                    values.push(("title", view.frame.title));
                }
            }
        }
        DialogReadback {
            kind: self.kind(),
            values,
        }
    }
}

impl Render for FormDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let actions = Clicks::new(cx);
        match &self.view {
            Form::Budget(presenter, select) => {
                budget::render(&presenter.view_model(), select, &actions, cx)
            }
            Form::Limit(presenter, category, starting) => {
                limit::render(&presenter.view_model(), category, starting, &actions, cx)
            }
            Form::Stop(presenter, select) => {
                stop::render(&presenter.view_model(), select, &actions, cx)
            }
            Form::Fill(presenter) => fill::render(&presenter.view_model(), &actions, cx),
            Form::Detail(presenter) => match presenter.view_model() {
                Some(view) => detail::render(&view, &actions, cx),
                // The Budget or Category went while it was open: nothing to draw.
                None => div().into_any_element(),
            },
        }
    }
}

/// A labelled `.seg`: the chosen cell filled dark, an accent border while focused. Each cell is
/// its text, whether it is the chosen one, and its click.
pub(super) fn segmented(
    id: &'static str,
    title: String,
    focused: bool,
    cells: Vec<(String, bool, dialog::OnClick)>,
    cx: &App,
) -> AnyElement {
    div()
        .child(crate::form::dialog_fields::label(title))
        .child(
            div()
                .flex()
                .border_1()
                .border_color(if focused {
                    color::accent(cx)
                } else {
                    color::border(cx)
                })
                .children(cells.into_iter().enumerate().map(
                    |(index, (text, chosen, on_click))| {
                        div()
                            .id(SharedString::from(format!("{id}-{index}")))
                            .debug_selector(move || format!("{id}-{index}"))
                            .cursor_pointer()
                            .flex_1()
                            .py(px(8.0))
                            .px(px(10.0))
                            .text_size(px(12.5))
                            .text_align(gpui::TextAlign::Center)
                            .font_weight(gpui::FontWeight::EXTRA_BOLD)
                            .when(chosen, |this| {
                                this.bg(color::foreground(cx))
                                    .text_color(color::background(cx))
                            })
                            .on_click(move |_event, window, cx| on_click(window, cx))
                            .child(text)
                    },
                )),
        )
        .into_any_element()
}

/// A line of help text under a field.
pub(super) fn help(text: String, cx: &App) -> AnyElement {
    div()
        .mt(px(6.0))
        .text_size(px(11.5))
        .text_color(color::muted(cx))
        .child(text)
        .into_any_element()
}

/// A click on a cell that picks `value` of `T`, as one input.
pub(super) fn picker<T: Copy + 'static>(
    choose: crate::shell::view::dialog_entity::OnChoose<T>,
    value: T,
) -> dialog::OnClick {
    Rc::new(move |window: &mut Window, cx: &mut App| choose(value, window, cx))
}
