# Getting around

Personal Ledger is built so you can do nearly everything without touching the mouse. This page shows you the few keys worth learning. There are only a handful, and they work the same way in the terminal app and the desktop app, so if you use both, you won't have to relearn anything.

## The big idea

Every single thing you can do in Personal Ledger has a name, and you can always reach it by typing that name. Keyboard shortcuts are just faster ways to get there. So if you forget a shortcut, you're never stuck: open the command box (below), type what you want, and go.

## Three kinds of keys

Almost everything you'll do comes down to three ideas.

### 1. Jump somewhere with `g`

Press `g`, let go, then press a letter to leap straight to a screen. Think of it as "**g**o to…". So `g` then `a` takes you to Accounts, and `g` then `d` takes you to the Dashboard.

If you press `g` and then a letter that doesn't go anywhere, nothing happens and you're back where you started, so it's safe to poke around. In the desktop app, the `g` also gives up on its own if you wait about a second.

Here are the jump letters for each app:

| Go to | Terminal app | Desktop app |
| --- | --- | --- |
| Accounts | `g` `a` | Settings › Accounts (`:accounts`) |
| Budgets | `g` `b` | `g` `b` |
| Categories | `g` `c` | Settings › Categories (`:categories`) |
| Dashboard | `g` `d` | `g` `d` |
| Payees | `g` `p` | Settings › Payees (`:payees`) |
| Reports | `g` `r` | `g` `r` |
| Settings | `g` `s` | `g` `s` |
| Tags | `g` `g` | Settings › Tags (`:tags`) |
| Transactions | `g` `t` | `g` `l` |
| Units | `g` `u` | Settings › Units (`:settings units`) |
| Balance checks | `g` `k` | not in the desktop app yet |
| Bills | not in the terminal app yet | `g` `w` |
| Documents | not in the terminal app yet | `g` `f` (not yet built) |
| Notifications | not in the terminal app yet | `g` `a` (not yet built) |
| Cash | not in the terminal app yet | `g` `c` (not yet built) |
| Inventory | not in the terminal app yet | `g` `o` (not yet built) |
| Loans | not in the terminal app yet | `g` `n` (not yet built) |
| Credit cards | not in the terminal app yet | `g` `k` (not yet built) |
| Investments | not in the terminal app yet | `g` `i` (not yet built) |

In the desktop app, Accounts, Categories, Payees and Tags no longer have a screen of their own or a `g` key: they are pages inside Settings. Typing `:accounts`, `:categories`, `:payees` or `:tags` still takes you to the right page, and `:settings` followed by a page name (such as `:settings tags`) opens any Settings page directly. The screens marked "not yet built" respond to their key and tell you so.

The two apps don't share every screen yet, and Tags and Transactions use different letters in each one. We'd like them to match eventually.

### 2. Do anything by name with `:`

Press `:` (a colon) and a command box opens. Start typing what you want to do, like `accounts new`, and it suggests matches as you go. Press `Enter` to run it.

This is your safety net. If you can't remember a shortcut, or you're not sure where something lives, `:` will find it.

Some commands are listed but aren't finished yet. If you run one, Personal Ledger tells you it's "not yet built" rather than quietly doing nothing.

### 3. Find things with `/`

Press `/` to start searching or filtering. In the terminal app this works on the Accounts, Payees and Tags screens. In the desktop app, Search narrows the Settings index (also `:settings search`) and the search box on the Transactions screen narrows its rows.

## Always know where you are

Personal Ledger is *modal*, which is a fancy way of saying the same key can mean different things depending on what you're doing. That sounds scarier than it is, because the status line at the bottom always tells you what's going on. There are four situations:

- **Normal:** the resting state, where keys are shortcuts. You won't see a label for this one.
- **Insert:** you're typing into a form, so your keys become text.
- **Command:** the `:` box is open.
- **Search:** you're typing a search or filter.

Ever unsure what a key will do? Glance at the bottom of the screen first.

## A few keys worth remembering

| Key | What it does |
| --- | --- |
| `?` | Shows help for the screen you're on |
| `Esc` | Goes back, or closes whatever's open |
| `:` | Opens the command box |
| `/` | Starts a search or filter |
| `g` then a letter | Jumps to a screen |

Each screen also has its own keys for things like adding, editing or deleting an item, and the help screen (`?`) lists them for wherever you are.

## Toasts

When something finishes, such as deleting an account or opening a ledger, a small message called a Toast pops up in the bottom-right corner, just above the status line. A coloured mark at its start tells you what kind it is:

| Mark | Kind | Stays for |
| --- | --- | --- |
| `i` | Info | 4 seconds |
| `✓` | Success | 4 seconds |
| `!` | Warning | 8 seconds |
| `✗` | Error | until you dismiss it |

Up to three Toasts show at once, newest at the bottom. If the same message comes up again, it shows a count such as `×2` instead of stacking a copy. A Toast waits while you have the command box, a dialog or help open, and in the desktop app while your mouse is over it. Long messages are cut short with `…`; the history shows them in full.

To get rid of Toasts:

- **`Ctrl+L`** clears them all (when no box or dialog is open).
- **`:dismiss`** clears the newest, and **`:dismiss all`** clears the lot.
- In the desktop app, click the `✕` on a Toast.

`Esc` deliberately leaves Toasts alone, so backing out of a dialog never loses an Error.

Missed one? Run **`:toasts`** (or `:messages`) to see every Toast from this session, newest first, with the time it came up. Press `Esc` to close it. The list holds the last 100 and is emptied when you quit.

If you'd rather not see Toasts, you can turn them off in [Settings](settings.md#toasts).

## Quitting

In the terminal app there are three ways out, and they all work:

- **`q`** quits politely.
- **`:quit`** does the same, by name.
- **`Ctrl+C`** quits straight away, which is handy if something ever seems stuck.

In the desktop app, you close the window like any other app.

## Changing the keys

If you use the terminal app, you can change the keys for going back, opening help and opening the command box in your config file. See [Settings](settings.md) for where that file lives.

The desktop app doesn't have this yet. The other keys, such as `g` for jumping and `q` for quitting, are fixed for now.

## For developers

Curious how the keys are built, or planning to change them? See [Navigation and keyboard grammar: design](navigation-design.md).
