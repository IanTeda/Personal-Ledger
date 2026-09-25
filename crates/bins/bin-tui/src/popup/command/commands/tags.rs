//! The Tags domain — the `:tag` grammar (issue #130, "Tags: :tag command grammar"), grounded
//! in what `view::tags::TagsView` and its two popups (`crate::popup::tag`) actually support,
//! now that all of "Tags catalog screen, views and popup" (issue #125) is built. Bindings are
//! the list's own real keys (`n`, `e`, `d`) — there is no `g`-jump chord yet for `tag` itself
//! (`Action::OpenTags`'s own doc explains why: the map's destination never named one, only
//! this command grammar).
//!
//! **Dispatch is narrower than the grammar's own argument lists suggest**, for exactly the
//! reason `commands::accounts`'s own module doc gives: the command popup has no real
//! argument-typing, so every entry here that reaches real behaviour does it against whatever's
//! currently selected on the Tags list (`View::tag_selection`), never against a typed `<tag>`.
//! `tag new` opens the new popup blank (no selection prerequisite — guarded by `View::
//! tag_store` being `Some` instead, mirroring `account new`); `tag edit` opens the edit popup
//! for the selection; `tag off`/`tag on` apply immediately via `Action::SetTagActive`, the
//! exact same `TagStore::set_active` path a future bare key would use (Tags has none yet); `tag
//! delete` arms the same lightweight delete confirm the list's own bare `d` key does
//! (`Action::ArmTagDelete`) — it still needs `y` on the Tags view itself to actually delete,
//! exactly like the keybinding; this command never deletes on its own.

use crossterm::event::KeyCode;

use super::{
    Arg, Chord, Command, CommandId, list_selection_inactive_preview, list_selection_preview,
    off_description,
};

pub const COMMANDS: &[Command] = &[
    Command {
        id: CommandId::Tag,
        name: "tag",
        chord: Chord::NONE,
        description: crate::msg::tui_command_tag_description,
        args: &[],
    },
    Command {
        id: CommandId::TagNew,
        name: "tag new <name>",
        chord: Chord(&[KeyCode::Char('n')]),
        description: crate::msg::tui_command_tag_new_description,
        args: &[Arg {
            placeholder: "<name>",
            preview: crate::msg::tui_command_preview_tag_new,
        }],
    },
    Command {
        id: CommandId::TagEdit,
        name: "tag edit <tag>",
        chord: Chord(&[KeyCode::Char('e')]),
        description: crate::msg::tui_command_tag_edit_description,
        args: &[Arg {
            placeholder: "<tag>",
            preview: crate::msg::tui_command_preview_tag_edit,
        }],
    },
    Command {
        id: CommandId::TagOff,
        name: "tag off <tag>",
        chord: Chord::NONE,
        description: off_description,
        args: &[Arg {
            placeholder: "<tag>",
            preview: list_selection_preview,
        }],
    },
    Command {
        id: CommandId::TagOn,
        name: "tag on <tag>",
        chord: Chord::NONE,
        description: on_description,
        args: &[Arg {
            placeholder: "<tag>",
            preview: list_selection_inactive_preview,
        }],
    },
    Command {
        id: CommandId::TagDelete,
        name: "tag delete <tag>",
        chord: Chord(&[KeyCode::Char('d')]),
        description: delete_description,
        args: &[Arg {
            placeholder: "<tag>",
            preview: list_selection_preview,
        }],
    },
];

/// `:tag on`'s description names the command it reverses, which stays a stable English id.
fn on_description() -> String {
    crate::msg::tui_command_on_description("tag off")
}

/// `:tag delete`'s description names the key that confirms the armed delete.
fn delete_description() -> String {
    crate::msg::tui_command_tag_delete_description(TAG_DELETE_CONFIRM_KEY)
}

/// The key the Tags list confirms an armed delete with.
const TAG_DELETE_CONFIRM_KEY: &str = "y";
