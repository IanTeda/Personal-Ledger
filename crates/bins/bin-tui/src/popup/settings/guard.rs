//! The base unit guard (TUI-021): committing a new base unit on Settings › General opens it, since
//! every reported total is converted again when the base changes. It states what changes and asks
//! you to type the new Unit's code; `enter` stays dim until the text matches exactly, and
//! `ctrl p` goes to Settings › Units for the price sources instead. (`p` is the card's key, but a
//! code can hold a `p`, as `gbp` does, so the typed field keeps every plain letter.)
//!
//! The guard hosts `lib-units`' Set base unit Presenter on the Open Ledger's shared ports, so the
//! Interactor, which Units can be the base, and the refusals are `lib-units`' alone: the editor
//! offers the Presenter's choices, opening the guard chooses one, and the typed confirmation is
//! the TUI's one extra step before the Presenter's confirm.
//!
//! Transactions and accounts are counted from the Ledger the Units ports read usage from. The
//! months converted again and the missing prices need the conversion queries, which don't exist
//! yet, so those stay worked examples, each marked `// wireframe:`.

use lib_core::UnitId;
use lib_units::presenter::{SetBaseUnitEvent, SetBaseUnitPresenter};
use lib_units::{SharedPorts, UnitsPorts};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::colours::Colours;
use crate::msg;
use crate::popup::REFERENCE_TERMINAL_WIDTH;
use crate::units::TuiPorts;
use crate::units::router::Router;

/// The popup's width: 78% of the reference terminal, the command popup's.
const POPUP_WIDTH: u16 = REFERENCE_TERMINAL_WIDTH * 78 / 100;

/// The fact label column, the longest label plus a gap.
const LABEL_WIDTH: u16 = 17;

/// Content rows inside the border: title, rule, two prose lines, a gap, five facts, a gap, the
/// 4-row price box, the 3-row confirmation box, the refusal, a rule and the keys.
const CONTENT_ROWS: u16 = 1 + 1 + 2 + 1 + 5 + 1 + 4 + 3 + 1 + 1 + 1;

// wireframe: TUI-021's worked figures, until the conversion queries exist.
const CONVERTED_MONTHS: i64 = 18;
const MISSING_MONTHS: i64 = 4;
const MISSING_RANGE: &str = "mar 25 – jun 25";

/// What `enter` in the guard did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Confirmed {
    /// The typed code doesn't match yet, so nothing was sent.
    NotYet,
    /// The base is now this Unit code.
    Set(String),
    /// The Presenter refused; the guard shows why and stays open.
    Refused,
}

/// The guard on moving the base from `from` to `to`, the Unit codes.
pub struct BaseUnitGuardPopup {
    ports: TuiPorts,
    presenter: SetBaseUnitPresenter,
    from: String,
    to: String,
    typed: String,
    transactions: usize,
    accounts: usize,
    already: usize,
}

impl BaseUnitGuardPopup {
    /// The guard on making the Unit coded `to` the base, chosen on the Presenter. `None` when `to`
    /// is the base already, or isn't one the Presenter offers.
    pub fn open(ports: &TuiPorts, to: &str) -> Option<Self> {
        let shared: SharedPorts = ports.clone();
        let mut presenter = SetBaseUnitPresenter::new(shared.clone());
        let (from, to_id, transactions, accounts, already) = {
            let held = ports.borrow();
            let code = |id: UnitId| held.units().unit(id).map(|unit| unit.code);
            let from = code(held.base_unit().base_unit())?;
            let to_id = presenter
                .view_model()
                .base
                .options
                .iter()
                .map(|choice| choice.value)
                .find(|id| code(*id).as_deref() == Some(to))?;
            let ledger = held.ledger();
            let accounts = ledger.accounts();
            let already = accounts
                .iter()
                .filter(|account| account.unit == to_id)
                .count();
            (
                from,
                to_id,
                ledger.transactions().len(),
                accounts.len(),
                already,
            )
        };
        if from == to {
            return None;
        }
        let mut toasts = Vec::new();
        presenter.handle(
            SetBaseUnitEvent::Choose(to_id),
            &mut Router::new(shared, &mut toasts),
        );
        Some(Self {
            ports: ports.clone(),
            presenter,
            from,
            to: to.to_string(),
            typed: String::new(),
            transactions,
            accounts,
            already,
        })
    }

    pub fn push(&mut self, ch: char) {
        self.typed.push(ch);
    }

    pub fn backspace(&mut self) {
        self.typed.pop();
    }

    /// Whether the typed text is the new code, exactly.
    pub fn matches(&self) -> bool {
        self.typed == self.to
    }

