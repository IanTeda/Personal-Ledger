//! The floating command palette (`docs/ux/desktop/Shell & Navigation/README.md`'s "1d" spec,
//! the shell's `:`/`InputMode::Command` state) -- `Shell` owns `Option<Palette>`, `Some` only
//! while that mode is active, mirroring how `bin-tui`'s own `Shell` owns
//! `Option<popup::command::CommandPopup>` (`docs/ux/desktop/README.md`'s Notes: "the desktop
//! shell should reuse that shape, not invent a second one").
//!
//! Ranking, filtering and selection are `gpui`-free (unit-tested without a window), the same
//! split `nav.rs` uses between pure state and its `RenderOnce` chrome; `render` is the one
//! method that touches `gpui`.

use gpui::{BoxShadow, div, point, prelude::*, px};

use crate::{
    command::{self, Command},
    theme::color,
};

/// Fixed width. Narrower than the "1d" spec's own `820px` -- another deliberate override (see
/// `crate::command`'s own module doc for the domain-grouping one): now that the binding sits
/// right next to the name instead of pushed to the row's far edge, 820px left the description
/// column far wider than the registry's own longest description ever needs, sized instead to
/// comfortably fit that longest description ("the tags every transaction can carry any number
/// of", 50 characters) plus the name/binding columns before it.
pub const WIDTH: gpui::Pixels = px(640.0);

/// Distance from the window's top edge: the "1d" spec's own `top: 96px`.
pub const TOP_OFFSET: gpui::Pixels = px(96.0);

/// Width of the `<command>` column -- fixed (rather than `flex_1`, which used to push the
/// binding off to the row's far edge) so the binding sits left-aligned right next to the name
/// it triggers, freeing the rest of the row for the description. Sized to comfortably fit the
/// registry's longest names ("transactions", "account new") at this text size.
const NAME_COLUMN_WIDTH: gpui::Pixels = px(130.0);

/// Width of the `<binding>` column, immediately after the name column.
const BINDING_COLUMN_WIDTH: gpui::Pixels = px(64.0);

