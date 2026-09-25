//! The "delete account" popup — `d` on an Accounts list row, or `:acct delete <acct> [into
//! <acct>]` (`docs/ux/tui/accounts/README.md` "7d — Delete"), the only irreversible operation
//! in this view. Genuinely interactive and genuinely mutates the fixture (transfers, then
//! deletes), the same as `crate::popup::category`'s popups — there's no Category precedent
//! for a typed-name delete confirmation yet (Category's own delete is still "Not yet
//! designed"), so the confirm-by-typing-the-name behaviour here follows
//! `crate::popup::unit::delete`'s §4e ("type the code") treatment instead, per this ticket's
//! own instruction to match "3e's unit-delete pattern". Unlike `popup::unit::delete`, this
//! isn't a fixed two-variant (`Refused`/`Allowed`) enum — an Account's delete is never
//! *permanently* refused the way a referenced Unit's is; it just needs a same-Unit transfer
//! target first when the account isn't empty, so this is one struct whose field set and
//! preview vary with that one fact, resolved once at open time.
//!
//! **One literal simplification of the handoff's own drawing**: the `┌ move to` / `└
//! candidates` rows are drawn with a partial box-drawing bracket around them; this renders
//! them as two plain label rows instead, matching this map's own "not authoritative about
//! exact colours/borders" fidelity note elsewhere.
//!
//! **`^a` (deactivate), not the handoff's own bare `a`**: typing a real account name into
//! `confirm` (or a real candidate name into `move to`) needs every letter, including `a` —
//! "Everyday Spending", "Mortgage Offset" and most real names contain one. A bare `a` shortcut
//! would make the confirm field impossible to type into honestly; `^a` mirrors the New/Edit
//! popups' own deactivate binding instead, and the rendered hint text says `^a`, not `a`, so
//! the popup never shows a key it doesn't actually answer to.

use lib_core::{AccountType, Money, RowID};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::{
    account::{Account, AccountStore},
    msg,
    popup::REFERENCE_TERMINAL_WIDTH,
};

const ACCENT: Color = Color::Red;
const POPUP_WIDTH_PERCENT: u32 = 88;
const POPUP_WIDTH: u16 = ((REFERENCE_TERMINAL_WIDTH as u32 * POPUP_WIDTH_PERCENT) / 100) as u16;
const LABEL_WIDTH: u16 = "transactions".len() as u16 + 1;

/// One body row's rendering logic, closed over whatever it needs to draw itself — see
/// [`DeleteAccountPopup::body_rows`]'s own doc for why this is a list of closures rather than
/// a fixed `Layout` + match.
type RowRenderer<'a> = Box<dyn Fn(&mut Frame, Rect) + 'a>;

/// Which field currently has focus. `move_to` only ever exists (and gains focus) when the
/// account being deleted is non-empty — see [`DeleteAccountPopup::needs_transfer`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Field {
    MoveTo,
    Confirm,
}

/// The `:acct delete` popup's own draft state.
pub struct DeleteAccountPopup {
    deleting_id: RowID,
    /// Resolved once, at open time, from the account's own transaction/Balance-Check counts —
    /// never recomputed afterwards, so typing into `confirm` can't somehow make this flip.
    needs_transfer: bool,
    target_input: String,
    confirm_input: String,
    focus: Field,
}

impl DeleteAccountPopup {
    /// Opens a popup for deleting `deleting_id`. Focus starts on `move_to` when the account
    /// holds transactions or Balance Checks, on `confirm` otherwise (there's nothing else to
    /// fill in on an empty account).
    pub fn new(store: &dyn AccountStore, deleting_id: RowID) -> Self {
        let needs_transfer = store
            .find(deleting_id)
            .map(|account| account.transaction_count > 0 || !account.balance_checks.is_empty())
            .unwrap_or(false);
        Self {
            deleting_id,
            needs_transfer,
            target_input: String::new(),
            confirm_input: String::new(),
            focus: if needs_transfer {
                Field::MoveTo
            } else {
                Field::Confirm
            },
        }
    }

