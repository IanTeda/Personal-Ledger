//! The "delete payee" popup — `d` on a Payees list row, or `^d` from the edit popup (8c)
//! (`docs/ux/tui/payees/README.md` "8e — Delete"). One struct whose content varies with
//! whether the Payee is referenced, mirroring `popup::account::delete`'s own "varies with
//! emptiness" shape rather than a fixed two-variant enum — simpler here than Account's own,
//! since nothing about a Payee's delete needs a variable-length transfer sub-form; only the
//! `delete` option's availability and a few counts vary, so a fixed row layout (like
//! `popup::payee::new`/`edit`) fits, unlike Account's dynamic `RowRenderer` list.
//!
//! **The refusal is attributed to the database's own foreign-key pragma, never a UI rule** —
//! `can_delete` is resolved once, at open time, from the Payee's own transaction/alias counts
//! (mirroring `popup::account::delete`'s own `needs_transfer`), and the popup states plainly
//! *why* delete is unavailable rather than just disabling a control. A renamed Payee can
//! therefore never be deleted: `PayeeStore::rename` always leaves an alias referencing it, so
//! `alias_count` never returns to zero once a rename has happened.
//!
//! **`^s` deactivates or deletes depending on which action is selected** — unlike
//! `popup::account::delete`'s own `^s`(delete)/`^a`(deactivate) split, this popup has one radio
//! choice (`(•) deactivate` / `( ) delete`) and one commit key, per the handoff's own "tab
//! next field · ^s deactivate/delete per selection". `m` jumps to the rename-matches popup
//! (8d) — the closing line's own redirect ("fold a duplicate in as a match instead") made
//! real as a keybinding, mirroring `popup::account::edit`'s own `^d` hand-off pattern. `Shell`
//! only routes bare `m` there while `confirm` *doesn't* have focus
//! ([`DeletePayeePopup::confirm_field_focused`]) — a Payee's own name can contain the letter
//! `m`, and typing it into `confirm` must never be hijacked, the same reasoning
//! `popup::account::new`'s own `type_field_focused` guard uses for `h`/`l`.

use lib_core::RowID;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::payee::PayeeStore;
use crate::popup::REFERENCE_TERMINAL_WIDTH;

const ACCENT: Color = Color::Red;
const POPUP_WIDTH_PERCENT: u32 = 88;
const POPUP_WIDTH: u16 = ((REFERENCE_TERMINAL_WIDTH as u32 * POPUP_WIDTH_PERCENT) / 100) as u16;
const LABEL_WIDTH: u16 = "suggestions".len() as u16 + 1;

/// Content rows inside the border: title, rule, `references`, the refusal/allowed line, a
/// blank spacer, the two-line action radio, a blank spacer, the four-line "whose rule this is"
/// note, a blank spacer, the four-line "after deactivate" block, a blank spacer, `confirm`, a
/// blank spacer, the two-line merge redirect, the footer's rule, then the footer itself.
const CONTENT_ROWS: u16 = 1 + 1 + 1 + 1 + 1 + 2 + 1 + 4 + 1 + 4 + 1 + 1 + 1 + 2 + 1 + 1;
const POPUP_HEIGHT: u16 = CONTENT_ROWS + 2;

/// Which action the radio currently selects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DeleteAction {
    Deactivate,
    Delete,
}

/// Which field currently has focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Field {
    Action,
    Confirm,
}

/// What `Ctrl+S` would do with the current draft.
pub enum DeleteCommit {
    Deactivate(RowID),
    Delete(RowID),
}

/// The `:payee delete` popup's own draft state.
pub struct DeletePayeePopup {
    deleting_id: RowID,
    /// Resolved once, at open time, from the Payee's own transaction/alias counts — never
    /// recomputed afterwards, mirroring `popup::account::delete`'s own `needs_transfer`.
    transaction_count: u32,
    alias_count: usize,
    can_delete: bool,
    action: DeleteAction,
    confirm_input: String,
    focus: Field,
}