/// One row of the resting/filtered list: a domain header (resting state only) or a command
/// entry -- mirrors `bin-tui`'s own `popup::command::Row` exactly.
enum Row {
    Header(&'static str),
    Entry(&'static Command),
}

/// State for the floating command palette: its input buffer and result selection. Filtering and
/// ranking are recomputed from `input`/`COMMANDS` on every call rather than cached, matching
/// `nav.rs`'s own "never tracked as its own field, so it can never drift out of sync" idiom --
/// the registry is small enough that this costs nothing.
#[derive(Default)]
pub struct Palette {
    input: String,
    /// Index into [`Self::matches`], clamped there rather than here.
    selected: usize,
    /// Previously run command names, most-recent-first, deduplicated -- `Shell` owns the
    /// durable copy (this instance is discarded when the palette closes) and clones it in on
    /// [`Self::with_history`], mirroring the "`Shell` owns `Option<Palette>`" split the module
    /// doc describes: history outlives any one palette session.
    history: Vec<String>,
    /// How far `^r` has walked back into [`Self::history`] -- `None` before the first press.
    /// Reset by any edit ([`Self::push_char`], [`Self::backspace`]) so resuming typing always
    /// starts a fresh browse rather than picking up mid-cycle.
    history_cursor: Option<usize>,
}

impl Palette {
    pub fn new() -> Self {
        Self::default()
    }

    /// A fresh palette pre-loaded with `history` for `^r` to cycle through -- see
    /// [`Self::cycle_history_back`].
    pub fn with_history(history: Vec<String>) -> Self {
        Self {
            history,
            ..Self::default()
        }
    }

    pub fn input(&self) -> &str {
        &self.input
    }

    /// Appends a typed character, resetting the selection to the top -- the filtered set
    /// changes on every keystroke, so the old index may no longer point at anything.
    pub fn push_char(&mut self, c: char) {
        self.input.push(c);
        self.selected = 0;
        self.history_cursor = None;
    }

    pub fn backspace(&mut self) {
        self.input.pop();
        self.selected = 0;
        self.history_cursor = None;
    }

    /// `tab`: fills the input with the selected result's full command name -- mirrors `enter`'s
    /// own "runs the selection" semantics rather than completing a shared prefix across every
    /// match. A no-op when nothing is selected (the filtered set is empty).
    pub fn complete_selected(&mut self) {
        if let Some(command) = self.selected_command() {
            self.input = command.name.to_string();
            self.selected = 0;
            self.history_cursor = None;
        }
    }

    /// `^r`: walks backward (older) through [`Self::history`] into the input buffer. The first
    /// press recalls the most recently run command; each further press without an intervening
    /// edit walks one entry further back, clamped at the oldest rather than wrapping. A no-op
    /// with no history yet.
    pub fn cycle_history_back(&mut self) {
        if self.history.is_empty() {
            return;
        }
        let next = match self.history_cursor {
            Some(index) => (index + 1).min(self.history.len() - 1),
            None => 0,
        };
        self.history_cursor = Some(next);
        self.input = self.history[next].clone();
        self.selected = 0;
    }

    pub fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn move_down(&mut self) {
        let count = self.matches().len();
        if count > 0 && self.selected + 1 < count {
            self.selected += 1;
        }
    }

    /// Every registered command matching the current input case-insensitively, against its
    /// name, description or owning [`Command::domain`], ranked by [`match_rank`] -- ties keep
    /// [`command::all`]'s own registration order (`Vec::sort_by_key` is stable).
    fn matches(&self) -> Vec<&'static Command> {
        let needle = self.input.to_lowercase();
        let mut matches: Vec<_> = command::all()
            .filter(|command| {
                needle.is_empty()
                    || command.name.to_lowercase().contains(&needle)
                    || command.description.to_lowercase().contains(&needle)
                    || command.domain.to_lowercase().contains(&needle)
            })
            .collect();
        matches.sort_by_key(|command| match_rank(command, &needle));
        matches
    }

    /// The rows to render: every domain with a header at rest (`input` empty), mirroring
    /// `bin-tui`'s own `CommandPopup::rows`/`Row::Header` -- or a flat, header-less list of
    /// matches while filtering, since once you're typing you already know what you want and
    /// ranking beats grouping. Domain headers are inserted purely for display: they don't
    /// consume a `selected` index, which continues to count entries only (`Self::matches`'s own
    /// order), the same way `bin-tui`'s `selected` skips its own `Row::Header` rows.
    fn rows(&self) -> Vec<Row> {
        let matches = self.matches();
        if self.input.is_empty() {
            let mut rows = Vec::with_capacity(matches.len() * 2);
            let mut last_domain = None;
            for command in matches {
                if last_domain != Some(command.domain) {
                    rows.push(Row::Header(command.domain));
                    last_domain = Some(command.domain);
                }
                rows.push(Row::Entry(command));
            }
            rows
        } else {
            matches.into_iter().map(Row::Entry).collect()
        }
    }

    /// The command the current selection would run on `Enter`.
    pub fn selected_command(&self) -> Option<&'static Command> {
        self.matches().get(self.selected).copied()
    }