    pub fn push_char(&mut self, c: char) {
        match self.focus {
            Field::MoveTo => self.target_input.push(c),
            Field::Confirm => self.confirm_input.push(c),
        }
    }

    pub fn backspace(&mut self) {
        match self.focus {
            Field::MoveTo => {
                self.target_input.pop();
            }
            Field::Confirm => {
                self.confirm_input.pop();
            }
        }
    }

    /// `Tab`: completes `move_to` against a transfer candidate's name (case-insensitive
    /// prefix match) when it has focus and one exists; otherwise advances focus. A no-op
    /// single-field "cycle" on an empty account, where `move_to` never exists to begin with.
    pub fn tab(&mut self, store: &dyn AccountStore) {
        if self.focus == Field::MoveTo {
            let needle = self.target_input.to_lowercase();
            if let Some(candidate) = store
                .transfer_candidates(self.deleting_id)
                .into_iter()
                .find(|account| account.name.to_lowercase().starts_with(&needle))
                && candidate.name != self.target_input
            {
                self.target_input = candidate.name.clone();
                return;
            }
        }
        if self.needs_transfer {
            self.focus = match self.focus {
                Field::MoveTo => Field::Confirm,
                Field::Confirm => Field::MoveTo,
            };
        }
    }

    /// The transfer candidate `move_to` currently resolves to (a case-insensitive exact match
    /// against one of `AccountStore::transfer_candidates`' own names), if any.
    fn resolved_target<'a>(&self, store: &'a dyn AccountStore) -> Option<&'a Account> {
        let needle = self.target_input.to_lowercase();
        store
            .transfer_candidates(self.deleting_id)
            .into_iter()
            .find(|account| account.name.to_lowercase() == needle)
    }

    /// The `(deleting_id, target_id)` `^s` would submit to `AccountStore::delete`, or `None`
    /// while the draft doesn't validate: `confirm` must exactly match the account's own name
    /// (case-sensitive — this is the one irreversible action in the view, and "close enough"
    /// isn't the bar for it), and a non-empty account additionally needs a resolved transfer
    /// target.
    pub fn delete_fields(&self, store: &dyn AccountStore) -> Option<(RowID, Option<RowID>)> {
        let account = store.find(self.deleting_id)?;
        if self.confirm_input != account.name {
            return None;
        }
        if self.needs_transfer {
            let target = self.resolved_target(store)?;
            Some((self.deleting_id, Some(target.id)))
        } else {
            Some((self.deleting_id, None))
        }
    }

    /// `^a`: the `(id, name, type, active)` `UpdateAccount` would apply to deactivate the
    /// account instead of deleting it — "the soft alternative to deletion", per the handoff's
    /// own "`a` deactivates instead, from inside the delete overlay". Needs `store` (unlike
    /// `delete_fields`, which only ever needs the already-typed confirm/target) because this
    /// popup holds no draft copy of the account's own name/type to carry forward unchanged.
    pub fn deactivate_fields(
        &self,
        store: &dyn AccountStore,
    ) -> Option<(RowID, String, AccountType, bool)> {
        let account = store.find(self.deleting_id)?;
        Some((
            self.deleting_id,
            account.name.clone(),
            account.account_type.clone(),
            false,
        ))
    }

    /// Renders the floating overlay, centred and sized to its own dynamic content (whether a
    /// transfer is needed, and whether its target has resolved yet), within `area`.
    pub fn render(&self, frame: &mut Frame, area: Rect, store: &dyn AccountStore) {
        let Some(account) = store.find(self.deleting_id) else {
            return;
        };

        let body = self.body_rows(store, account);
        let content_rows = 1 + 1 + body.len() as u16 + 1 + 1; // title, rule, body, rule, footer
        let popup = popup_rect(area, content_rows);

        frame.render_widget(Clear, popup);
        let block = Block::bordered();
        let inner = block.inner(popup);
        frame.render_widget(block, popup);

        let mut constraints = vec![Constraint::Length(1), Constraint::Length(1)];
        constraints.extend(std::iter::repeat_n(Constraint::Length(1), body.len()));
        constraints.extend([Constraint::Length(1), Constraint::Length(1)]);
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(inner);

        render_title(frame, rows[0], &account.name);
        frame.render_widget(Block::new().borders(Borders::BOTTOM), rows[1]);
        for (render_row, area) in body.iter().zip(rows[2..2 + body.len()].iter()) {
            render_row(frame, *area);
        }
        let footer_rule_index = 2 + body.len();
        frame.render_widget(
            Block::new().borders(Borders::BOTTOM),
            rows[footer_rule_index],
        );
        render_footer_hints(frame, rows[footer_rule_index + 1]);
    }

    /// Builds the popup's variable body as a list of row-rendering closures — the row *count*
    /// depends on whether a transfer is needed, whether its target has resolved, and whether
    /// the source account has any Balance Checks to lose, so a fixed `Layout` constant (every
    /// other popup in this map can use one) doesn't fit here.
    fn body_rows<'a>(
        &'a self,
        store: &'a dyn AccountStore,
        account: &'a Account,
    ) -> Vec<RowRenderer<'a>> {
        let mut rows: Vec<RowRenderer<'a>> = Vec::new();

        let balance = store.balance(self.deleting_id);
        rows.push(field_row(
            msg::tui_account_delete_field_holds(),
            Line::from(format!(
                "{} txns · {} {}",
                account.transaction_count,
                format_money_at(&balance, account.unit.decimal_places),
                account.unit.code
            )),
        ));
        if account.open_count > 0 || !account.balance_checks.is_empty() {
            rows.push(plain_row_indented(Line::from(Span::styled(
                format!(
                    "{} unreconciled · {} balance checks",
                    account.open_count,
                    account.balance_checks.len()
                ),
                dim(),
            ))));
        }

        if self.needs_transfer {
            rows.push(blank_row());
            rows.push(field_row(
                msg::tui_account_delete_field_transactions(),
                Line::from(vec![
                    Span::raw("( ) delete them too · "),
                    Span::styled("refused", Style::default().fg(ACCENT)),
                ]),
            ));
            rows.push(plain_row_indented(Line::from(
                "(•) move to another account",
            )));
            rows.push(field_row(
                msg::tui_account_delete_field_move_to(),
                text_field_value(&self.target_input, self.focus == Field::MoveTo),
            ));
            let total = store.accounts().len();
            let candidates = store.transfer_candidates(self.deleting_id);
            rows.push(field_row(
                msg::tui_account_delete_field_candidates(),
                Line::from(Span::styled(
                    format!(
                        "{} only · {} of {total}  · tab",
                        account.unit.code,
                        candidates.len()
                    ),
                    dim(),
                )),
            ));
            rows.push(blank_row());

            match self.resolved_target(store) {
                Some(target) => {
                    let target_balance = store.balance(target.id);
                    let new_balance =
                        Money(target_balance.0.clone() + account.transactions_sum.0.clone());
                    rows.push(plain_row(Line::from(
                        msg::tui_account_delete_heading_after(),
                    )));
                    rows.push(field_row(
                        msg::tui_account_delete_field_into(),
                        Line::from(target.name.clone()),
                    ));
                    rows.push(field_row(
                        msg::tui_account_delete_field_balance(),
                        Line::from(format!(
                            "{} → {}",
                            format_money_at(&target_balance, target.unit.decimal_places),
                            format_money_at(&new_balance, target.unit.decimal_places)
                        )),
                    ));
                    rows.push(field_row(
                        msg::tui_account_delete_field_moves(),
                        Line::from(Span::styled(
                            format!("{} txns · status kept", account.transaction_count),
                            dim(),
                        )),
                    ));
                    if !account.balance_checks.is_empty() {
                        rows.push(field_row(
                            msg::tui_account_delete_field_loses(),
                            Line::from(Span::styled(
                                format!(
                                    "{} balance checks · they assert a",
                                    account.balance_checks.len()
                                ),
                                dim(),
                            )),
                        ));
                        rows.push(plain_row_indented(Line::from(Span::styled(
                            "balance this account no longer has",
                            dim(),
                        ))));
                    }
                }
                None => {
                    rows.push(plain_row(Line::from(Span::styled(
                        "after — resolves once a same-unit account is typed above",
                        dim(),
                    ))));
                }
            }
            rows.push(blank_row());
        } else {
            rows.push(blank_row());
        }

        rows.push(field_row(
            msg::tui_account_delete_field_confirm(),
            text_field_value(&self.confirm_input, self.focus == Field::Confirm),
        ));
        rows.push(plain_row(Line::from(Span::styled(
            "deletion is permanent and not synced back",
            Style::default().fg(ACCENT),
        ))));
        rows.push(plain_row(Line::from(
            "^a deactivate instead — keeps everything readable",
        )));

        rows
    }
}

