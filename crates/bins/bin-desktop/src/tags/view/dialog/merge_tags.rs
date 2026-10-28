//! The Merge tags Dialog (`docs/ux/desktop-mockups/19-tags/README.md`'s 7e) on the shared
//! `crate::dialog` frame at the handoff's 460px: two selects with a `→` between them (the source
//! keeps its accent border, marking the Tag that goes), the red-bordered irreversible callout with
//! live counts and names, and a primary button whose label follows the target.
//!
//! The selects take the arrows and `Space`; `Enter` commits an open list, else merges. Typing goes
//! nowhere, rather than reaching the Shell.

use gpui::{AnyElement, App, div, prelude::*, px};
use lib_tags::presenter::{MergeField, MergeTagsEvent, MergeTagsPresenter, MergeTagsViewModel};
use lib_viper::view::SelectView;

use super::{Clicks, TagsDialog, View};
use crate::form::colour_field::error_line;
use crate::form::select::SelectState;
use crate::form::select_field::{self, SelectFieldProps};
use crate::form::viper::{choose_option, enter, labels, select_key, synced, toggle_select};
use crate::{
    dialog,
    shell::view::dialog_host::{DialogKey, DialogOutcome},
    theme::color,
};

/// The Dialog's width: the handoff's 7e.
const WIDTH: gpui::Pixels = px(460.0);

fn handle(presenter: &mut MergeTagsPresenter, event: MergeTagsEvent) {
    let mut router = crate::tags::router::DialogRouter::default();
    presenter.handle(event, &mut router);
}

/// Sends the value a select key chose to the select's own field.
fn choose(presenter: &mut MergeTagsPresenter, field: MergeField, id: lib_core::TagId) {
    let event = match field {
        MergeField::Source => MergeTagsEvent::Source(id),
        MergeField::Target => MergeTagsEvent::Target(id),
    };
    handle(presenter, event);
}

pub(super) fn key(
    presenter: &mut MergeTagsPresenter,
    source: &mut SelectState,
    target: &mut SelectState,
    key: DialogKey,
) -> DialogOutcome {
    let view = presenter.view_model();
    let (field, select, select_view) = match view.focused {
        MergeField::Source => (MergeField::Source, source, &view.source),
        MergeField::Target => (MergeField::Target, target, &view.target),
    };
    match key {
        DialogKey::Tab | DialogKey::BackTab => {
            // Leaving a select commits an open list's highlight, as a click elsewhere would.
            if select.is_open() {
                let (_, value) = select_key(select, select_view, DialogKey::Enter);
                if let Some(id) = value {
                    choose(presenter, field, id);
                }
            }
            handle(presenter, MergeTagsEvent::NextField);
            DialogOutcome::Handled
        }
        // Enter commits an open list, else merges.
        DialogKey::Enter if !select.is_open() => enter(view.frame.can_confirm),
        DialogKey::Enter | DialogKey::Char(' ') | DialogKey::Up | DialogKey::Down => {
            let (_, value) = select_key(select, select_view, key);
            if let Some(id) = value {
                choose(presenter, field, id);
            }
            DialogOutcome::Handled
        }
        DialogKey::Char(_) | DialogKey::Backspace => DialogOutcome::Handled,
        _ => DialogOutcome::Ignored,
    }
}

fn merge(
    dialog: &mut TagsDialog,
) -> Option<(&mut MergeTagsPresenter, &mut SelectState, &mut SelectState)> {
    match &mut dialog.view {
        View::Merge(presenter, source, target) => Some((presenter, source, target)),
        View::Form(_) | View::Remove(_) => None,
    }
}

/// A click on a select's closed field focuses it and toggles its list, closing the other.
fn click_field(dialog: &mut TagsDialog, field: MergeField) {
    if let Some((presenter, source, target)) = merge(dialog) {
        handle(presenter, MergeTagsEvent::Focus(field));
        let view = presenter.view_model();
        match field {
            MergeField::Source => {
                target.cancel();
                toggle_select(source, &view.source);
            }
            MergeField::Target => {
                source.cancel();
                toggle_select(target, &view.target);
            }
        }
    }
}

