//! The table feasibility demo (FC-TUI-003): renders synthetic transaction rows via ratatui's
//! built-in `Table` widget, proving table rendering works.

use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Color, Style},
    text::Span,
    widgets::{Block, Cell, Row, Table},
};

use crate::{action::Action, screen::Screen};

/// One dummy row, in the shape of a Personal Ledger Transaction (see FR.16 in
/// `docs/product-requirements.md`): a date, payee, category, amount, and Transaction Status.
struct Transaction {
    date: &'static str,
    payee: &'static str,
    category: &'static str,
    amount: f64,
    status: &'static str,
}

/// Demonstrates a table of synthetic transaction data.
pub struct TableScreen {
    transactions: Vec<Transaction>,
}

impl TableScreen {
    /// Builds the screen with dummy transaction rows spanning every Transaction Status
    /// (Open, Cleared, Reconciled) and both income and expense amounts.
    pub fn new() -> Self {
        let transactions = vec![
            Transaction {
                date: "2026-08-01",
                payee: "Employer Pty Ltd",
                category: "Salary",
                amount: 3_200.00,
                status: "Reconciled",
            },
            Transaction {
                date: "2026-08-02",
                payee: "Woolworths",
                category: "Groceries",
                amount: -84.32,
                status: "Reconciled",
            },
            Transaction {
                date: "2026-08-04",
                payee: "Landlord",
                category: "Rent",
                amount: -1_200.00,
                status: "Reconciled",
            },
            Transaction {
                date: "2026-08-07",
                payee: "Energy Australia",
                category: "Utilities",
                amount: -145.60,
                status: "Cleared",
            },
            Transaction {
                date: "2026-08-10",
                payee: "Woolworths",
                category: "Groceries",
                amount: -62.15,
                status: "Cleared",
            },
            Transaction {
                date: "2026-08-12",
                payee: "Netflix",
                category: "Entertainment",
                amount: -22.99,
                status: "Cleared",
            },
            Transaction {
                date: "2026-08-14",
                payee: "Opal Transport",
                category: "Transport",
                amount: -38.40,
                status: "Cleared",
            },
            Transaction {
                date: "2026-08-15",
                payee: "Employer Pty Ltd",
                category: "Salary",
                amount: 3_200.00,
                status: "Open",
            },
            Transaction {
                date: "2026-08-17",
                payee: "Coles",
                category: "Groceries",
                amount: -71.88,
                status: "Open",
            },
            Transaction {
                date: "2026-08-19",
                payee: "Unknown transfer",
                category: "Uncategorised",
                amount: -250.00,
                status: "Open",
            },
        ];
        Self { transactions }
    }
}

impl Screen for TableScreen {
    fn update(&mut self, _action: &Action) {
        // Static dummy data — nothing to react to yet.
    }

    fn title(&self) -> &'static str {
        "Table"
    }

    fn view(&self, frame: &mut Frame, area: Rect) {
        let header = Row::new(["Date", "Payee", "Category", "Amount", "Status"])
            .style(Style::default().fg(Color::Yellow));

        let rows = self.transactions.iter().map(|transaction| {
            let amount_color = if transaction.amount >= 0.0 {
                Color::Green
            } else {
                Color::Red
            };
            let status_color = match transaction.status {
                "Reconciled" => Color::Blue,
                "Cleared" => Color::Green,
                _ => Color::Gray,
            };
            Row::new([
                Cell::from(transaction.date),
                Cell::from(transaction.payee),
                Cell::from(transaction.category),
                Cell::from(Span::styled(
                    format!("{:>10.2}", transaction.amount),
                    Style::default().fg(amount_color),
                )),
                Cell::from(Span::styled(
                    transaction.status,
                    Style::default().fg(status_color),
                )),
            ])
        });

        let widths = [
            Constraint::Length(10),
            Constraint::Length(20),
            Constraint::Length(16),
            Constraint::Length(12),
            Constraint::Length(12),
        ];

        let table = Table::new(rows, widths)
            .header(header)
            .column_spacing(1)
            .block(Block::bordered().title(" Table demo — FC-TUI-003 (dummy data) "));

        frame.render_widget(table, area);
    }
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;

    #[test]
    fn renders_without_panicking() {
        let screen = TableScreen::new();
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");

        terminal
            .draw(|frame| screen.view(frame, frame.area()))
            .expect("rendering the table should not error");
    }
}
