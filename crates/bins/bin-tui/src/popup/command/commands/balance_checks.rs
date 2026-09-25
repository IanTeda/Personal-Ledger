//! The Balance Checks domain — commands grounded in what
//! `crate::screen::balance_checks_list`/`balance_check_detail` actually support
//! (list/new/edit/delete/import); `docs/ux/tui/README.md` calls this domain "reconcile" in
//! its navigation jump (`g k`) but "check" in its command grammar — the real screen's own
//! `i` import key (CSV import, FR.33) has no README precedent, so it's carried over as-is.

use crossterm::event::KeyCode;

use super::{Arg, Chord, Command, CommandId};

pub const COMMANDS: &[Command] = &[
    Command {
        id: CommandId::CheckList,
        name: "check list",
        chord: Chord(&[KeyCode::Char('g'), KeyCode::Char('k')]),
        description: crate::msg::tui_command_check_list_description,
        args: &[],
    },
    Command {
        id: CommandId::CheckNew,
        name: "check new <account> <date> <balance>",
        chord: Chord(&[KeyCode::Char('n')]),
        description: crate::msg::tui_command_check_new_description,
        args: &[Arg {
            placeholder: "<account>",
            preview: crate::msg::tui_command_preview_check_new,
        }],
    },
    Command {
        id: CommandId::CheckEdit,
        name: "check edit",
        chord: Chord(&[KeyCode::Char('e')]),
        description: crate::msg::tui_command_check_edit_description,
        args: &[],
    },
    Command {
        id: CommandId::CheckDelete,
        name: "check delete",
        chord: Chord(&[KeyCode::Char('d')]),
        description: crate::msg::tui_command_check_delete_description,
        args: &[],
    },
    Command {
        id: CommandId::CheckImport,
        name: "check import <path.csv>",
        chord: Chord(&[KeyCode::Char('i')]),
        description: crate::msg::tui_command_check_import_description,
        args: &[Arg {
            placeholder: "<path.csv>",
            preview: crate::msg::tui_command_preview_check_import,
        }],
    },
];
