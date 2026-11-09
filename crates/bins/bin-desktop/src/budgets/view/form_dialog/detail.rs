//! The Category detail (9d) on the shared `crate::dialog` frame at the handoff's 600px: four
//! stats, the Progress row's bar with its caption, the largest Transactions behind Spent, the
//! track record, and a footer that hands off to Transactions already filtered.

use gpui::{AnyElement, App, div, prelude::*, px};
use lib_budgets::presenter::{DetailEvent, DetailPresenter, DetailViewModel};
use lib_shell::Hint;

use crate::budgets::view::progress;
use crate::dialog;
use crate::shell::view::dialog_entity::{DialogView, ModuleDialogClicks, framed};
use crate::theme::color;

/// The dialog's width: the handoff's 9d.
const WIDTH: gpui::Pixels = px(600.0);

/// The open Dialog's clicks.
type Clicks = ModuleDialogClicks<DetailPresenter>;

/// The `lib-budgets` Category detail Presenter's [`DialogView`], hosted in a `ModuleDialog`
/// (ADR-0056). Its view model is `None` once the Budget or Category went while it was open, which
/// draws nothing.
impl DialogView for DetailPresenter {
    type Widgets = ();

    fn kind(_view: &Option<DetailViewModel>) -> &'static str {
        "Detail"
    }

    fn render(
        view: &Option<DetailViewModel>,
        _widgets: &mut (),
        clicks: &Clicks,
        cx: &App,
    ) -> AnyElement {
        match view {
            Some(view) => render(view, clicks, cx),
            None => div().into_any_element(),
        }
    }

    fn readback(view: &Option<DetailViewModel>) -> Vec<(&'static str, String)> {
        view.as_ref()
            .map(|view| framed(&view.frame, Vec::new()))
            .unwrap_or_default()
    }

    fn hints(view: &Option<DetailViewModel>) -> Vec<Hint> {
        view.as_ref()
            .map(|view| view.hints.clone())
            .unwrap_or_default()
    }
}

fn stat(label: &str, value: String, negative: bool, cx: &App) -> impl IntoElement {
    div()
        .flex_1()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .child(
            div()
                .text_size(px(10.0))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_color(color::muted(cx))
                .child(lib_locale::format::upper(label)),
        )
        .child(
            div()
                .text_size(px(20.0))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_color(if negative {
                    color::negative_text(cx)
                } else {
                    color::foreground(cx)
                })
                .child(value),
        )
}

fn render(view: &DetailViewModel, actions: &Clicks, cx: &App) -> AnyElement {
    let stats = div().flex().gap(px(16.0)).children(
        view.stats
            .iter()
            .map(|each| stat(&each.label, each.value.clone(), each.negative, cx)),
    );
    let bar = div()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(progress::bar(view.bar, view.elapsed, false, cx))
        .child(
            div()
                .text_size(px(11.5))
                .text_color(color::muted(cx))
                .child(view.caption.clone()),
        );

    let list = div()
        .flex()
        .flex_col()
        .child(
            div()
                .pb(px(6.0))
                .border_b(px(1.0))
                .border_color(color::hairline(cx))
                .text_size(px(10.0))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_color(color::muted(cx))
                .child(lib_locale::format::upper(&view.list_title)),
        )
        .children(view.lines.iter().enumerate().map(|(index, line)| {
            let active = index == view.selected;
            div()
                .flex()
                .gap(px(12.0))
                .py(px(6.0))
                .border_b(px(1.0))
                .border_color(color::hairline(cx))
                .text_size(px(12.5))
                .when(active, |this| {
                    this.bg(color::selection_background(cx))
                        .text_color(color::selection_text(cx))
                })
                .child(div().w(px(64.0)).flex_none().child(line.date.clone()))
                .child(div().flex_1().child(line.payee.clone()))
                .child(div().w(px(150.0)).flex_none().child(line.account.clone()))
                .child(
                    div()
                        .w(px(90.0))
                        .flex_none()
                        .text_right()
                        .child(line.amount.clone()),
                )
        }))
        .children(view.more.clone().map(|row| {
            div()
                .py(px(6.0))
                .text_size(px(12.0))
                .text_color(color::muted(cx))
                .child(row)
        }))
        .child(
            div()
                .pt(px(6.0))
                .text_size(px(12.0))
                .text_color(color::muted(cx))
                .child(view.bills.clone()),
        );

    let track = div()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .text_size(px(12.5))
        .child(view.track.clone())
        .children(
            view.rollover_note
                .clone()
                .map(|note| div().text_color(color::muted(cx)).child(note)),
        );

    let on_transactions = actions.send(DetailEvent::Transactions);
    let on_close = actions.send(DetailEvent::Close);
    let mut buttons = vec![
        div()
            .id("budgets-detail-transactions")
            .debug_selector(|| "budgets-detail-transactions".to_string())
            .cursor_pointer()
            .py(px(8.0))
            .px(px(16.0))
            .mr_auto()
            .font_weight(gpui::FontWeight::EXTRA_BOLD)
            .text_color(color::foreground(cx))
            .on_click(move |_event, window, cx| on_transactions(window, cx))
            .child(view.open_transactions.clone())
            .into_any_element(),
    ];
    // With Edit budget beside it, Close steps back to the outlined secondary button.
    if view.can_edit {
        let on_edit = actions.send(DetailEvent::Edit);
        buttons.push(
            div()
                .id("budgets-detail-close")
                .debug_selector(|| "budgets-detail-close".to_string())
                .cursor_pointer()
                .py(px(8.0))
                .px(px(16.0))
                .border_1()
                .border_color(color::border(cx))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .on_click(move |_event, window, cx| on_close(window, cx))
                .child(view.close.clone())
                .into_any_element(),
        );
        buttons.push(
            dialog::confirm_button(
                "budgets-detail-edit",
                view.edit.clone(),
                true,
                false,
                on_edit,
                cx,
            )
            .into_any_element(),
        );
    } else {
        buttons.push(
            dialog::confirm_button(
                "budgets-detail-close",
                view.close.clone(),
                true,
                false,
                on_close,
                cx,
            )
            .into_any_element(),
        );
    }

    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(view.frame.title.clone(), false, cx))
        .child(dialog::body(vec![
            stats.into_any_element(),
            bar.into_any_element(),
            list.into_any_element(),
            track.into_any_element(),
        ]))
        .child(dialog::action_row(buttons, cx));
    dialog::overlay(WIDTH, false, card, cx)
}