fn dim() -> Style {
    Style::default().add_modifier(Modifier::DIM)
}

fn text_field_value(value: &str, focused: bool) -> Line<'static> {
    let mut spans = vec![Span::raw(value.to_string())];
    if focused {
        spans.push(Span::styled("\u{258c}", Style::default().fg(ACCENT)));
    }
    Line::from(spans)
}

fn field_row<'a>(label: String, value: Line<'a>) -> RowRenderer<'a> {
    let value = value.clone();
    Box::new(move |frame, area| {
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(LABEL_WIDTH), Constraint::Min(0)])
            .split(area);
        frame.render_widget(Paragraph::new(Span::styled(&label, dim())), columns[0]);
        frame.render_widget(Paragraph::new(value.clone()), columns[1]);
    })
}

/// A full-width line with no label column at all (section headings, accent/plain statements).
fn plain_row<'a>(text: Line<'a>) -> RowRenderer<'a> {
    Box::new(move |frame, area| {
        frame.render_widget(Paragraph::new(text.clone()), area);
    })
}

/// A continuation line indented to align under the value column — the second half of a
/// two-line `loses` row, whose first line already carries the `loses` label.
fn plain_row_indented<'a>(text: Line<'a>) -> RowRenderer<'a> {
    Box::new(move |frame, area| {
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(LABEL_WIDTH), Constraint::Min(0)])
            .split(area);
        frame.render_widget(Paragraph::new(text.clone()), columns[1]);
    })
}

