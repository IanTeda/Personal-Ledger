//! The Categories domain — the `:cat` grammar `docs/ux/tui/categories/README.md` "Command
//! grammar" specifies, grounded in what `view::categories::CategoriesView` and its three
//! popups (`crate::popup::category`) actually support now that all of "Categories screen,
//! views and popup" (issue #106) is built. Bindings are the tree screen's own real keys
//! (`g c`, `n`, `e`, `m`, `a`), not the old flat-Categories screen's aspirational ones this
//! domain used to describe (`crate::screen::categories_list` is retired — see "Retire old
//! Screen-architecture Categories code").
//!
//! **Dispatch is narrower than the grammar's own argument lists suggest.** The command popup
//! has no real argument-typing (`Command::args` is a fixed preview string, never resolved
//! against what's typed — `CommandPopup`'s own module doc), so every entry here that reaches
//! real behaviour does it against whatever's currently selected on the Categories tree (the
//! same convention the tree's own `n`/`e`/`m`/`a` keys already use), never against a typed
//! `<cat>`/`<parent>`/`<name>`. `cat new`/`cat edit`/`cat move` open their popup prefilled from
//! the selection; `cat archive` applies immediately, also to the selection. `cat rename` (needs
//! a typed new name), `cat merge` (not yet designed at all — matches the edit popup's own `X`
//! stub) and `cat tree` (needs a results surface to print to, which doesn't exist) have no path
//! to real dispatch yet and fall through to the existing "not yet built" message — genuinely
//! not buildable from this ticket alone, not an oversight.

use crossterm::event::KeyCode;

use super::{Arg, Chord, Command};

pub const COMMANDS: &[Command] = &[
    Command {
        name: "cat",
        chord: Chord(&[KeyCode::Char('g'), KeyCode::Char('c')]),
        description: "categories tree — direct vs rollup, spend and transactions",
        args: &[],
    },
    Command {
        name: "cat new <name> [parent]",
        chord: Chord(&[KeyCode::Char('n')]),
        description: "add a category — parent defaults to the tree selection",
        args: &[Arg {
            placeholder: "<name>",
            preview: "e.g. dining, groceries, salary",
        }],
    },
    Command {
        name: "cat edit <cat>",
        chord: Chord(&[KeyCode::Char('e')]),
        description: "edit the highlighted category",
        args: &[Arg {
            placeholder: "<cat>",
            preview: "groceries · expense · depth 3",
        }],
    },
    Command {
        name: "cat move <cat> <parent>",
        chord: Chord(&[KeyCode::Char('m')]),
        description: "move — refuses cycles and a cross-root move with transactions",
        args: &[Arg {
            placeholder: "<parent>",
            preview: "e.g. expenses/food/daily — completes on full paths",
        }],
    },
    Command {
        name: "cat rename <cat> <name>",
        chord: Chord::NONE,
        description: "rename — nothing else references a category by name",
        args: &[Arg {
            placeholder: "<name>",
            preview: "e.g. dining",
        }],
    },
    Command {
        name: "cat merge <from> <into>",
        chord: Chord::NONE,
        description: "reassigns transactions to <into>, deletes <from>",
        args: &[Arg {
            placeholder: "<into>",
            preview: "the surviving category",
        }],
    },
    Command {
        name: "cat archive <cat>",
        chord: Chord(&[KeyCode::Char('a')]),
        description: "active = 0 — keeps every transaction and total",
        args: &[Arg {
            placeholder: "<cat>",
            preview: "the tree selection",
        }],
    },
    Command {
        name: "cat tree [root]",
        chord: Chord::NONE,
        description: "prints the subtree — scriptable/pipeable",
        args: &[Arg {
            placeholder: "[root]",
            preview: "defaults to both roots",
        }],
    },
];
