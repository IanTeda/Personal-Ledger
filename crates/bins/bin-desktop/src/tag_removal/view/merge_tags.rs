//! The Merge tags Dialog (`docs/ux/desktop-mockups/19-tags/README.md`'s 7e) on the shared
//! `crate::dialog` frame at the handoff's 460px: two selects with a `→` between them (the source
//! keeps its accent border, marking the Tag that goes), the red-bordered irreversible callout with
//! live counts and names, and a primary button whose label follows the target.
//!
//! The Presenter reads the keys: the selects take the arrows and `Space`, `Enter` commits an open
//! list, else merges, and typing goes nowhere, rather than reaching the Shell.
//!
//! The `lib-tag-removal` Presenter's [`DialogView`], hosted in a `ModuleDialog` (ADR-0056). The
//! selects' scroll windows are the View's widget state ([`ListWindows`]), which the Presenter
//! never sees; their open lists and highlights are the Presenter's.

use gpui::{AnyElement, App, div, prelude::*, px};
use lib_shell::Hint;
use lib_tag_removal::presenter::{
    MergeField, MergeTagsEvent, MergeTagsPresenter, MergeTagsViewModel,
};
use lib_viper::view::SelectView;

use crate::form::colour_field::error_line;
use crate::form::select::SelectState;
use crate::form::select_field::{self, SelectFieldProps};
use crate::form::viper::{drawn, labels};
use crate::shell::view::dialog_entity::{DialogView, ModuleDialogClicks, framed};
use crate::{dialog, theme::color};

/// The Dialog's width: the handoff's 7e.
const WIDTH: gpui::Pixels = px(460.0);

/// The open Dialog's clicks.
type Clicks = ModuleDialogClicks<MergeTagsPresenter>;

/// The top row of the source's and of the target's list window.
#[derive(Debug, Default)]
pub struct ListWindows {
    source: usize,
    target: usize,
}

impl DialogView for MergeTagsPresenter {
    type Widgets = ListWindows;

    fn kind(_view: &MergeTagsViewModel) -> &'static str {
        "MergeTags"
    }

    fn render(
        view: &MergeTagsViewModel,
        windows: &mut ListWindows,
        clicks: &Clicks,
        cx: &App,
    ) -> AnyElement {
        render(view, windows, clicks, cx)
    }

    /// The chosen Tags' names and any refusal, between the title and whether it would confirm.
    fn readback(view: &MergeTagsViewModel) -> Vec<(&'static str, String)> {
        framed(&view.frame, readback(view))
    }

    fn hints(view: &MergeTagsViewModel) -> Vec<Hint> {
        view.hints.clone()
    }
}

/// A click on row `index` of `field`'s open list focuses it and picks that row's Tag, read from
/// `select`'s options as drawn.
fn click_option(
    select: &SelectView<lib_core::TagId>,
    field: MergeField,
    actions: &Clicks,
) -> select_field::OnOptionClick {
    let tags: Vec<lib_core::TagId> = select.options.iter().map(|choice| choice.value).collect();
    actions.option(move |index| {
        let mut events = vec![MergeTagsEvent::Focus(field)];
        events.extend(tags.get(index).map(|tag| match field {
            MergeField::Source => MergeTagsEvent::Source(*tag),
            MergeField::Target => MergeTagsEvent::Target(*tag),
        }));
        events
    })
}

fn render(
    view: &MergeTagsViewModel,
    windows: &mut ListWindows,
    actions: &Clicks,
    cx: &App,
) -> AnyElement {
    let (source, target) = (
        drawn(&view.source, view.source_list, &mut windows.source),
        drawn(&view.target, view.target_list, &mut windows.target),
    );
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
                state,
                focused: select_view.focused,
                accent,
                read_only: None,
                // A click on a closed field focuses it and toggles its list, closing the other.
                on_field_click: actions.send(MergeTagsEvent::Toggle(field)),
                on_option_click: click_option(select_view, field, actions),
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
            &source,
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
            &target,
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
fn readback(view: &MergeTagsViewModel) -> Vec<(&'static str, String)> {
    [
        ("source", view.source_name.clone()),
        ("target", view.target_name.clone()),
    ]
    .into_iter()
    .filter_map(|(label, name)| Some((label, name?)))
    .chain([("refusal", view.refusal.clone().unwrap_or_default())])
    .collect()
}