fn blank_row<'a>() -> RowRenderer<'a> {
    Box::new(|_frame, _area| {})
}

fn render_title(frame: &mut Frame, area: Rect, name: &str) {
    let tag = ":acct delete";
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(tag.chars().count() as u16),
        ])
        .split(area);
    frame.render_widget(
        Paragraph::new(format!("{} {name}", msg::tui_account_delete_title())),
        columns[0],
    );
    frame.render_widget(
        Paragraph::new(Span::styled(tag, dim())).alignment(Alignment::Right),
        columns[1],
    );
}

fn render_footer_hints(frame: &mut Frame, area: Rect) {
    let hints = vec![
        ("tab", msg::tui_account_delete_help_tab()),
        ("^s", msg::tui_account_delete_help_delete()),
        ("^a", msg::tui_account_delete_help_deactivate()),
        ("esc", msg::tui_account_delete_help_cancel()),
    ];
    let key_style = Style::default().add_modifier(Modifier::BOLD);
    let label_style = dim();

    let mut spans = Vec::with_capacity(hints.len() * 3);
    for (index, (key, label)) in hints.iter().enumerate() {
        if index > 0 {
            spans.push(Span::raw("  "));
        }
        spans.push(Span::styled(*key, key_style));
        spans.push(Span::raw(" "));
        spans.push(Span::styled(label.as_str(), label_style));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn popup_rect(area: Rect, content_rows: u16) -> Rect {
    let width = POPUP_WIDTH.min(area.width);
    let x = area.x + (area.width.saturating_sub(width)) / 2;

    let height = (content_rows + 2).min(area.height);
    let anchor_y = area.y + (area.height / 6).max(1);
    let y = anchor_y.min(area.y + area.height.saturating_sub(height));

    Rect {
        x,
        y,
        width,
        height,
    }
}

/// Formats `value` at `decimal_places`, grouped by the Locale.
fn format_money_at(value: &Money, decimal_places: i64) -> String {
    crate::format::money(value, decimal_places)
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;
    use crate::account::AccountFixture;

    fn find_id(store: &AccountFixture, name: &str) -> RowID {
        store
            .accounts()
            .iter()
            .find(|account| account.name == name)
            .unwrap_or_else(|| panic!("fixture should seed an account named {name}"))
            .id
    }

    fn render(popup: &DeleteAccountPopup, store: &dyn AccountStore) -> String {
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
    fn new_on_an_empty_account_skips_the_transfer_step() {
        let store = AccountFixture::new();
        let wallet = find_id(&store, "Wallet");
        let popup = DeleteAccountPopup::new(&store, wallet);
        assert!(!popup.needs_transfer);
        assert_eq!(popup.focus, Field::Confirm);
    }

    #[test]
    fn new_on_a_non_empty_account_starts_on_move_to() {
        let store = AccountFixture::new();
        let everyday = find_id(&store, "Everyday Spending");
        let popup = DeleteAccountPopup::new(&store, everyday);
        assert!(popup.needs_transfer);
        assert_eq!(popup.focus, Field::MoveTo);
    }

    #[test]
    fn renders_without_panicking_for_both_empty_and_non_empty_accounts() {
        let store = AccountFixture::new();
        render(
            &DeleteAccountPopup::new(&store, find_id(&store, "Wallet")),
            &store,
        );
        render(
            &DeleteAccountPopup::new(&store, find_id(&store, "Everyday Spending")),
            &store,
        );
    }

    #[test]
    fn tab_on_move_to_completes_against_a_transfer_candidates_name() {
        let store = AccountFixture::new();
        let everyday = find_id(&store, "Everyday Spending");
        let mut popup = DeleteAccountPopup::new(&store, everyday);
        popup.target_input = "mortgage".to_string();

        popup.tab(&store);
        assert_eq!(popup.target_input, "Mortgage Offset");
        assert_eq!(
            popup.focus,
            Field::MoveTo,
            "completing shouldn't move focus"
        );
    }

    #[test]
    fn tab_cycles_between_move_to_and_confirm_on_a_non_empty_account() {
        let store = AccountFixture::new();
        let everyday = find_id(&store, "Everyday Spending");
        let mut popup = DeleteAccountPopup::new(&store, everyday);
        popup.target_input = "Mortgage Offset".to_string(); // exact match, no completion to apply

        popup.tab(&store);
        assert_eq!(popup.focus, Field::Confirm);
        popup.tab(&store);
        assert_eq!(popup.focus, Field::MoveTo);
    }

    #[test]
    fn tab_is_a_no_op_cycle_on_an_empty_account() {
        let store = AccountFixture::new();
        let wallet = find_id(&store, "Wallet");
        let mut popup = DeleteAccountPopup::new(&store, wallet);
        popup.tab(&store);
        assert_eq!(popup.focus, Field::Confirm);
    }

    #[test]
    fn delete_fields_requires_an_exact_case_sensitive_name_match() {
        let store = AccountFixture::new();
        let wallet = find_id(&store, "Wallet");
        let mut popup = DeleteAccountPopup::new(&store, wallet);
        popup.confirm_input = "wallet".to_string(); // wrong case
        assert_eq!(popup.delete_fields(&store), None);

        popup.confirm_input = "Wallet".to_string();
        assert_eq!(popup.delete_fields(&store), Some((wallet, None)));
    }

    #[test]
    fn delete_fields_on_a_non_empty_account_also_requires_a_resolved_target() {
        let store = AccountFixture::new();
        let everyday = find_id(&store, "Everyday Spending");
        let mut popup = DeleteAccountPopup::new(&store, everyday);
        popup.confirm_input = "Everyday Spending".to_string();
        assert_eq!(
            popup.delete_fields(&store),
            None,
            "no transfer target resolved yet"
        );

        popup.target_input = "Mortgage Offset".to_string();
        let mortgage_offset = find_id(&store, "Mortgage Offset");
        assert_eq!(
            popup.delete_fields(&store),
            Some((everyday, Some(mortgage_offset)))
        );
    }

    #[test]
    fn shows_holds_figures_and_the_refused_delete_them_too_option() {
        let store = AccountFixture::new();
        let everyday = find_id(&store, "Everyday Spending");
        let text = render(&DeleteAccountPopup::new(&store, everyday), &store);

        assert!(text.contains("1284 txns"));
        assert!(text.contains("4,210.65 AUD"));
        assert!(text.contains("12 unreconciled · 8 balance checks"));
        assert!(text.contains("delete them too"));
        assert!(text.contains("refused"));
        assert!(text.contains("move to another account"));
    }

    #[test]
    fn shows_the_after_preview_once_a_target_resolves() {
        let store = AccountFixture::new();
        let everyday = find_id(&store, "Everyday Spending");
        let mut popup = DeleteAccountPopup::new(&store, everyday);
        popup.target_input = "Mortgage Offset".to_string();

        let text = render(&popup, &store);
        assert!(text.contains("after"));
        assert!(text.contains("into"));
        assert!(text.contains("Mortgage Offset"));
        // Mortgage Offset 24 429.50 -> 24 429.50 + 3 210.65 (Everyday Spending's
        // transactions_sum) = 27 640.15.
        assert!(text.contains("24,429.50"));
        assert!(text.contains("27,640.15"));
        assert!(text.contains("1284 txns · status kept"));
        assert!(text.contains("8 balance checks · they assert a"));
        assert!(text.contains("balance this account no longer has"));
    }

    #[test]
    fn shows_a_placeholder_instead_of_after_while_unresolved() {
        let store = AccountFixture::new();
        let everyday = find_id(&store, "Everyday Spending");
        let text = render(&DeleteAccountPopup::new(&store, everyday), &store);
        assert!(text.contains("resolves once a same-unit account is typed above"));
    }

    #[test]
    fn empty_account_shows_no_transfer_section_at_all() {
        let store = AccountFixture::new();
        let wallet = find_id(&store, "Wallet");
        let text = render(&DeleteAccountPopup::new(&store, wallet), &store);

        assert!(!text.contains("move to another account"));
        assert!(!text.contains("candidates"));
        assert!(!text.contains("after"));
    }

    #[test]
    fn shows_the_title_accent_warning_and_footer_with_ctrl_a_not_bare_a() {
        let store = AccountFixture::new();
        let everyday = find_id(&store, "Everyday Spending");
        let text = render(&DeleteAccountPopup::new(&store, everyday), &store);

        assert!(text.contains("delete Everyday Spending"));
        assert!(text.contains(":acct delete"));
        assert!(text.contains("deletion is permanent and not synced back"));
        assert!(text.contains("^a deactivate instead"));
        for key in ["tab", "^s", "^a", "esc"] {
            assert!(text.contains(key), "{key} hint missing");
        }
    }

    #[test]
    fn popup_rect_stays_within_the_terminal_area() {
        let area = Rect::new(0, 0, 96, 30);
        let popup = popup_rect(area, 20);
        assert!(popup.x + popup.width <= area.width);
        assert!(popup.y + popup.height <= area.height);
    }
}
