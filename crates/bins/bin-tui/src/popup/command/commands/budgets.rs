//! The Budgets domain — commands grounded in what `crate::screen::budgets_list`/
//! `budget_detail` actually support (list/new/edit/delete); bindings and wording follow
//! `docs/ux/tui/README.md`'s own `:help` window example for this domain almost verbatim
//! (`g b`, `n`, `e`, `x` for deactivate — realised here as delete, since the real screen has
//! no separate deactivate verb).

use crossterm::event::KeyCode;

use super::{Arg, Chord, Command, CommandId};

pub const COMMANDS: &[Command] = &[
    Command {
        id: CommandId::BudgetList,
        name: "budget list [period]",
        chord: Chord(&[KeyCode::Char('g'), KeyCode::Char('b')]),
        description: crate::msg::tui_command_budget_list_description,
        args: &[Arg {
            placeholder: "[period]",
            preview: crate::msg::tui_command_preview_budget_list,
        }],
    },
    Command {
        id: CommandId::BudgetNew,
        name: "budget new <category> <limit>",
        chord: Chord(&[KeyCode::Char('n')]),
        description: crate::msg::tui_command_budget_new_description,
        args: &[Arg {
            placeholder: "<category>",
            preview: crate::msg::tui_command_preview_budget_new,
        }],
    },
    Command {
        id: CommandId::BudgetEdit,
        name: "budget edit <category> [limit]",
        chord: Chord(&[KeyCode::Char('e')]),
        description: crate::msg::tui_command_budget_edit_description,
        args: &[Arg {
            placeholder: "<category>",
            preview: crate::msg::tui_command_preview_budget_edit,
        }],
    },
    Command {
        id: CommandId::BudgetDelete,
        name: "budget delete <category>",
        chord: Chord(&[KeyCode::Char('d')]),
        description: crate::msg::tui_command_budget_delete_description,
        args: &[Arg {
            placeholder: "<category>",
            preview: crate::msg::tui_command_preview_budget_delete,
        }],
    },
];
