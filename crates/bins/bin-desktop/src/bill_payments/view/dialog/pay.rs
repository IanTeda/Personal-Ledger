//! The Pay Dialog (`docs/ux/desktop-mockups/12-bills/README.md`'s 8d) on the shared
//! `crate::dialog` frame, an Entity in the Dialog host (`shell::view::dialog_entity`): the segmented switch under the title bar,
//! then either the Match list or the Pay it directly fields. Every key and click becomes a
//! `lib-bill-payments` Presenter event, and this draws the view model and nothing else.
//!
//! The body scrolls past [`BODY_MAX_HEIGHT`], so the switch and the buttons stay on screen however
//! many candidates the Match list holds, and `j`/`k` scroll the chosen row into view.
//!
//! Keys: `←`/`→` switch panel. On Match `j`/`k` (and `select_next`/`select_previous`) choose a
//! row, and `select` on "None of these" switches to Pay it directly instead of confirming. On Pay
//! it directly typing edits the focused field and `next_field` moves to the other. `select`
//! confirms once the panel would.
//!
//! Confirm and Cancel go to the Presenter, which closes the Dialog through its Router; a confirmed
//! outcome is handed to the page, and the host closes on [`DialogClosed`].

use gpui::{
    AnyElement, App, Context, Entity, EventEmitter, Render, ScrollHandle, SharedString, Window,
    div, prelude::*, px,
};
use lib_bill_payments::PayOutcome;
use lib_bill_payments::presenter::{
    MatchChoice, PayEvent, PayField, PayMode, PayPresenter, PayViewModel,
};

use crate::bill_payments::router::DialogRouter;
use crate::bills::DesktopPorts;
use crate::bills::ports::copy_reads;
use crate::dialog;
use crate::form::dialog_fields::{label, text_field, two_up};
use crate::form::viper::typed;
use crate::ledger_store::LedgerStore;
use crate::shell::view::dialog_entity::{DialogClicks, DialogClosed, HostedDialog};
use crate::shell::view::dialog_host::{DialogKey, DialogOutcome, DialogReadback};
use crate::shell::view::statusline::Hint;
use crate::theme::color;
use lib_shell::DialogCommand;
use lib_viper::keymap::{BACK, Keymap, SELECT};

/// The Dialog's width: 8d's.
const WIDTH: gpui::Pixels = px(460.0);
/// The body's scroll cap, the Plan form's, which keeps the switch and the buttons inside a
/// 1080px window.
const BODY_MAX_HEIGHT: gpui::Pixels = px(480.0);

/// The open Pay Dialog's clicks.
type Clicks = DialogClicks<PayDialog>;

/// What a landed Pay hands the page that opened it.
pub(crate) type Landed = Box<dyn Fn(PayOutcome, &mut App)>;

/// The open Pay Dialog: its Presenter, the Bills ports its confirm copies the reads into, and the
/// Ledger read copy it reads from and brings in line.
pub struct PayDialog {
    presenter: PayPresenter,
    ports: DesktopPorts,
    ledger: Entity<LedgerStore>,
    landed: Landed,
    /// The body's scroll, which `j`/`k` move to keep the chosen Match row in view.
    scroll: ScrollHandle,
}

impl PayDialog {
    pub(crate) fn new(
        presenter: PayPresenter,
        ports: &DesktopPorts,
        ledger: &Entity<LedgerStore>,
        landed: Landed,
    ) -> Self {
        Self {
            presenter,
            ports: ports.clone(),
            ledger: ledger.clone(),
            landed,
            scroll: ScrollHandle::new(),
        }
    }

    fn view(&self) -> PayViewModel {
        self.presenter.view_model()
    }

    /// Sends `event` to the Presenter, returning what its Router was asked.
    fn send(&mut self, event: PayEvent) -> DialogRouter {
        let mut router = DialogRouter::default();
        self.presenter.handle(event, &mut router);
        router
    }

    /// A click's event, as one input.
    fn on_click(actions: &Clicks, event: PayEvent) -> dialog::OnClick {
        actions.click(move |dialog| {
            dialog.send(event.clone());
        })
    }