impl DeletePayeePopup {
    /// Opens a popup for deleting `deleting_id`. `deactivate` is always the preselected
    /// action, per the handoff's own field table — `delete` only becomes reachable (via
    /// `Field::Action`'s own space-toggle) once `can_delete` is true.
    pub fn new(store: &dyn PayeeStore, deleting_id: RowID) -> Self {
        let transaction_count = store
            .find(deleting_id)
            .map(|payee| payee.transaction_count)
            .unwrap_or(0);
        let alias_count = store.aliases(deleting_id).len();
        Self {
            deleting_id,
            transaction_count,
            alias_count,
            can_delete: transaction_count == 0 && alias_count == 0,
            action: DeleteAction::Deactivate,
            confirm_input: String::new(),
            focus: Field::Action,
        }
    }

    pub fn editing_id(&self) -> RowID {
        self.deleting_id
    }

    pub fn push_char(&mut self, c: char) {
        match self.focus {
            // Space toggles the radio rather than being typed literally — switching *to*
            // `Delete` is refused outright while `can_delete` is false, so the selection can
            // never land somewhere the popup would then have to un-select for you.
            Field::Action if c == ' ' => {
                self.action = match self.action {
                    DeleteAction::Deactivate if self.can_delete => DeleteAction::Delete,
                    _ => DeleteAction::Deactivate,
                };
            }
            Field::Action => {}
            Field::Confirm => self.confirm_input.push(c),
        }
    }

    pub fn backspace(&mut self) {
        if self.focus == Field::Confirm {
            self.confirm_input.pop();
        }
    }

    pub fn tab(&mut self) {
        self.focus = match self.focus {
            Field::Action => Field::Confirm,
            Field::Confirm => Field::Action,
        };
    }

    /// Whether `confirm` currently has focus — `Shell` checks this before routing a bare `m`
    /// to [`Action::OpenPayeeMatchesPopup`] rather than as ordinary text (a Payee's own name
    /// can contain the letter `m`; typing it into `confirm` must never be hijacked, mirroring
    /// `popup::account::new`'s own `type_field_focused` guard for `h`/`l`).
    pub fn confirm_field_focused(&self) -> bool {
        self.focus == Field::Confirm
    }

    /// What `Ctrl+S` would do — deactivating always validates; deleting needs `can_delete`
    /// *and* `confirm` to exactly match the Payee's own name (case-sensitive — this is the
    /// one irreversible action here, matching `popup::account::delete`'s own bar).
    pub fn commit(&self, store: &dyn PayeeStore) -> Option<DeleteCommit> {
        let payee = store.find(self.deleting_id)?;
        match self.action {
            DeleteAction::Deactivate => Some(DeleteCommit::Deactivate(self.deleting_id)),
            DeleteAction::Delete => {
                if !self.can_delete || self.confirm_input != payee.name {
                    return None;
                }
                Some(DeleteCommit::Delete(self.deleting_id))
            }
        }
    }

