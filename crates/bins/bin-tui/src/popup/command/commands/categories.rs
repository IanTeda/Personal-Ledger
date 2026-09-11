//! The Categories domain — the `:category` grammar `docs/ux/tui/categories/README.md`
//! "Command grammar" specifies (that doc's own text writes it as `:cat`; the command name is
//! `:category`, matching every other domain's own full-word convention — `unit`, `budget`,
//! `payee`, none abbreviated), grounded in what `view::categories::CategoriesView` and its
//! three popups (`crate::popup::category`) actually support now that all of "Categories
//! screen, views and popup" (issue #106) is built. Bindings are the tree screen's own real
//! keys (`g c`, `n`, `e`, `m`, `a`), not the old flat-Categories screen's aspirational ones
//! this domain used to describe (`crate::screen::categories_list` is retired — see "Retire old
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

use super::{Arg, Chord, Command};

pub const COMMANDS: &[Command] = &[
    Command {
        name: "category",
        chord: Chord(&[KeyCode::Char('g'), KeyCode::Char('c')]),
        description: "categories tree — direct vs rollup, spend and transactions",
        args: &[],
    },
    Command {
        name: "category new <name> [parent]",
        chord: Chord(&[KeyCode::Char('n')]),
        description: "add a category — parent defaults to the tree selection",
        args: &[Arg {
            placeholder: "<name>",
            preview: "e.g. dining, groceries, salary",
        }],
    },
    Command {
        name: "category edit <cat>",
        chord: Chord(&[KeyCode::Char('e')]),
        description: "edit the highlighted category",
        args: &[Arg {
            placeholder: "<cat>",
            preview: "groceries · expense · depth 3",
        }],
    },
    Command {
        name: "category move <cat> <parent>",
        chord: Chord(&[KeyCode::Char('m')]),
        description: "move — refuses cycles and a cross-root move with transactions",
        args: &[Arg {
            placeholder: "<parent>",
            preview: "e.g. expenses/food/daily — completes on full paths",
        }],
    },
    Command {
        name: "category rename <cat> <name>",
        chord: Chord::NONE,
        description: "rename — nothing else references a category by name",
        args: &[Arg {
            placeholder: "<name>",
            preview: "e.g. dining",
        }],
    },
    Command {
        name: "category merge <from> <into>",
        chord: Chord::NONE,
        description: "reassigns transactions to <into>, deletes <from>",
        args: &[Arg {
            placeholder: "<into>",
            preview: "the surviving category",
        }],
    },
    Command {
        name: "category archive <cat>",
        chord: Chord(&[KeyCode::Char('a')]),
        description: "active = 0 — keeps every transaction and total",
        args: &[Arg {
            placeholder: "<cat>",
            preview: "the tree selection",
        }],
    },
    Command {
        name: "category tree [root]",
        chord: Chord::NONE,
        description: "prints the subtree — scriptable/pipeable",
        args: &[Arg {
            placeholder: "[root]",
            preview: "defaults to both roots",
        }],
    },
];