fn click_option(dialog: &mut TagsDialog, field: MergeField, index: usize) {
    if let Some((presenter, source, target)) = merge(dialog) {
        handle(presenter, MergeTagsEvent::Focus(field));
        let view = presenter.view_model();
        let id = match field {
            MergeField::Source => choose_option(source, &view.source, index),
            MergeField::Target => choose_option(target, &view.target, index),
        };
        if let Some(id) = id {
            choose(presenter, field, id);
        }
    }
}

pub(super) fn render(
    view: &MergeTagsViewModel,
    source: &SelectState,
    target: &SelectState,
    actions: &Clicks,
    cx: &App,
) -> AnyElement {
    let select = |id: &'static str,
                  select_view: &SelectView<lib_core::TagId>,
                  state: &SelectState,
                  field: MergeField,
                  accent: bool| {
        let options = labels(select_view);
        select_field::render(
            SelectFieldProps {
                id: id.into(),
                label: select_view.label.clone().into(),
                options: &options,
                state: &synced(state, select_view),
                focused: select_view.focused,
                accent,
                read_only: None,
                on_field_click: actions.click(move |dialog| click_field(dialog, field)),
                on_option_click: actions.option(move |dialog, index| {
                    click_option(dialog, field, index);
                }),
            },
            cx,
        )
    };
    let selects = div()
        .flex()
        .items_start()
        .gap(px(10.0))
        .child(select(
            "merge-tags-source",
            &view.source,
            source,
            MergeField::Source,
            true,
        ))
        .child(
            div()
                .flex_none()
                // Level with the closed fields, below their labels.
                .mt(px(26.0))
                .text_size(px(16.0))
                .text_color(color::faint_text(cx))
                .child("\u{2192}"),
        )
        .child(select(
            "merge-tags-target",
            &view.target,
            target,
            MergeField::Target,
            false,
        ))
        .into_any_element();
    let mut fields = vec![selects, callout(view, cx)];
    fields.extend(view.refusal.clone().map(|text| error_line(text, cx)));
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(view.frame.title.clone(), false, cx))
        .child(dialog::body(fields))
        .child(dialog::action_row(
            [
                dialog::cancel_button("merge-tags-cancel", actions.cancel.clone(), cx)
                    .into_any_element(),
                dialog::confirm_button(
                    "merge-tags-confirm",
                    view.frame.submit.clone(),
                    view.frame.can_confirm,
                    false,
                    actions.confirm.clone(),
                    cx,
                )
                .into_any_element(),
            ],
            cx,
        ));
    dialog::overlay(WIDTH, false, card, cx)
}

/// The irreversible callout: `dialog::info_panel`'s red-bordered style, with the count in bold.
/// Until both Tags are chosen it asks for them instead.
fn callout(view: &MergeTagsViewModel, cx: &App) -> AnyElement {
    let panel = div()
        .p(px(10.0))
        .bg(color::chrome(cx))
        .border_l(px(2.0))
        .border_color(color::accent(cx))
        .text_size(px(11.5))
        .text_color(color::muted(cx));
    if view.choosing {
        return panel
            .child(
                view.callout
                    .iter()
                    .map(|segment| segment.text.as_str())
                    .collect::<String>(),
            )
            .into_any_element();
    }
    panel
        .child(dialog::rich_text(
            view.callout.clone(),
            Some(color::foreground(cx).into()),
        ))
        .into_any_element()
}

/// The names of the Tags chosen, for a test.
pub(super) fn readback(view: &MergeTagsViewModel) -> Vec<(&'static str, String)> {
    [
        ("source", view.source_name.clone()),
        ("target", view.target_name.clone()),
    ]
    .into_iter()
    .filter_map(|(label, name)| Some((label, name?)))
    .chain([("refusal", view.refusal.clone().unwrap_or_default())])
    .collect()
}
