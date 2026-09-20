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

use super::{Arg, Chord, Command, CommandId};

pub const COMMANDS: &[Command] = &[
    Command {
        id: CommandId::Tag,
        name: "tag",
        chord: Chord::NONE,
        description: "flat alphabetical list, summary box, lightweight delete",
        args: &[],
    },
    Command {
        id: CommandId::TagNew,
        name: "tag new <name>",
        chord: Chord(&[KeyCode::Char('n')]),
        description: "add a tag — opens the new popup, blank",
        args: &[Arg {
            placeholder: "<name>",
            preview: "e.g. Japan Trip 2026, Tax Deductible",
        }],
    },
    Command {
        id: CommandId::TagEdit,
        name: "tag edit <tag>",
        chord: Chord(&[KeyCode::Char('e')]),
        description: "edit the highlighted tag",
        args: &[Arg {
            placeholder: "<tag>",
            preview: "Japan Trip 2026 · active · 7 transactions",
        }],
    },
    Command {
        id: CommandId::TagOff,
        name: "tag off <tag>",
        chord: Chord::NONE,
        description: "is_active = 0 — hides it from the list unless za",
        args: &[Arg {
            placeholder: "<tag>",
            preview: "the list selection",
        }],
    },
    Command {
        id: CommandId::TagOn,
        name: "tag on <tag>",
        chord: Chord::NONE,
        description: "is_active = 1 — reverses tag off",
        args: &[Arg {
            placeholder: "<tag>",
            preview: "the list selection, with za held to see it",
        }],
    },
    Command {
        id: CommandId::TagDelete,
        name: "tag delete <tag>",
        chord: Chord(&[KeyCode::Char('d')]),
        description: "arms the lightweight delete confirm — y on the list confirms",
        args: &[Arg {
            placeholder: "<tag>",
            preview: "the list selection",
        }],
    },
];