    /// Hands a landed payment to the page and tells the host, once the Presenter has closed the
    /// Dialog.
    fn closed(&self, router: DialogRouter, cx: &mut Context<'_, Self>) {
        let Some(outcome) = router.closed else {
            return;
        };
        if let Some(outcome) = outcome {
            (self.landed)(outcome, cx);
        }
        cx.emit(DialogClosed);
    }
}

impl EventEmitter<DialogClosed> for PayDialog {}

impl HostedDialog for PayDialog {
    /// Every key is the Dialog's own: the Presenter holds every field.
    fn key(&mut self, key: DialogKey, _cx: &mut Context<'_, Self>) -> DialogOutcome {
        let view = self.view();
        let event = match (view.mode, key) {
            (_, DialogKey::Left | DialogKey::Right) => PayEvent::ToggleMode,
            // Enter confirms through the host while the panel would; otherwise the Presenter
            // decides, which on "None of these" switches panel.
            (_, DialogKey::Enter) if view.frame.can_confirm => return DialogOutcome::Confirm,
            (_, DialogKey::Enter) => PayEvent::Confirm,
            (PayMode::Match, DialogKey::Char('j') | DialogKey::Down) => PayEvent::NextChoice,
            (PayMode::Match, DialogKey::Char('k') | DialogKey::Up) => PayEvent::PrevChoice,
            (PayMode::Direct, DialogKey::Tab | DialogKey::BackTab) => PayEvent::NextField,
            (PayMode::Direct, DialogKey::Char(_) | DialogKey::Backspace) => {
                let (field, event): (_, fn(String) -> PayEvent) = if view.amount.focused {
                    (&view.amount, PayEvent::Amount)
                } else {
                    (&view.date, PayEvent::Date)
                };
                match typed(&field.text, key) {
                    Some(text) => event(text),
                    None => return DialogOutcome::Handled,
                }
            }
            // Match takes no typing.
            (PayMode::Match, DialogKey::Char(_) | DialogKey::Backspace) => {
                return DialogOutcome::Handled;
            }
            _ => return DialogOutcome::Ignored,
        };
        let chooses = matches!(event, PayEvent::NextChoice | PayEvent::PrevChoice);
        self.send(event);
        if chooses && let Some(row) = chosen_row(&self.view()) {
            self.scroll.scroll_to_item(row);
        }
        DialogOutcome::Handled
    }

    fn can_confirm(&self) -> bool {
        self.view().frame.can_confirm
    }

    /// Sends Confirm to the Presenter, which pays or matches, then brings the Ledger's read copies
    /// in line. A refusal stays on the view model and keeps the Dialog open.
    fn confirm(&mut self, cx: &mut Context<'_, Self>) {
        copy_reads(&self.ports, self.ledger.read(cx).ledger());
        let router = self.send(PayEvent::Confirm);
        crate::bill_payments::refresh(&self.ports, &self.ledger, cx);
        self.closed(router, cx);
    }

    /// A cancelled Pay has written nothing.
    fn cancel(&mut self, cx: &mut Context<'_, Self>) {
        let router = self.send(PayEvent::Cancel);
        self.closed(router, cx);
    }

    /// Pay has no select.
    fn close_open_select(&mut self) -> bool {
        false
    }

    fn hints(&self, keymap: &Keymap<DialogCommand>) -> Option<Vec<Hint>> {
        let key = |command| keymap.chord(command).unwrap_or_default().to_string();
        Some(vec![
            (key(BACK), crate::msg::desktop_hint_cancel()),
            (key(SELECT), crate::msg::desktop_hint_confirm()),
            ("j/k".to_string(), crate::msg::desktop_hint_choose()),
            (
                "\u{2190}/\u{2192}".to_string(),
                crate::msg::desktop_hint_switch_panel(),
            ),
        ])
    }

