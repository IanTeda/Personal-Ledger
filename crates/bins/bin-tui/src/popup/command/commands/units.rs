//! The Units domain — commands grounded in what `crate::screen::units_list`/`unit_detail`
//! actually support (list/new/edit/delete); bindings are `docs/ux/tui/README.md`'s own
//! documented scheme for this domain (`g u`, and `n` for `:unit new` verbatim from its
//! `:help` window example).

use crossterm::event::KeyCode;

use super::{Arg, Chord, Command, CommandId};

pub const COMMANDS: &[Command] = &[
    Command {
        id: CommandId::Unit,
        name: "unit",
        chord: Chord(&[KeyCode::Char('g'), KeyCode::Char('u')]),
        description: crate::msg::tui_command_unit_description,
        args: &[],
    },
    Command {
        id: CommandId::UnitNew,
        name: "unit new <code> <type>",
        chord: Chord(&[KeyCode::Char('n')]),
        description: crate::msg::tui_command_unit_new_description,
        args: &[Arg {
            placeholder: "<code>",
            preview: crate::msg::tui_command_preview_unit_new,
        }],
    },
    Command {
        id: CommandId::UnitEdit,
        name: "unit edit <code>",
        chord: Chord(&[KeyCode::Char('e')]),
        description: crate::msg::tui_command_unit_edit_description,
        args: &[Arg {
            placeholder: "<code>",
            preview: crate::msg::tui_command_preview_unit_edit,
        }],
    },
    Command {
        id: CommandId::UnitDelete,
        name: "unit delete <code>",
        chord: Chord(&[KeyCode::Char('d')]),
        description: crate::msg::tui_command_unit_delete_description,
        args: &[Arg {
            placeholder: "<code>",
            preview: crate::msg::tui_command_preview_unit_delete,
        }],
    },
];
