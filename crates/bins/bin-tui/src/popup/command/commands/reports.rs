//! The Reports domain — the retired `screen::reports` was one screen with an internal picker
//! across five real report kinds (Account Balance, Category Total, Payee Total, Budget vs
//! Actual, Balance Check Variance), the shape `crate::view::reports` is to be rebuilt to,
//! rather than `docs/ux/tui/README.md`'s own aspirational
//! `category`/`payee`/`networth`/`variance` sub-verbs, which don't match what's built. Only
//! opening the screen has a global jump (`g r`); a specific report is reached via the
//! screen's own Tab/Left/Right picker, so each has no binding yet.

use crossterm::event::KeyCode;

use super::{Chord, Command, CommandId};

pub const COMMANDS: &[Command] = &[
    Command {
        id: CommandId::ReportList,
        name: "report list",
        chord: Chord(&[KeyCode::Char('g'), KeyCode::Char('r')]),
        description: crate::msg::tui_command_report_list_description,
        args: &[],
    },
    Command {
        id: CommandId::ReportAccountBalance,
        name: "report account-balance",
        chord: Chord::NONE,
        description: crate::msg::tui_command_report_account_balance_description,
        args: &[],
    },
    Command {
        id: CommandId::ReportCategoryTotal,
        name: "report category-total",
        chord: Chord::NONE,
        description: crate::msg::tui_command_report_category_total_description,
        args: &[],
    },
    Command {
        id: CommandId::ReportPayeeTotal,
        name: "report payee-total",
        chord: Chord::NONE,
        description: crate::msg::tui_command_report_payee_total_description,
        args: &[],
    },
    Command {
        id: CommandId::ReportBudgetVariance,
        name: "report budget-variance",
        chord: Chord::NONE,
        description: crate::msg::tui_command_report_budget_variance_description,
        args: &[],
    },
    Command {
        id: CommandId::ReportBalanceCheckVariance,
        name: "report balance-check-variance",
        chord: Chord::NONE,
        description: crate::msg::tui_command_report_balance_check_variance_description,
        args: &[],
    },
];