    /// The open Dialog's panel, what it shows and whether it would confirm, for a test.
    fn readback(&self, _cx: &App) -> DialogReadback {
        let view = self.view();
        let choice = match view.choice {
            Some(MatchChoice::Candidate(index)) => index.to_string(),
            Some(MatchChoice::NoneOfThese) => "none".to_string(),
            None => String::new(),
        };
        DialogReadback {
            kind: "Pay",
            values: vec![
                ("title", view.frame.title),
                ("mode", format!("{:?}", view.mode)),
                ("candidates", view.candidates.len().to_string()),
                ("choice", choice),
                ("amount", view.amount.text),
                ("date", view.date.text),
                ("refusal", view.refusal.unwrap_or_default()),
                ("can_confirm", view.frame.can_confirm.to_string()),
            ],
        }
    }
}

impl Render for PayDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        render(&self.view(), &self.scroll, &Clicks::new(cx), cx)
    }
}

fn render(view: &PayViewModel, scroll: &ScrollHandle, actions: &Clicks, cx: &App) -> AnyElement {
    let mut fields = match view.mode {
        PayMode::Match => match_panel(view, actions, cx),
        PayMode::Direct => direct_panel(view, actions, cx),
    };
    if let Some(refusal) = &view.refusal {
        fields.push(error_line(refusal.clone(), cx));
    }
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(view.frame.title.clone(), false, cx))
        .child(
            div()
                .px(px(24.0))
                .pt(px(16.0))
                .child(switch(view, actions, cx)),
        )
        .child(
            dialog::body(fields)
                .id("pay-bill-body")
                .debug_selector(|| "pay-bill-body".to_string())
                .max_h(BODY_MAX_HEIGHT)
                .overflow_y_scroll()
                .track_scroll(scroll),
        )
        .child(dialog::action_row(
            [
                dialog::cancel_button("pay-bill-cancel", actions.cancel.clone(), cx)
                    .into_any_element(),
                dialog::confirm_button(
                    "pay-bill-confirm",
                    view.frame.submit.clone(),
                    view.frame.can_confirm,
                    view.frame.destructive,
                    actions.confirm.clone(),
                    cx,
                )
                .into_any_element(),
            ],
            cx,
        ));
    dialog::overlay(WIDTH, false, card, cx)
}

/// The two-cell switch: the chosen cell filled dark (8d's `.seg-opt`), not the accent the Settings
/// segmented control uses.
fn switch(view: &PayViewModel, actions: &Clicks, cx: &App) -> AnyElement {
    div()
        .flex()
        .border_1()
        .border_color(color::border(cx))
        .children(view.modes.iter().enumerate().map(|(index, mode)| {
            let on_click = PayDialog::on_click(actions, PayEvent::Mode(mode.value));
            div()
                .id(SharedString::from(format!("pay-bill-mode-{index}")))
                .debug_selector(move || format!("pay-bill-mode-{index}"))
                .cursor_pointer()
                .flex_1()
                .py(px(8.0))
                .px(px(12.0))
                .text_size(px(13.0))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .when(mode.value == view.mode, |this| {
                    this.bg(color::foreground(cx))
                        .text_color(color::background(cx))
                })
                .on_click(move |_event, window, cx| on_click(window, cx))
                .child(mode.label.clone())
        }))
        .into_any_element()
}

/// The chosen Match row's index among the body's children, in [`match_panel`]'s order: the
/// heading, the no-candidates line if any, the candidates, then "None of these".
fn chosen_row(view: &PayViewModel) -> Option<usize> {
    let first = 1 + usize::from(view.no_candidates.is_some());
    match view.choice? {
        MatchChoice::Candidate(index) => Some(first + index),
        MatchChoice::NoneOfThese => Some(first + view.candidates.len()),
    }
}

fn match_panel(view: &PayViewModel, actions: &Clicks, cx: &App) -> Vec<AnyElement> {
    let mut rows = vec![
        div()
            .text_size(px(11.0))
            .font_weight(gpui::FontWeight::EXTRA_BOLD)
            .text_color(color::faint_text(cx))
            .child(view.candidates_heading.clone())
            .into_any_element(),
    ];
    if let Some(none) = &view.no_candidates {
        rows.push(
            div()
                .text_size(px(13.0))
                .text_color(color::muted(cx))
                .child(none.clone())
                .into_any_element(),
        );
    }
    for (index, candidate) in view.candidates.iter().enumerate() {
        let choice = MatchChoice::Candidate(index);
        rows.push(radio_row(
            choice,
            view.choice == Some(choice),
            candidate.summary.clone(),
            Some(candidate.account.clone()),
            actions,
            cx,
        ));
    }
    rows.push(radio_row(
        MatchChoice::NoneOfThese,
        view.choice == Some(MatchChoice::NoneOfThese),
        view.none_of_these.clone(),
        None,
        actions,
        cx,
    ));
    rows.push(dialog::info_panel(view.match_notice.clone(), cx).into_any_element());
    rows
}

