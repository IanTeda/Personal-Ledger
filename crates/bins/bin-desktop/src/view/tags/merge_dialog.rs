//! Renders the **Merge tags** dialog (`docs/ux/desktop/Tags/README.md`'s 7e) on the shared
//! `crate::dialog` chrome at the handoff's 460px: two selects with a `→` between them (the source
//! keeps its accent border, marking the Tag that goes), the red-bordered irreversible callout with
//! live counts and names, and a primary button whose label follows the target.
//!
//! #354 settled the semantics: the target keeps its name, colour and `is_active`, so the callout
//! names the target's colour, or says it has none (#352).

use std::rc::Rc;

use gpui::{AnyElement, App, Window, div, prelude::*, px};

use crate::{
    dialog,
    tags::{MergeField, MergeOption, MergeTagsForm, Tag},
    theme::color,
    view::accounts::select_field::{self, SelectFieldProps},
};

/// The dialog's width: the handoff's 7e.
pub const WIDTH: gpui::Pixels = px(460.0);

pub type OnFieldClick = Rc<dyn Fn(MergeField, &mut Window, &mut App)>;
pub type OnOptionClick = Rc<dyn Fn(MergeField, usize, &mut Window, &mut App)>;

pub struct MergeTagsProps<'a> {
    pub form: &'a MergeTagsForm,
    pub options: &'a [MergeOption],
    /// The chosen source and target Tags, once each is chosen.
    pub source: Option<&'a Tag>,
    pub target: Option<&'a Tag>,
    /// Transactions carrying the source.
    pub transactions: usize,
    pub on_field_click: OnFieldClick,
    pub on_option_click: OnOptionClick,
    pub on_cancel: dialog::OnClick,
    pub on_confirm: dialog::OnClick,
}

pub fn render(props: MergeTagsProps<'_>, cx: &App) -> AnyElement {
    let MergeTagsProps {
        form,
        options,
        source,
        target,
        transactions,
        on_field_click,
        on_option_click,
        on_cancel,
        on_confirm,
    } = props;
    let source_labels = form.labels(options, MergeField::Source);
    let target_labels = form.labels(options, MergeField::Target);
    let field_click = |field: MergeField| -> dialog::OnClick {
        let on_field_click = on_field_click.clone();
        Rc::new(move |window: &mut Window, cx: &mut App| on_field_click(field, window, cx))
    };
    let option_click = |field: MergeField| -> select_field::OnOptionClick {
        let on_option_click = on_option_click.clone();
        Rc::new(move |index: usize, window: &mut Window, cx: &mut App| {
            on_option_click(field, index, window, cx)
        })
    };

    let selects = div()
        .flex()
        .items_start()
        .gap(px(10.0))
        .child(select_field::render(
            SelectFieldProps {
                id: "merge-tags-source",
                label: crate::msg::desktop_tags_merge_source_label().into(),
                options: &source_labels,
                state: &form.source,
                focused: form.focused == MergeField::Source,
                accent: true,
                read_only: None,
                on_field_click: field_click(MergeField::Source),
                on_option_click: option_click(MergeField::Source),
            },
            cx,
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
        .child(select_field::render(
            SelectFieldProps {
                id: "merge-tags-target",
                label: crate::msg::desktop_tags_merge_target_label().into(),
                options: &target_labels,
                state: &form.target,
                focused: form.focused == MergeField::Target,
                accent: false,
                read_only: None,
                on_field_click: field_click(MergeField::Target),
                on_option_click: option_click(MergeField::Target),
            },
            cx,
        ))
        .into_any_element();

    let pair = source.zip(target);
    let submit = target.map_or_else(crate::msg::desktop_tags_merge_submit_empty, |target| {
        crate::msg::desktop_tags_merge_submit(&target.name)
    });

    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(
            crate::msg::desktop_tags_merge_title(),
            false,
            cx,
        ))
        .child(dialog::body([selects, callout(pair, transactions, cx)]))
        .child(dialog::action_row(
            [
                dialog::cancel_button("merge-tags-cancel", on_cancel, cx).into_any_element(),
                dialog::confirm_button(
                    "merge-tags-confirm",
                    submit,
                    pair.is_some(),
                    false,
                    on_confirm,
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
fn callout(pair: Option<(&Tag, &Tag)>, transactions: usize, cx: &App) -> AnyElement {
    let panel = div()
        .flex()
        .flex_wrap()
        .p(px(10.0))
        .bg(color::chrome(cx))
        .border_l(px(2.0))
        .border_color(color::accent(cx))
        .text_size(px(11.5))
        .text_color(color::muted(cx));
    let Some((source, target)) = pair else {
        return panel
            .child(crate::msg::desktop_tags_merge_choose())
            .into_any_element();
    };
    panel
        .children(
            callout_segments(source, target, transactions)
                .into_iter()
                .map(|segment| {
                    let text = div().child(segment.text);
                    match segment.tag.as_deref() {
                        Some("strong") => text
                            .font_weight(gpui::FontWeight::EXTRA_BOLD)
                            .text_color(color::foreground(cx)),
                        _ => text,
                    }
                }),
        )
        .into_any_element()
}

fn callout_segments(source: &Tag, target: &Tag, transactions: usize) -> Vec<lib_locale::Segment> {
    let count = i64::try_from(transactions).unwrap_or(i64::MAX);
    if target.color.is_some() {
        crate::msg::desktop_tags_merge_callout(count, &source.name, &target.name)
    } else {
        crate::msg::desktop_tags_merge_callout_no_colour(count, &source.name, &target.name)
    }
}

#[cfg(test)]
mod tests {
    use lib_core::HexColor;

    use super::*;

    fn tag(id: u32, name: &str, color: Option<HexColor>) -> Tag {
        Tag {
            id,
            name: name.to_string(),
            color,
            is_active: true,
        }
    }

    fn text(segments: &[lib_locale::Segment]) -> String {
        segments
            .iter()
            .map(|segment| segment.text.as_str())
            .collect()
    }

    #[test]
    fn the_callout_bolds_the_count_and_names_the_targets_colour_or_its_lack() {
        crate::locale::init_for_tests();
        let source = tag(1, "Shared", None);
        let coloured = tag(2, "shared", Some(crate::tags::swatches()[0].clone()));
        let segments = callout_segments(&source, &coloured, 9);
        let strong: Vec<&str> = segments
            .iter()
            .filter(|segment| segment.tag.as_deref() == Some("strong"))
            .map(|segment| segment.text.as_str())
            .collect();
        assert_eq!(strong, ["9 transactions"]);
        assert!(text(&segments).contains("using shared's colour"));

        let plain = tag(2, "shared", None);
        assert!(text(&callout_segments(&source, &plain, 1)).contains("which has no colour"));
    }
}