    /// The floating overlay itself: `position: absolute`, centred at [`TOP_OFFSET`] from the
    /// window's top edge (the "1d" spec's `left: 50%; transform: translateX(-50%)`, achieved
    /// here with a full-width absolute row that centres its one child, rather than a percentage
    /// position `gpui` has no transform to recentre). Borrows `self` rather than consuming it
    /// (unlike the chrome's `RenderOnce` components) since `Shell` must keep this state past the
    /// render call that draws it.
    pub fn render(&self) -> gpui::AnyElement {
        let rows = self.rows();
        let match_count = rows
            .iter()
            .filter(|row| matches!(row, Row::Entry(_)))
            .count();
        let selected = self.selected.min(match_count.saturating_sub(1));

        // Rows are a mix of headers and entries; `selected` counts entries only (mirroring
        // `bin-tui`'s own `selected_row_index`), so this walks both in lockstep rather than
        // using the row's own index to decide highlighting.
        let mut entry_index = 0;
        let rendered_rows = rows.into_iter().map(|row| match row {
            Row::Header(domain) => domain_header(domain).into_any_element(),
            Row::Entry(command) => {
                let is_selected = entry_index == selected;
                entry_index += 1;
                result_row(command, &self.input, is_selected).into_any_element()
            }
        });

        div()
            .absolute()
            .top(TOP_OFFSET)
            .left(px(0.0))
            .right(px(0.0))
            .flex()
            .justify_center()
            .child(
                div()
                    .w(WIDTH)
                    .flex()
                    .flex_col()
                    .bg(color::GROUND)
                    .border_t(px(2.0))
                    .border_b(px(2.0))
                    .border_l(px(2.0))
                    .border_r(px(2.0))
                    .border_color(color::INK)
                    .shadow(vec![BoxShadow {
                        color: color::PALETTE_SHADOW.into(),
                        offset: point(px(0.0), px(12.0)),
                        blur_radius: px(32.0),
                        spread_radius: px(0.0),
                    }])
                    .child(input_row(&self.input, match_count))
                    .child(div().h(px(2.0)).bg(color::INK))
                    .children(rendered_rows)
                    .child(div().h(px(1.0)).bg(color::HAIRLINE))
                    .child(footer_row()),
            )
            .into_any_element()
    }
}

/// Ranks `command` against a (lowercased, possibly empty) `needle` -- mirrors `bin-tui`'s own
/// `match_rank` exactly: `0` for a name that starts with the needle (the resting, empty-query
/// case ties every command here, so registration order wins outright), `1` for the needle
/// appearing anywhere in the name or the owning [`Command::domain`] (and not already tier `0`),
/// `2` for a match found only in the description.
fn match_rank(command: &Command, needle: &str) -> u8 {
    let name = command.name.to_lowercase();
    if needle.is_empty() || name.starts_with(needle) {
        0
    } else if name.contains(needle) || command.domain.to_lowercase().contains(needle) {
        1
    } else {
        2
    }
}

/// A domain header in the resting-state list (`Row::Header`) -- mirrors `bin-tui`'s own
/// section-header styling for its command popup (dim, extra-bold, uppercase, small).
fn domain_header(domain: &'static str) -> impl IntoElement {
    div()
        .px(px(16.0))
        .pt(px(10.0))
        .pb(px(4.0))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(10.0))
        .text_color(color::INK_TERTIARY)
        .child(domain.to_uppercase())
}

/// The "1d" spec's input row: leading `>`, the live query with a block caret, right-aligned
/// match count.
fn input_row(input: &str, match_count: usize) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(px(10.0))
        .px(px(16.0))
        .py(px(12.0))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(15.0))
        .text_color(color::INK)
        .child(">")
        .child(div().flex_1().child(input.to_string()))
        .child(div().w(px(8.0)).h(px(17.0)).bg(color::ACCENT))
        .child(
            div()
                .font_weight(gpui::FontWeight::NORMAL)
                .text_size(px(11.5))
                .text_color(color::INK_SECONDARY)
                .child(format!("{match_count} of {}", command::COMMANDS.len())),
        )
}

/// One result row: the command name with its matched substring picked out, then the binding
/// column (an em dash when the command has none), then the description -- selected styling
/// inverts per the "1d" spec ("Selected result: ink fill, ground text, accent-on-dark
/// substring").
fn result_row(command: &'static Command, needle: &str, selected: bool) -> impl IntoElement {
    let (bg, text, matched, description_color, binding_color) = if selected {
        (
            Some(color::INK),
            color::INK_ON_DARK,
            color::ACCENT_ON_DARK,
            color::INK_ON_DARK_TERTIARY,
            color::INK_ON_DARK_SECONDARY,
        )
    } else {
        (
            None,
            color::INK,
            color::ACCENT,
            color::INK_SECONDARY,
            color::INK_TERTIARY,
        )
    };

    div()
        .when_some(bg, |this, bg| this.bg(bg))
        .flex()
        .items_center()
        .gap(px(12.0))
        .px(px(16.0))
        .py(px(8.0))
        .child(
            div()
                .w(NAME_COLUMN_WIDTH)
                .flex_none()
                .text_color(text)
                .child(highlighted_name(command.name, needle, matched, text)),
        )
        .child(
            div()
                .w(BINDING_COLUMN_WIDTH)
                .flex_none()
                .text_size(px(11.5))
                .text_color(binding_color)
                .child(command.binding.unwrap_or("\u{2014}")),
        )
        .child(
            div()
                .flex_1()
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis()
                .text_color(description_color)
                .child(command.description),
        )
}