/// A bordered radio row; the chosen one gets the accent border and tint.
fn radio_row(
    choice: MatchChoice,
    selected: bool,
    text: String,
    trailing: Option<String>,
    actions: &Clicks,
    cx: &App,
) -> AnyElement {
    let on_click = PayDialog::on_click(actions, PayEvent::Choose(choice));
    let id = match choice {
        MatchChoice::Candidate(index) => format!("pay-bill-candidate-{index}"),
        MatchChoice::NoneOfThese => "pay-bill-none-of-these".to_string(),
    };
    div()
        .debug_selector({
            let id = id.clone();
            move || id.clone()
        })
        .id(SharedString::from(id))
        .cursor_pointer()
        .flex()
        .items_center()
        .gap(px(10.0))
        .py(px(10.0))
        .px(px(12.0))
        .border_1()
        .border_color(if selected {
            color::accent(cx)
        } else {
            color::border(cx)
        })
        .when(selected, |this| this.bg(color::accent_tint(cx)))
        .text_size(px(13.0))
        .on_click(move |_event, window, cx| on_click(window, cx))
        .child(
            div()
                .size(px(14.0))
                .flex_none()
                .rounded_full()
                .border_1()
                .flex()
                .items_center()
                .justify_center()
                .border_color(if selected {
                    color::accent(cx)
                } else {
                    color::muted(cx)
                })
                .when(selected, |this| {
                    this.child(div().size(px(6.0)).rounded_full().bg(color::accent(cx)))
                }),
        )
        .child(div().flex_1().min_w(px(0.0)).child(text))
        .children(trailing.map(|account| {
            div()
                .text_size(px(11.5))
                .text_color(color::muted(cx))
                .child(account)
        }))
        .into_any_element()
}

fn direct_panel(view: &PayViewModel, actions: &Clicks, cx: &App) -> Vec<AnyElement> {
    let with_error = |field: AnyElement, error: Option<String>| -> AnyElement {
        div()
            .flex_1()
            .min_w(px(0.0))
            .child(field)
            .children(error.map(|text| error_line(text, cx)))
            .into_any_element()
    };
    vec![
        two_up([
            with_error(
                text_field(
                    "pay-bill-amount",
                    label(view.amount.label.clone()),
                    &view.amount.text,
                    &view.amount.placeholder,
                    view.amount.focused,
                    PayDialog::on_click(actions, PayEvent::Focus(PayField::Amount)),
                    cx,
                ),
                view.amount_error.clone(),
            ),
            with_error(
                text_field(
                    "pay-bill-date",
                    label(view.date.label.clone()),
                    &view.date.text,
                    &view.date.placeholder,
                    view.date.focused,
                    PayDialog::on_click(actions, PayEvent::Focus(PayField::Date)),
                    cx,
                ),
                view.date_error.clone(),
            ),
        ]),
        div()
            .flex()
            .flex_wrap()
            .gap(px(6.0))
            .children(view.chips.iter().map(|text| {
                div()
                    .py(px(3.0))
                    .px(px(8.0))
                    .border_1()
                    .border_color(color::border(cx))
                    .text_size(px(11.5))
                    .text_color(color::muted(cx))
                    .child(text.clone())
            }))
            .into_any_element(),
        dialog::info_panel(view.direct_notice.clone(), cx).into_any_element(),
    ]
}

fn error_line(text: String, cx: &App) -> AnyElement {
    div()
        .mt(px(6.0))
        .text_size(px(11.5))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_color(color::accent_text(cx))
        .child(text)
        .into_any_element()
}