    /// Sends the Presenter's confirm, once the typed code matches, and commits what it wrote.
    pub fn confirm(&mut self) -> Confirmed {
        if !self.matches() {
            return Confirmed::NotYet;
        }
        let mut toasts = Vec::new();
        let closed = {
            let mut router = Router::new(self.ports.clone(), &mut toasts);
            self.presenter
                .handle(SetBaseUnitEvent::Confirm, &mut router);
            router.closed.take()
        };
        self.ports.borrow_mut().commit();
        match closed {
            Some(Some(_)) => Confirmed::Set(self.to.clone()),
            _ => Confirmed::Refused,
        }
    }

    /// The guard, anchored in the top third of `area`, the whole frame.
    pub fn render(&self, frame: &mut Frame<'_>, area: Rect, c: &Colours) {
        let popup = popup_rect(area);
        frame.render_widget(Clear, popup);
        let block = Block::bordered();
        let inner = block.inner(popup);
        frame.render_widget(block, popup);

        let [
            title,
            rule,
            prose,
            _,
            facts,
            _,
            prices,
            confirm,
            refusal,
            rule_2,
            keys,
        ] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Length(5),
            Constraint::Length(1),
            Constraint::Length(4),
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .areas(inner);

        self.render_title(frame, title, c);
        frame.render_widget(Block::new().borders(Borders::BOTTOM), rule);
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(msg::tui_settings_guard_prose_units()),
                Line::from(msg::tui_settings_guard_prose_totals(&self.to)),
            ]),
            prose,
        );
        self.render_facts(frame, facts, c);
        render_prices(frame, prices, c);
        self.render_confirmation(frame, confirm, c);
        if let Some(reason) = self.presenter.view_model().refusal {
            frame.render_widget(Paragraph::new(Span::styled(reason, c.accent())), refusal);
        }
        frame.render_widget(Block::new().borders(Borders::BOTTOM), rule_2);
        self.render_keys(frame, keys, c);
    }

    /// `base unit  aud → btc`, with the equivalent `:set base btc` dim on the right.
    fn render_title(&self, frame: &mut Frame<'_>, area: Rect, c: &Colours) {
        let command = format!(":set base {}", self.to);
        let width = u16::try_from(command.chars().count()).unwrap_or(0);
        let [left, right] =
            Layout::horizontal([Constraint::Min(0), Constraint::Length(width)]).areas(area);
        let bold = Style::default().add_modifier(Modifier::BOLD);
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(msg::tui_settings_guard_title(), bold),
                Span::raw("  "),
                Span::raw(msg::tui_settings_guard_change(&self.from, &self.to)),
            ])),
            left,
        );
        frame.render_widget(Paragraph::new(Span::styled(command, c.muted())), right);
    }

    fn render_facts(&self, frame: &mut Frame<'_>, area: Rect, c: &Colours) {
        let count = |n: usize| crate::format::count(i64::try_from(n).unwrap_or(i64::MAX));
        let facts = [
            (
                msg::tui_settings_guard_transactions(),
                msg::tui_settings_guard_transactions_value(&count(self.transactions)),
                false,
            ),
            (
                msg::tui_settings_guard_accounts(),
                msg::tui_settings_guard_accounts_value(
                    &count(self.accounts),
                    &count(self.already),
                    &self.to,
                ),
                false,
            ),
            (
                msg::tui_settings_guard_converted(),
                msg::tui_settings_guard_converted_value(CONVERTED_MONTHS),
                false,
            ),
            (
                msg::tui_settings_guard_missing(),
                msg::tui_settings_guard_missing_value(MISSING_MONTHS, MISSING_RANGE),
                true,
            ),
            (
                msg::tui_settings_guard_then(),
                msg::tui_settings_guard_then_value(),
                false,
            ),
        ];
        let rows = Layout::vertical([Constraint::Length(1); 5]).split(area);
        for ((label, value, accent), row) in facts.into_iter().zip(rows.iter()) {
            let [label_area, value_area] =
                Layout::horizontal([Constraint::Length(LABEL_WIDTH), Constraint::Min(0)])
                    .areas(*row);
            frame.render_widget(Paragraph::new(Span::styled(label, c.muted())), label_area);
            let style = if accent { c.accent() } else { Style::default() };
            frame.render_widget(Paragraph::new(Span::styled(value, style)), value_area);
        }
    }

    /// The typed confirmation, in a `negative`-bordered box as the typed deletes are.
    fn render_confirmation(&self, frame: &mut Frame<'_>, area: Rect, c: &Colours) {
        let block = Block::bordered().border_style(c.negative());
        let inner = block.inner(area);
        frame.render_widget(block, area);
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(msg::tui_settings_guard_type(&self.to), c.muted()),
                Span::raw("  "),
                Span::raw(self.typed.clone()),
                Span::styled("▌", c.cursor()),
            ])),
            inner,
        );
    }

    /// The guard's keys, `enter` dim until the typed code matches.
    fn render_keys(&self, frame: &mut Frame<'_>, area: Rect, c: &Colours) {
        let bold = Style::default().add_modifier(Modifier::BOLD);
        let enter = if self.matches() { bold } else { c.muted() };
        let enter_label = if self.matches() {
            Style::default()
        } else {
            c.muted()
        };
        let spans = vec![
            Span::styled("enter", enter),
            Span::styled(
                format!(" {}", msg::tui_settings_guard_hint_change()),
                enter_label,
            ),
            Span::raw(" · "),
            Span::styled("ctrl p", bold),
            Span::raw(format!(" {}", msg::tui_settings_guard_hint_price_sources())),
            Span::raw(" · "),
            Span::styled("esc", bold),
            Span::raw(format!(" {}", msg::tui_settings_guard_hint_cancel())),
        ];
        frame.render_widget(Paragraph::new(Line::from(spans)), area);
    }
}