/// The "1d" spec's footer hint row, below the rule under the result list: every key the palette
/// answers to, in the handoff's own order and wording.
fn footer_row() -> impl IntoElement {
    div()
        .px(px(16.0))
        .py(px(8.0))
        .text_size(px(11.5))
        .text_color(color::INK_SECONDARY)
        .child("\u{2191}\u{2193} select \u{b7} tab complete \u{b7} enter run \u{b7} ^r history \u{b7} esc close")
}

/// Splits `name` around the first case-insensitive occurrence of `needle`, rendering the match
/// in `matched_color` at extra-bold weight and the rest in `default_color` -- an empty needle
/// (the resting, unfiltered state) highlights nothing.
fn highlighted_name(
    name: &'static str,
    needle: &str,
    matched_color: gpui::Rgba,
    default_color: gpui::Rgba,
) -> gpui::AnyElement {
    if needle.is_empty() {
        return div()
            .text_color(default_color)
            .child(name)
            .into_any_element();
    }

    let lower_name = name.to_lowercase();
    let Some(start) = lower_name.find(needle) else {
        return div()
            .text_color(default_color)
            .child(name)
            .into_any_element();
    };
    let end = start + needle.len();

    div()
        .flex()
        .child(name[..start].to_string())
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_color(matched_color)
                .child(name[start..end].to_string()),
        )
        .child(name[end..].to_string())
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typing_appends_to_the_input_buffer() {
        let mut palette = Palette::new();
        palette.push_char('b');
        palette.push_char('u');
        palette.push_char('d');
        assert_eq!(palette.input(), "bud");
    }

    #[test]
    fn backspace_removes_the_last_character() {
        let mut palette = Palette::new();
        palette.push_char('a');
        palette.backspace();
        assert_eq!(palette.input(), "");
    }

    #[test]
    fn backspace_on_empty_input_does_not_panic() {
        let mut palette = Palette::new();
        palette.backspace();
        assert_eq!(palette.input(), "");
    }

    #[test]
    fn resting_state_lists_every_command_in_registration_order() {
        let palette = Palette::new();
        assert_eq!(palette.matches().len(), command::COMMANDS.len());
        assert_eq!(palette.matches()[0].name, command::COMMANDS[0].name);
    }

    #[test]
    fn resting_rows_include_a_header_per_domain() {
        let palette = Palette::new();
        let header_count = palette
            .rows()
            .iter()
            .filter(|row| matches!(row, Row::Header(_)))
            .count();
        let domain_count = command::COMMANDS
            .iter()
            .map(|command| command.domain)
            .collect::<std::collections::HashSet<_>>()
            .len();
        assert_eq!(header_count, domain_count);
    }

    #[test]
    fn resting_rows_have_no_headers_split_across_a_domains_entries() {
        // Every header should be followed immediately by its domain's own entries, then the
        // next header -- never re-appear once a different domain's header has been emitted.
        let mut seen = Vec::new();
        for row in Palette::new().rows() {
            if let Row::Header(domain) = row {
                assert!(
                    !seen.contains(&domain),
                    "domain {domain:?} header appeared twice"
                );
                seen.push(domain);
            }
        }
    }

    #[test]
    fn filtered_rows_have_no_headers() {
        let mut palette = Palette::new();
        for c in "tags".chars() {
            palette.push_char(c);
        }
        assert!(
            palette
                .rows()
                .iter()
                .all(|row| matches!(row, Row::Entry(_))),
            "a filtered (non-resting) list should be flat, no domain headers"
        );
    }

    #[test]
    fn typing_filters_to_matching_commands_only() {
        let mut palette = Palette::new();
        for c in "tags".chars() {
            palette.push_char(c);
        }
        let matches = palette.matches();
        assert!(!matches.is_empty());
        assert!(matches.len() < command::COMMANDS.len());
        assert!(matches.iter().all(|c| c.name.contains("tags")));
    }

    #[test]
    fn typing_resets_the_selection() {
        let mut palette = Palette::new();
        palette.move_down();
        palette.move_down();
        palette.push_char('a');
        assert_eq!(palette.selected, 0);
    }

    #[test]
    fn selection_is_clamped_to_the_current_match_count() {
        let mut palette = Palette::new();
        for c in "account".chars() {
            palette.push_char(c);
        }
        let count = palette.matches().len();
        for _ in 0..count + 5 {
            palette.move_down();
        }
        assert_eq!(palette.selected, count - 1);
    }

    #[test]
    fn move_up_is_clamped_at_the_top() {
        let mut palette = Palette::new();
        palette.move_up();
        assert_eq!(palette.selected, 0);
    }

    #[test]
    fn an_exact_name_match_outranks_a_substring_match() {
        let mut palette = Palette::new();
        for c in "reports".chars() {
            palette.push_char(c);
        }
        assert_eq!(palette.selected_command().map(|c| c.name), Some("reports"));
    }

    #[test]
    fn a_description_only_match_still_surfaces() {
        let mut palette = Palette::new();
        for c in "ledger".chars() {
            palette.push_char(c);
        }
        // No command is named "ledger", but several describe themselves with the word (and
        // "open"/"new"/"close" also match via their own "Ledger" domain -- see the next test).
        assert!(!palette.matches().is_empty());
    }

    #[test]
    fn a_domain_match_outranks_a_description_only_match() {
        let mut palette = Palette::new();
        for c in "ledger".chars() {
            palette.push_char(c);
        }
        // "open"/"new"/"close" match via their own "Ledger" domain (tier 1); "settings"
        // matches only because its description says "ledger preferences" (tier 2).
        let matches = palette.matches();
        let ledger_domain_rank = matches.iter().position(|c| c.name == "open").unwrap();
        let settings_rank = matches.iter().position(|c| c.name == "settings").unwrap();
        assert!(
            ledger_domain_rank < settings_rank,
            "a domain match should rank above a description-only match"
        );
    }

    #[test]
    fn tab_completes_to_the_selected_result_full_name() {
        let mut palette = Palette::new();
        for c in "acc".chars() {
            palette.push_char(c);
        }
        palette.complete_selected();
        assert_eq!(palette.input(), "accounts");
    }

    #[test]
    fn tab_on_no_results_does_not_panic() {
        let mut palette = Palette::new();
        for c in "nonexistent-command".chars() {
            palette.push_char(c);
        }
        palette.complete_selected();
        assert_eq!(palette.input(), "nonexistent-command");
    }

    #[test]
    fn ctrl_r_with_no_history_is_a_no_op() {
        let mut palette = Palette::new();
        palette.push_char('x');
        palette.cycle_history_back();
        assert_eq!(palette.input(), "x");
    }

    #[test]
    fn ctrl_r_recalls_the_most_recent_command_first() {
        let mut palette =
            Palette::with_history(vec!["account new".to_string(), "accounts".to_string()]);
        palette.cycle_history_back();
        assert_eq!(palette.input(), "account new");
    }

    #[test]
    fn repeated_ctrl_r_walks_further_back_and_clamps_at_the_oldest() {
        let mut palette =
            Palette::with_history(vec!["account new".to_string(), "accounts".to_string()]);
        palette.cycle_history_back();
        palette.cycle_history_back();
        assert_eq!(palette.input(), "accounts");
        palette.cycle_history_back();
        assert_eq!(palette.input(), "accounts");
    }

    #[test]
    fn editing_after_a_recall_resets_the_history_cursor() {
        let mut palette =
            Palette::with_history(vec!["account new".to_string(), "accounts".to_string()]);
        palette.cycle_history_back();
        palette.cycle_history_back();
        assert_eq!(palette.input(), "accounts");
        palette.backspace();
        // Cursor reset -- the next `^r` recalls the most recent entry again, not "accounts".
        palette.cycle_history_back();
        assert_eq!(palette.input(), "account new");
    }
}
