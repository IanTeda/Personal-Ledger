//! The Categories domain — the `:category` grammar `docs/ux/tui/categories/README.md`
//! "Command grammar" specifies (that doc's own text writes it as `:cat`; the command name is
//! `:category`, matching every other domain's own full-word convention — `unit`, `budget`,
//! `payee`, none abbreviated), grounded in what `view::categories::CategoriesView` and its
//! three popups (`crate::popup::category`) actually support now that all of "Categories
//! screen, views and popup" (issue #106) is built. Bindings are the tree screen's own real
//! keys (`g c`, `n`, `e`, `m`, `a`), not the old flat-Categories screen's aspirational ones
//! this domain used to describe (`screen::categories_list` is retired — see "Retire old
//! Screen-architecture Categories code").
//!
//! **Dispatch is narrower than the grammar's own argument lists suggest.** The command popup
//! has no real argument-typing (`Command::args` is a fixed preview string, never resolved
//! against what's typed — `CommandPopup`'s own module doc), so every entry here that reaches
//! real behaviour does it against whatever's currently selected on the Categories tree (the
//! same convention the tree's own `n`/`e`/`m`/`a` keys already use), never against a typed
//! `<cat>`/`<parent>`/`<name>`. `category new`/`category edit`/`category move` open their popup
//! prefilled from the selection; `category archive` applies immediately, also to the selection.
//! `category rename` (needs a typed new name), `category merge` (not yet designed at all —
//! matches the edit popup's own `X` stub) and `category tree` (needs a results surface to
//! print to, which doesn't exist) have no path to real dispatch yet and fall through to the
//! existing "not yet built" message — genuinely not buildable from this ticket alone, not an
//! oversight.

use crossterm::event::KeyCode;

use super::{Arg, Chord, Command, CommandId};

pub const COMMANDS: &[Command] = &[
    Command {
        id: CommandId::Category,
        name: "category",
        chord: Chord(&[KeyCode::Char('g'), KeyCode::Char('c')]),
        description: crate::msg::tui_command_category_description,
        args: &[],
    },
    Command {
        id: CommandId::CategoryNew,
        name: "category new <name> [parent]",
        chord: Chord(&[KeyCode::Char('n')]),
        description: crate::msg::tui_command_category_new_description,
        args: &[Arg {
            placeholder: "<name>",
            preview: crate::msg::tui_command_preview_category_new,
        }],
    },
    Command {
        id: CommandId::CategoryEdit,
        name: "category edit <cat>",
        chord: Chord(&[KeyCode::Char('e')]),
        description: crate::msg::tui_command_category_edit_description,
        args: &[Arg {
            placeholder: "<cat>",
            preview: crate::msg::tui_command_preview_category_edit,
        }],
    },
    Command {
        id: CommandId::CategoryMove,
        name: "category move <cat> <parent>",
        chord: Chord(&[KeyCode::Char('m')]),
        description: crate::msg::tui_command_category_move_description,
        args: &[Arg {
            placeholder: "<parent>",
            preview: crate::msg::tui_command_preview_category_move,
        }],
    },
    Command {
        id: CommandId::CategoryRename,
        name: "category rename <cat> <name>",
        chord: Chord::NONE,
        description: crate::msg::tui_command_category_rename_description,
        args: &[Arg {
            placeholder: "<name>",
            preview: crate::msg::tui_command_preview_category_rename,
        }],
    },
    Command {
        id: CommandId::CategoryMerge,
        name: "category merge <from> <into>",
        chord: Chord::NONE,
        description: merge_description,
        args: &[Arg {
            placeholder: "<into>",
            preview: crate::msg::tui_command_preview_category_merge,
        }],
    },
    Command {
        id: CommandId::CategoryArchive,
        name: "category archive <cat>",
        chord: Chord(&[KeyCode::Char('a')]),
        description: crate::msg::tui_command_category_archive_description,
        args: &[Arg {
            placeholder: "<cat>",
            preview: crate::msg::tui_command_preview_tree_selection,
        }],
    },
    Command {
        id: CommandId::CategoryTree,
        name: "category tree [root]",
        chord: Chord::NONE,
        description: crate::msg::tui_command_category_tree_description,
        args: &[Arg {
            placeholder: "[root]",
            preview: crate::msg::tui_command_preview_category_tree,
        }],
    },
];

/// `:category merge`'s description names both of its own argument tokens, which stay stable
/// English exactly as its usage line spells them.
fn merge_description() -> String {
    crate::msg::tui_command_category_merge_description("<into>", "<from>")
}