    /// Renders the floating overlay, centred and fixed-height, within `area`.
    pub fn render(&self, frame: &mut Frame, area: Rect, store: &dyn PayeeStore) {
        let Some(payee) = store.find(self.deleting_id) else {
            return;
        };
        let popup = popup_rect(area);

        frame.render_widget(Clear, popup);
        let block = Block::bordered();
        let inner = block.inner(popup);
        frame.render_widget(block, popup);

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // title
                Constraint::Length(1), // rule
                Constraint::Length(1), // references
                Constraint::Length(1), // refusal/allowed line
                Constraint::Length(1), // blank spacer
                Constraint::Length(1), // action: deactivate
                Constraint::Length(1), // action: delete
                Constraint::Length(1), // blank spacer
                Constraint::Length(1), // rule note line 1
                Constraint::Length(1), // rule note line 2
                Constraint::Length(1), // rule note line 3
                Constraint::Length(1), // rule note line 4
                Constraint::Length(1), // blank spacer
                Constraint::Length(1), // after: suggestions
                Constraint::Length(1), // after: txns
                Constraint::Length(1), // after: matches
                Constraint::Length(1), // after: reversible
                Constraint::Length(1), // blank spacer
                Constraint::Length(1), // confirm
                Constraint::Length(1), // blank spacer
                Constraint::Length(1), // redirect line 1
                Constraint::Length(1), // redirect line 2
                Constraint::Length(1), // rule
                Constraint::Length(1), // footer hints
            ])
            .split(inner);

        render_title(frame, rows[0], &payee.name);
        frame.render_widget(Block::new().borders(Borders::BOTTOM), rows[1]);

        render_field(
            frame,
            rows[2],
            "references",
            Line::from(format!(
                "{} txns · {} matches",
                self.transaction_count, self.alias_count
            )),
        );
        let refusal_text = if self.can_delete {
            "no references — this payee can be deleted".to_string()
        } else {
            "the database refuses this delete".to_string()
        };
        let refusal_style = if self.can_delete {
            dim()
        } else {
            Style::default().fg(ACCENT)
        };
        frame.render_widget(
            Paragraph::new(Span::styled(refusal_text, refusal_style)),
            rows[3],
        );
        // rows[4] is left blank — breathing space above the action radio.

        let deactivate_glyph = radio_glyph(self.action == DeleteAction::Deactivate);
        render_field(
            frame,
            rows[5],
            "action",
            Line::from(format!("{deactivate_glyph} deactivate")),
        );
        let delete_glyph = radio_glyph(self.action == DeleteAction::Delete);
        let delete_condition_style = if self.can_delete {
            dim()
        } else {
            Style::default().fg(ACCENT)
        };
        render_indented(
            frame,
            rows[6],
            Line::from(vec![
                Span::raw(format!("{delete_glyph} delete · ")),
                Span::styled("needs 0 txns, 0 matches", delete_condition_style),
            ]),
        );
        // rows[7] is left blank — breathing space above the rule note.

        frame.render_widget(
            Paragraph::new("the foreign-key pragma rejects this, not a UI rule —"),
            rows[8],
        );
        frame.render_widget(
            Paragraph::new("a transaction or the payee's own rename match points at it."),
            rows[9],
        );
        frame.render_widget(
            Paragraph::new("a renamed payee can therefore never be deleted — its own"),
            rows[10],
        );
        frame.render_widget(
            Paragraph::new("rename match references it forever."),
            rows[11],
        );
        // rows[12] is left blank — breathing space above the after-deactivate block.

        render_field(
            frame,
            rows[13],
            "suggestions",
            Line::from(Span::styled("not offered when posting", dim())),
        );
        render_field(
            frame,
            rows[14],
            "",
            Line::from(Span::styled(
                format!(
                    "{} txns · keep this payee and its name",
                    self.transaction_count
                ),
                dim(),
            )),
        );
        render_field(
            frame,
            rows[15],
            "",
            Line::from(Span::styled(
                format!(
                    "{} matches · keep resolving · nothing rewritten",
                    self.alias_count
                ),
                dim(),
            )),
        );
        render_field(
            frame,
            rows[16],
            "reversible",
            Line::from(Span::styled("a on the row turns it back on", dim())),
        );
        // rows[17] is left blank — breathing space above confirm.

        render_text_field(
            frame,
            rows[18],
            "confirm",
            &self.confirm_input,
            self.focus == Field::Confirm,
        );
        // rows[19] is left blank — breathing space above the redirect.

        frame.render_widget(
            Paragraph::new("to fold a duplicate into this payee, add its name"),
            rows[20],
        );
        frame.render_widget(Paragraph::new("as a match instead — m matches"), rows[21]);

        frame.render_widget(Block::new().borders(Borders::BOTTOM), rows[22]);
        render_footer_hints(frame, rows[23]);
    }
}

fn radio_glyph(selected: bool) -> &'static str {
    if selected { "(\u{2022})" } else { "( )" }
}

fn dim() -> Style {
    Style::default().add_modifier(Modifier::DIM)
}

fn render_title(frame: &mut Frame, area: Rect, name: &str) {
    let tag = ":payee delete";
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(tag.chars().count() as u16),
        ])
        .split(area);
    frame.render_widget(Paragraph::new(format!("delete {name}")), columns[0]);
    frame.render_widget(
        Paragraph::new(Span::styled(tag, dim())).alignment(Alignment::Right),
        columns[1],
    );
}

fn render_field(frame: &mut Frame, area: Rect, label: &str, value: Line<'static>) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(LABEL_WIDTH), Constraint::Min(0)])
        .split(area);
    frame.render_widget(Paragraph::new(Span::styled(label, dim())), columns[0]);
    frame.render_widget(Paragraph::new(value), columns[1]);
}