/// The box naming the gap the guard exists to catch, with the fix offered as a choice.
fn render_prices(frame: &mut Frame<'_>, area: Rect, c: &Colours) {
    let block = Block::bordered();
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let bold = Style::default().add_modifier(Modifier::BOLD);
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(msg::tui_settings_guard_gaps(MISSING_MONTHS)),
            Line::from(vec![
                Span::styled("ctrl p", bold),
                Span::raw(" "),
                Span::styled(msg::tui_settings_guard_price_sources(), c.muted()),
            ]),
        ]),
        inner,
    );
}

/// The popup's `Rect`, anchored in the top third of `area` and clamped to fit.
fn popup_rect(area: Rect) -> Rect {
    let width = POPUP_WIDTH.min(area.width);
    let height = (CONTENT_ROWS + 2).min(area.height);
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = (area.y + (area.height / 6).max(1)).min(area.y + area.height.saturating_sub(height));
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

    fn stub() -> TuiPorts {
        crate::units::UnitsSetup::stub(&lib_config::KeyBindingConfig::default()).ports
    }

    fn render(guard: &BaseUnitGuardPopup) -> String {
        let mut terminal =
            Terminal::new(TestBackend::new(96, 30)).expect("test backend should initialise");
        terminal
            .draw(|frame| guard.render(frame, frame.area(), &Colours::default()))
            .expect("drawing the guard should not error");
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
    fn it_opens_only_on_a_new_base_the_presenter_offers() {
        let ports = stub();
        assert!(
            BaseUnitGuardPopup::open(&ports, "aud").is_none(),
            "already the base"
        );
        assert!(
            BaseUnitGuardPopup::open(&ports, "zzz").is_none(),
            "not a Unit"
        );
        assert!(BaseUnitGuardPopup::open(&ports, "btc").is_some());
    }

    #[test]
    fn enter_does_nothing_until_the_typed_code_matches_then_sets_the_base() {
        let ports = stub();
        let mut guard = BaseUnitGuardPopup::open(&ports, "btc").expect("btc can be the base");
        guard.push('b');
        guard.push('t');
        assert_eq!(guard.confirm(), Confirmed::NotYet);
        guard.push('x');
        guard.backspace();
        guard.push('c');
        assert!(guard.matches());
        assert_eq!(guard.confirm(), Confirmed::Set("btc".into()));
        let base = ports.borrow().base_unit().base_unit();
        assert_eq!(base, UnitId::SEED_BTC);
    }

    #[test]
    fn enter_is_drawn_dim_until_the_typed_code_matches() {
        crate::locale::init_for_tests();
        let c = Colours::default();
        let enter_style = |guard: &BaseUnitGuardPopup| {
            let mut terminal =
                Terminal::new(TestBackend::new(96, 30)).expect("test backend should initialise");
            terminal
                .draw(|frame| guard.render(frame, frame.area(), &c))
                .expect("drawing the guard should not error");
            let buffer = terminal.backend().buffer().clone();
            let (x, y) = (0..buffer.area.height)
                .find_map(|y| {
                    let row: String = (0..buffer.area.width)
                        .map(|x| buffer[(x, y)].symbol())
                        .collect();
                    row.find("enter change")
                        .map(|byte| (row[..byte].chars().count(), y))
                })
                .expect("the keys row is drawn");
            buffer[(u16::try_from(x).unwrap_or_default(), y)].style()
        };
        let mut guard = BaseUnitGuardPopup::open(&stub(), "btc").expect("btc can be the base");
        guard.push('b');
        assert_eq!(enter_style(&guard).fg, c.muted().fg);
        guard.push('t');
        guard.push('c');
        assert_ne!(enter_style(&guard).fg, c.muted().fg);
    }

    #[test]
    fn it_states_the_change_and_names_the_code_to_type() {
        crate::locale::init_for_tests();
        let guard = BaseUnitGuardPopup::open(&stub(), "btc").expect("btc can be the base");
        let text = render(&guard);
        for needle in [
            "base unit  aud → btc",
            ":set base btc",
            "converted again into btc",
            "· unchanged",
            "already in btc",
            "missing prices",
            "type btc to confirm",
            "enter change base unit",
            "ctrl p price sources",
        ] {
            assert!(text.contains(needle), "{needle} missing:\n{text}");
        }
    }
}