/// A continuation line indented to align under the value column, with no label of its own —
/// mirrors `popup::account::delete::plain_row_indented`.
fn render_indented(frame: &mut Frame, area: Rect, value: Line<'static>) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(LABEL_WIDTH), Constraint::Min(0)])
        .split(area);
    frame.render_widget(Paragraph::new(value), columns[1]);
}

fn render_text_field(frame: &mut Frame, area: Rect, label: &str, value: &str, focused: bool) {
    let mut spans = vec![Span::raw(value.to_string())];
    if focused {
        spans.push(Span::styled("\u{258c}", Style::default().fg(ACCENT)));
    }
    render_field(frame, area, label, Line::from(spans));
}

fn render_footer_hints(frame: &mut Frame, area: Rect) {
    const HINTS: &[(&str, &str)] = &[
        ("tab", "next field"),
        ("^s", "confirm"),
        ("m", "matches"),
        ("esc", "cancel"),
    ];
    let key_style = Style::default().add_modifier(Modifier::BOLD);
    let label_style = dim();

    let mut spans = Vec::with_capacity(HINTS.len() * 3);
    for (index, (key, label)) in HINTS.iter().enumerate() {
        if index > 0 {
            spans.push(Span::raw("  "));
        }
        spans.push(Span::styled(*key, key_style));
        spans.push(Span::raw(" "));
        spans.push(Span::styled(*label, label_style));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn popup_rect(area: Rect) -> Rect {
    let width = POPUP_WIDTH.min(area.width);
    let x = area.x + (area.width.saturating_sub(width)) / 2;

    let height = POPUP_HEIGHT.min(area.height);
    let anchor_y = area.y + (area.height / 6).max(1);
    let y = anchor_y.min(area.y + area.height.saturating_sub(height));

    Rect {
        x,
        y,
        width,
        height,
    }
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;
    use crate::payee::PayeeFixture;

    fn find_id(store: &PayeeFixture, name: &str) -> RowID {
        store
            .payees()
            .iter()
            .find(|payee| payee.name == name)
            .unwrap_or_else(|| panic!("fixture should seed a payee named {name}"))
            .id
    }

    fn render(popup: &DeletePayeePopup, store: &dyn PayeeStore) -> String {
        let backend = TestBackend::new(96, 30);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        terminal
            .draw(|frame| popup.render(frame, frame.area(), store))
            .expect("rendering the popup should not error");

        let buffer = terminal.backend().buffer();
        let mut text = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                text.push_str(buffer[(x, y)].symbol());
            }
            text.push('\n');
        }
        text
    }

    #[test]
    fn new_preselects_deactivate_and_focuses_the_action_radio() {
        let store = PayeeFixture::new();
        let woolworths = find_id(&store, "Woolworths");
        let popup = DeletePayeePopup::new(&store, woolworths);
        assert_eq!(popup.action, DeleteAction::Deactivate);
        assert_eq!(popup.focus, Field::Action);
        assert!(!popup.can_delete, "Woolworths has transactions and matches");
    }

    #[test]
    fn a_referenced_payee_cannot_be_switched_to_delete() {
        let store = PayeeFixture::new();
        let woolworths = find_id(&store, "Woolworths");
        let mut popup = DeletePayeePopup::new(&store, woolworths);
        popup.push_char(' ');
        assert_eq!(
            popup.action,
            DeleteAction::Deactivate,
            "delete should stay refused"
        );
    }

    #[test]
    fn an_unreferenced_payee_can_be_switched_to_delete_and_back() {
        let store = PayeeFixture::new();
        let old_vendor = find_id(&store, "Old Vendor");
        let mut popup = DeletePayeePopup::new(&store, old_vendor);
        assert!(popup.can_delete);

        popup.push_char(' ');
        assert_eq!(popup.action, DeleteAction::Delete);
        popup.push_char(' ');
        assert_eq!(popup.action, DeleteAction::Deactivate);
    }

    #[test]
    fn tab_cycles_between_action_and_confirm_and_wraps() {
        let store = PayeeFixture::new();
        let woolworths = find_id(&store, "Woolworths");
        let mut popup = DeletePayeePopup::new(&store, woolworths);
        assert_eq!(popup.focus, Field::Action);
        popup.tab();
        assert_eq!(popup.focus, Field::Confirm);
        popup.tab();
        assert_eq!(popup.focus, Field::Action);
    }

    #[test]
    fn commit_deactivates_without_needing_a_confirm() {
        let store = PayeeFixture::new();
        let woolworths = find_id(&store, "Woolworths");
        let popup = DeletePayeePopup::new(&store, woolworths);
        assert!(
            matches!(popup.commit(&store), Some(DeleteCommit::Deactivate(id)) if id == woolworths)
        );
    }

    #[test]
    fn commit_refuses_delete_on_a_referenced_payee_even_with_the_exact_name_typed() {
        let store = PayeeFixture::new();
        let woolworths = find_id(&store, "Woolworths");
        let mut popup = DeletePayeePopup::new(&store, woolworths);
        // Can't even select `Delete` (refused by `push_char`), but force it directly to prove
        // `commit` itself also refuses regardless.
        popup.action = DeleteAction::Delete;
        popup.confirm_input = "Woolworths".to_string();
        assert!(popup.commit(&store).is_none());
    }

    #[test]
    fn commit_requires_the_exact_case_sensitive_name_to_delete_an_unreferenced_payee() {
        let store = PayeeFixture::new();
        let old_vendor = find_id(&store, "Old Vendor");
        let mut popup = DeletePayeePopup::new(&store, old_vendor);
        popup.push_char(' '); // select Delete
        popup.tab();
        popup.confirm_input = "old vendor".to_string(); // wrong case
        assert!(popup.commit(&store).is_none());

        popup.confirm_input = "Old Vendor".to_string();
        assert!(matches!(popup.commit(&store), Some(DeleteCommit::Delete(id)) if id == old_vendor));
    }

    #[test]
    fn renders_the_refusal_worded_as_the_databases_own_rule() {
        let store = PayeeFixture::new();
        let woolworths = find_id(&store, "Woolworths");
        let text = render(&DeletePayeePopup::new(&store, woolworths), &store);

        assert!(text.contains("184 txns · 2 matches"));
        assert!(text.contains("the database refuses this delete"));
        assert!(text.contains("foreign-key pragma"));
        assert!(text.contains("not a UI rule"));
        assert!(text.contains("needs 0 txns, 0 matches"));
    }

    #[test]
    fn renders_delete_as_allowed_for_an_unreferenced_payee() {
        let store = PayeeFixture::new();
        let old_vendor = find_id(&store, "Old Vendor");
        let text = render(&DeletePayeePopup::new(&store, old_vendor), &store);
        assert!(text.contains("0 txns · 0 matches"));
        assert!(text.contains("this payee can be deleted"));
    }

    #[test]
    fn shows_the_after_deactivate_block_and_the_merge_redirect() {
        let store = PayeeFixture::new();
        let woolworths = find_id(&store, "Woolworths");
        let text = render(&DeletePayeePopup::new(&store, woolworths), &store);

        assert!(text.contains("not offered when posting"));
        assert!(text.contains("184 txns · keep this payee and its name"));
        assert!(text.contains("2 matches · keep resolving"));
        assert!(text.contains("nothing rewritten"));
        assert!(text.contains("a on the row turns it back on"));
        assert!(text.contains("add its name"));
        assert!(text.contains("m matches"));
    }

    #[test]
    fn shows_the_title_and_footer_hints() {
        let store = PayeeFixture::new();
        let woolworths = find_id(&store, "Woolworths");
        let text = render(&DeletePayeePopup::new(&store, woolworths), &store);

        assert!(text.contains("delete Woolworths"));
        assert!(text.contains(":payee delete"));
        for key in ["tab", "^s", "m", "esc"] {
            assert!(text.contains(key), "{key} hint missing");
        }
    }

    #[test]
    fn popup_rect_stays_within_the_terminal_area() {
        let area = Rect::new(0, 0, 96, 30);
        let popup = popup_rect(area);
        assert!(popup.x + popup.width <= area.width);
        assert!(popup.y + popup.height <= area.height);
    }
}
