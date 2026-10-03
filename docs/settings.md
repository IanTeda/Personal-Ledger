# Settings

Personal Ledger has two types of configurations: static and dynamic. Static configurations are needed to bootstrap or start the application, are not intended to change often, or are system-space or user-space configurations applied across different ledger files. Static configurations live in configuration files. Dynamic configurations aren't needed to bootstrap the application, change often, and can be set and updated within the application itself.

## Static Configuration Files

Personal Ledger looks for static configuration files in multiple locations. It uses a layered configuration approach with a defined precedence hierarchy to support different deployment scenarios. The next sections describe that hierarchy in detail.

## Configuration Hierarchy

Personal Ledger looks for and loads configuration files in an order of precedence. If it finds a configuration file in a location, it parses it and uses the settings it finds. If it finds a subsequent file with higher precedence and a conflicting setting, it will supersede that setting with the one from the higher-precedence configuration file.

Personal Ledger uses the following order of precedence, from lowest to highest. 

1. Built-in Defaults (lowest precedence).
2. System Configuration
  * Platform-specific system-wide config directory
  * Linux: /etc/personal-ledger/personal-ledger.conf
  * macOS: /Library/Preferences/personal-ledger/personal-ledger.conf
  * Windows: %ALLUSERSPROFILE%\personal-ledger\personal-ledger.conf
3. Executable Directory
  * Configuration file in the same directory as the binary
  * Useful for portable applications
4. Current Working Directory
  * File: config/personal-ledger.conf
  * Allows project-specific overrides when running from a directory
5. User Configuration
  * Platform-specific user config directory
  * Linux/macOS: ~/.config/personal-ledger/personal-ledger.conf
  * Windows: %APPDATA%\personal-ledger\personal-ledger.conf
6. Explicit Configuration File
  * The file pointed to by --config/-c, if given (still layered in below Environment
    Variables, same as every other file source above)
7. Environment Variables
  * Prefix: PERSONAL_LEDGER_
  * Example: PERSONAL_LEDGER_PERSONAL_LEDGER__LOG=debug
  * Example: PERSONAL_LEDGER_SYNC_SERVER__DATABASE_URI=sqlite:/tmp/test.db
  * Use double underscores (__) to separate nested keys
8. Explicit Command Line Flags (highest precedence)
  * --data/-d, --file/-f, --log/-l, --locale -- override the `[Personal-Ledger]` section's
    corresponding setting directly, applied after every source above (including
    Environment Variables)

## INI Configuration Format

Personal Ledger uses the INI (Initialisation) file format for static configuration files. INI is a simple, human-readable format consisting of sections, keys, and values.

### Basic Syntax

* Sections: Enclosed in square brackets [], e.g., [Personal-Ledger]
* Keys and Values: key = value, e.g., log = "debug"
* Comments: Lines starting with # or ; are comments.
* Case Sensitivity: Section names are case-insensitive (e.g., [Personal-Ledger] and [personal-ledger] are equivalent)

Example Structure

# This is a comment
[SectionName]
key1 = "string value"
key2 = 42
key3 = true

### Rules

* Section names should be descriptive and contain only alphanumeric characters, underscores, and hyphens.
* Keys should use lowercase with underscores (snake_case)
* String values should be quoted when they contain spaces or special characters.
* Boolean values: true or false 
* Numeric values: integers or floats as appropriate.

## Section Names

Configuration sections group related settings together. The application currently supports:

* [Personal-Ledger]: Bootstrap and startup configurations.
* [Keybindings]: Navigation and action keybindings.
* [Theme]: Colour Role overrides laid over the chosen Colour Theme, with [Theme.Light] and [Theme.Dark] for one Colour Variant. Read by bin-tui and bin-desktop.
* [Sync-Server]: Sync Server-specific settings (bind address). Only bin-sync-server uses the configuration settings. Both bin-tui and bin-desktop Sync Server settings are dynamic and thus use settings pulled from the database.

### Personal Ledger Section

The [Personal-Ledger] section includes settings applied across Personal Ledger files and the client on a given system, but they are not intended to sync across computer systems.

__config:__

Points to a configuration directory not in the predefined hierarchy discussed above

* Type: String (Optional)
* CLI Flag: --config/-c
* Default: Refer to above ‘Configuration Hierarchy’

Example:

```ini
[Personal-Ledger]
config = "~/.config/personal_ledger.conf"
```

```bash
personal_ledger --config/-c '~/.config/personal_ledger.conf'
```

__data:__

Use a specific directory as the default location for Personal Ledger database files.

* Type: String
* CLI Flags: --data/-d 
* Default:
  * Defaults to the users `document` folder

Example:

```ini
[Personal-Ledger]
data = "~/Documents/Personal-Ledger"
```

```bash
personal_ledger --data/-d '~/Documents/Personal-Ledger/'
```

__file:__

Open to a specific Personal Ledger database file if no file is provided.

* Type: String (Optional)
* CLI Flags: --file/-f 
* Default: <data-dir>/My-Personal-Ledger.pldb 

Example:

```ini
[Personal-Ledger]
file = "~/Documents/My-Personal-Ledger.pldb"
```

```bash
personal_ledger --file/-f '~/Documents/My-Personal-Ledger.pldb'
```

__log:__

Controls the verbosity of logging output.

* Type: String
* CLI Flags:  --log/-l 
* Valid Values:
  * "trace": Most verbose, includes all internal debugging information
  * "debug": Detailed debugging information
  * "info": General information messages (default)
  * "warn": Warning messages only
  * "error": Error messages only
  * "off": No logging output
* Default: "info"

Example:

```ini
[Personal-Ledger]
log = "debug"
```

```bash
personal_ledger --log/-l 'debug'
```

__locale:__

The Locale the Client uses for its text, numbers, dates and currency amounts, as a BCP-47 language tag. Per Client and never synced; changing it needs a restart. `lib-config` only resolves the request and reports where it came from -- `lib-locale` decides which Locale is actually used, so a well-formed tag it doesn't support (for example `fr-FR`) is passed through unchanged and negotiated there.

* Type: String (Optional)
* CLI Flag: --locale (long form only)
* Environment Variable: PERSONAL_LEDGER_PERSONAL_LEDGER__LOCALE
* Valid Values: any well-formed BCP-47 tag, stored in canonical casing (`en-gb` becomes `en-GB`). A malformed value is a startup error. There is no `system` keyword: leave the key unset to follow the operating system.
* Default, in order:
  1. The value from the configuration files, environment variable or `--locale`
  2. The operating system's Locale, when it is a usable tag (`C` and `POSIX` are ignored)
  3. "en-US"

Example:

```ini
[Personal-Ledger]
locale = "en-GB"
```

```bash
personal_ledger --locale 'en-GB'
```

The Sync Server ignores this setting.

__terminal_colours:__

TUI only. When true, the TUI ignores the Colour Theme and any `[theme]` overrides and draws in the terminal's own ANSI colours, so your terminal theme wins. Per Client and never synced; changing it needs a restart. The Desktop ignores it.

* Type: Boolean
* Default: false
* Environment Variable: PERSONAL_LEDGER_PERSONAL_LEDGER__TERMINAL_COLOURS

Example:

```ini
[Personal-Ledger]
terminal_colours = true
```

__log_file_path:__

Optional path to a file that log output should also be written to, in addition to the console. Parent directories are created if they don’t already exist.

* Type: String (Optional)
* Default: unset (console output only)

Example:

```ini
[Personal-Ledger]
log_file_path = "/var/log/personal-ledger/personal-ledger.log"
```

## The Settings screen in the Desktop app

Settings in the Desktop app is paged: a list of pages down the left (the index) and one page open beside it. Choosing an entry swaps the page; nothing scrolls from one section into the next. The pages are General, Display, Units, Institutions, Accounts, Categories, Tags, Payees, Documents, Inventory, Sync server, Data and backup, Tracing and About.

Accounts, Categories, Tags and Payees live here rather than on the main rail. You reach them from the index, with `:settings accounts` (and likewise `categories`, `tags`, `payees`, `documents`, `inventory`), or with the older `:accounts`, `:categories`, `:tags` and `:payees` commands, which still work.

- Press `g` `s` to open Settings with the index focused. `j` and `k` move through the index and the page follows. `/` searches the index.
- Press `l` to move into the open page and `h` to come back to the index. Page keys such as `e` (edit), `d` (delete), `n` (new), `x` (remove) and `m` (merge) only act while the page has focus.
- `Esc` backs out one step at a time: a dialog or search first, then the page back to the index.
- The page you last had open is remembered, so Settings reopens where you left it.

### Documents page

The Documents page is where you manage the list of Document types, the kinds your documents are filed under. A new ledger starts with Receipts, Statements, Tax, Insurance, Warranties & manuals, Contracts, Identity, Bills and Other. Like the rest of Documents, it runs on sample data for now, so nothing you change is saved.

The page is a table with a row per type: its name, what date it tracks (Renews, Ends, Expires, Revalue or none), how long before that date to remind you, whether it is tied to a financial year, and how many files it holds. The order of the rows is the order of the Type filter in Documents.

- Press `n` to add a type. A dialog asks for a name, what date it tracks, a reminder lead time and whether it is tied to a financial year. Names must be different from every other type, ignoring capitals.
- Press `e` to edit the selected type. Renaming a type keeps its files and any saved filters.
- Press `shift`+`j` or `shift`+`k` to move the selected type down or up. Documents shows its Type filter in the same order.
- Press `x` to remove the selected type. If it still holds files, you choose which type they move to (Other is pre-selected). Removing a type never removes a document.
- **Other** is the fallback type. You can rename and reorder it, but you can't remove it; pressing `x` on it tells you why.

A reminder only does something when the type tracks a date, and reminders don't notify you yet. Turning Financial year on or off changes what Documents shows for the type, not any document's date.

### Inventory page

The Inventory page is where you set up the places whose contents you keep a register of. A **Property** is one such place, such as a house or a storage unit, and each Property holds any number of **Rooms**. Each Property is one Inventory register, so its Rooms become that register's room tabs, in the order shown here. Like the rest of Inventory, it runs on sample data for now, so nothing you change is saved.

The page is a table with a row per Property: its name and address, its insurance policy (insurer and policy number), its sum insured, its single-item limit and how many items it holds. Properties are listed in the order you added them. They all start collapsed. Expanding one shows its Rooms in the order of the room tabs, each with its item count and value, and a clickable **+ Add room** row at the end. A line under the table counts what you have, for example "2 properties · 11 rooms · 232 items". With no Properties the table is replaced by "No properties yet. Press n to add one."

- Press `j` and `k` to move through the visible rows, Properties and the Rooms of expanded Properties alike. `e` and `x` act on whichever row is selected.
- Press `→` to expand a collapsed Property, or to move to the first Room of an expanded one. Press `←` to collapse an expanded Property, or to move from a Room to its Property.
- Press `n` to add a Property. A dialog asks for a name (unique across your Properties, ignoring capitals), an address and a fiat Unit. The Unit is fixed once the Property is added, because every value in it is in that Unit. The new Property opens expanded.
- Insurance cover is optional. Fill in the insurer, the sum insured and, if you like, a policy number, a renewal date and a single-item limit. Leave the insurer empty for no cover, which clears the other cover fields. The renewal date is for display and changes nothing when it passes. The single-item limit can't be more than the sum insured.
- Press `r` to add a Room to the selected Property, or to the Property of the selected Room. A collapsed Property expands so you see the new Room. Room names must be different within a Property, ignoring capitals, but two Properties can each have a Kitchen.
- Press `e` to edit the selected Property or Room. Renaming a Room keeps its items.
- Press `shift`+`j` or `shift`+`k` to move the selected Room down or up within its Property. They do nothing on a Property row or at the ends of the list.
- Press `x` to remove the selected row.
  - A Room with items asks which Room of the same Property takes them, with the one above pre-selected. A Property's only Room can't be removed while it holds items; the dialog tells you to add another Room first or remove the Property.
  - A Property that holds Rooms or items asks you to type its name to confirm. Its Rooms and items are removed with it, and any Document that linked to those items loses that link. The Documents and the transactions that bought the items are kept. You can remove your last Property.

Sum insured and Item limit are what the Inventory register uses for its cover check and its over-limit flag. The register itself isn't built yet.

## Toasts

A Toast is a short message in the bottom-right corner that tells you how something turned out, for example "Deleted tag Food". You can turn Toasts off from Settings: go to Display, where the Toasts row sits above the Colour Theme settings. In the Desktop app it's an On / Off switch; in the TUI, move to the row and press `Enter` to flip it. From the command box you can also run `toasts on` or `toasts off`.

With Toasts off, the message shows on the status line at the bottom instead, for as long as the Toast would have shown. Errors still appear as Toasts either way, so you never miss one; the note beside the row says so.

Two keys go with Toasts, both set in the Keybindings Section below:

- `dismiss_toasts` (`Ctrl+L` by default) clears every Toast on screen.
- `toast_history` has no key by default. The `toasts` command (or `messages`) opens the list of this session's Toasts; set this key if you'd like a shortcut too.

See [Getting around](getting-around.md#toasts) for reading and dismissing Toasts.

### Heads up

- Whether Toasts are on is remembered only until you close the app, and only on this device.

## Colour Theme and Colour Appearance

You choose how Personal Ledger looks from Settings, not from a configuration file. Two settings work together:

- **Colour Theme**: the set of colours the app draws in. Five are built in: Modernist (the default), High Contrast, Catppuccin, Gruvbox and Nord. Each one has a light and a dark version.
- **Colour Appearance**: which version you see. Light and Dark always use that version. System follows your computer's light or dark setting, and the picker tells you which one it found, for example "System (currently Dark)". If it can't find out, it uses Dark and says so: "System (not detected, using Dark)".

A change applies straight away, with no Apply button. Both are meant to be Preferences that sync across your own devices.

### In the Desktop app

Open Settings and go to Display. Below the other display settings is a Colour Theme group:

- Appearance is a Light / Dark / System switch.
- Colour Theme is a grid of preview cards, each showing a small ledger drawn in that Colour Theme. The card you've chosen has a border in the accent colour and a bold name.
- With the keyboard, press `Tab` to move onto the grid, use the arrow keys or `h` `j` `k` `l` to move between cards, and press `Enter` to choose one. Moving between cards doesn't change anything until you press `Enter`.
- From the command palette you can also run `theme modernist`, `theme high-contrast`, `theme catppuccin`, `theme gruvbox` or `theme nord`, and `appearance light`, `appearance dark` or `appearance system`.

When your computer switches between light and dark, the Desktop follows it straight away if Appearance is System.

### In the TUI

Open Settings and press `Tab` to reach the Display group, where you'll find the colour theme and appearance rows. Press `Enter` on a row to open a list. Each Colour Theme in the list shows seven colour swatches. As you move with `j` and `k`, the screen previews that choice; press `Enter` to keep it or `Esc` to go back to what you had.

The TUI draws the Colour Theme's exact colours by default. If your terminal only supports 256 colours, it uses the closest ones it has. If it only supports 16 colours, or you set `terminal_colours = true` (see the Personal Ledger Section), it uses your terminal's own colours instead. You can still change the rows while `terminal_colours` is on, and a note under them explains why this TUI isn't using them.

For System, the TUI asks your terminal whether its background is light or dark when it starts. It doesn't notice a change after that, so restart the TUI to pick up a new setting.

### Changing individual colours

If a built-in Colour Theme is nearly right, you can change individual colours on one device with the `[theme]` section below. While any colour is changed that way, Settings shows a note under the pickers naming the colours and the file they came from. If a colour you set is hard to read against another, Settings shows a warning line such as "accent on background is 2.4:1, needs 3:1".

### Heads up

- Your Colour Theme and Colour Appearance are remembered only until you close the app. Saving them to your ledger, and syncing them to your other devices, isn't built yet.

## Theme Section

The `[theme]` section overrides the colours of the Colour Theme you picked in Settings, on this Client only. It holds only the seven Colour Role keys; it can't choose a Colour Theme or Colour Appearance, which are Preferences and sync. Every other colour (borders, hover, selection, chart series) is calculated from these roles, so it follows your overrides. Changing `[theme]` needs a restart.

A key in `[theme]` applies to both the light and dark Colour Variants. A key in `[theme.light]` or `[theme.dark]` applies to that Colour Variant only and wins over the same key in `[theme]`. Name only the roles you want to change; the rest come from the Colour Theme.

Keys: `foreground`, `background`, `accent`, `cursor`, `muted`, `positive`, `negative`.

* Type: Hex colour, with or without the leading `#` (e.g. `"#1F6FEB"` or `"1F6FEB"`)
* Default: unset (the Colour Theme's own colour)
* Environment Variable: PERSONAL_LEDGER_THEME__ACCENT, PERSONAL_LEDGER_THEME__DARK__ACCENT (and so on for each key)

A value that isn't a hex colour, or a key that isn't one of the seven roles, is ignored with a warning in the log; the Client still starts. A colour that fails the contrast rules is still used, with a warning in the log and in Settings.

Example:

```ini
[Theme]
accent = "#1F6FEB"

[Theme.Dark]
accent = "#58A6FF"
```

## Keybindings Section

The `[Keybindings]` section defines the keyboard navigation keys and key combinations. Refer to [Navigation and keyboard grammar](navigation-design.md) for Personal Ledger's philosophy and approach to keyboard navigation.

The section has one fixed key, `super_key` — the modifier held down for global shortcuts, mirroring a window manager's mod/prefix key. It accepts `"ctrl"`, `"alt"`, `"shift"`, `"super"`, or `"none"` to require no modifier at all, and defaults to `"ctrl"`. Everything else in the section is an open-ended map of command name to key, so new screens can add their own commands without a schema change. Binding two commands to the same key is a startup error, since it would leave one of them unreachable.

The built-in defaults are:

| Command              | Default key |
| -------------------- | ----------- |
| `back`               | `esc`       |
| `help`               | `?`         |
| `open_command_popup` | `:`         |
| `move_up`            | `k`         |
| `move_down`          | `j`         |
| `select`             | `enter`     |
| `new`                | `n`         |
| `delete`             | `d`         |
| `confirm`            | `y`         |
| `cancel`             | `x`         |
| `dismiss_toasts`     | `ctrl+l`    |
| `toast_history`      | unbound     |

`quit` is deliberately **not** configurable: `bin-tui` has three separate, hardcoded quit mechanisms (`Ctrl+C` hard-quit, `Q`/`q` graceful quit, and the `:quit` command), none of which read this section.

`dismiss_toasts` dismisses every showing Toast, in `Normal` mode only; it is deliberately not `esc`, so a sticky Error Toast doesn't vanish on the key that backs out of a dialog. `toast_history` has no default key because the `toasts` command (alias `messages`) opens the Toast history; set it to bind one. See [Toasts design](toasts-design.md).

Individual bindings can be overridden by environment variable with the `PERSONAL_LEDGER_KEYBINDINGS__` prefix, e.g. `PERSONAL_LEDGER_KEYBINDINGS__BACK=ctrl+h` or `PERSONAL_LEDGER_KEYBINDINGS__SUPER_KEY=alt`.

Note that the `g`-leader navigation grammar described in [navigation-design.md](navigation-design.md) (`g d` for the dashboard, `g b` for budgets, and so on) is the intended design, but neither Client reads its bindings from this section yet — `lib-config` parses and validates the section, and the bins still use their own hardcoded shortcuts.

Example:

```ini
[Keybindings]
super_key = "ctrl"
back = "esc"
help = "?"
open_command_popup = ":"
move_up = "k"
move_down = "j"
select = "enter"
new = "n"
delete = "d"
confirm = "y"
cancel = "x"
dismiss_toasts = "ctrl+l"
```

## Sync Server Section

bin-sync-server uses a reduced precedence chain instead (ADR-0014): Environment Variables → Explicit Configuration File (--config/-c) → Built-in Defaults. The Current Working Directory/Executable Directory/User/System tiers don’t apply – they don’t correspond to anything meaningful inside a Docker container, the Sync Server’s only deployment target.

The [Sync-Server] section currently has two settings:

bind_address

The socket address the Sync Server’s gRPC/HTTP listener binds to.

* Type: String
* Default: "0.0.0.0:50051"

database_uri

The URI of the Sync Server’s own database (its durable Change Set log, per ADR-0009) — not a Client’s local `[Personal-Ledger] file`; the Sync Server’s database is server-side storage, unrelated to any Client’s local Ledger file. Connection-pool settings (max/min connections, timeouts) are fixed in code, not configurable.

* Type: String
* Default: "sqlite:./sync-server.sqlite"

Example:

```ini
[Sync-Server]
bind_address = "0.0.0.0:50051"
database_uri = "sqlite:./sync-server.sqlite"
```

* Hardcoded default values in the application code

Higher precedence sources override lower precedence ones. For example, an environment variable will override any configuration file setting.

Example Configuration File

```ini
# Personal Ledger Configuration File
#
# This file contains configuration settings for the Personal Ledger.
# It uses INI format with sections and key-value pairs.
#
# Section names are case-insensitive (e.g., [Personal-Ledger] or [personal-ledger] both work).
# Values should be quoted strings where appropriate.
#
# For more information, see the documentation at docs/settings.md



# Settings applied across Personal Ledger files and the client on a given system, but 
# they are not intended to sync across computer systems.
[Personal-Ledger]

# Points to a configuration file outside the normal search hierarchy. Usually left unset.
# config = "~/.config/personal-ledger/personal-ledger.conf"

# Client data directory
data = "~/Documents/Personal-Ledger"

# Personal Ledger database file to open with
file = "~/Documents/Personal-Ledger/My-Personal-Ledger.pldb"

# Client tracing log level
log = "debug"

# Optional: also write log output to this file, in addition to the console
# log_file_path = "/var/log/personal-ledger/personal-ledger.log"

# Optional: the interface Locale, as a BCP-47 tag -- "en-US", "en-GB" or "en-AU".
# When absent, the operating system's locale is used, falling back to "en-US".
# locale = "en-AU"



# Keyboard navigation keys and key combinations.
[Keybindings]

super_key = "ctrl"
back = "esc"
help = "?"
open_command_popup = ":"
move_up = "k"
move_down = "j"
select = "enter"
new = "n"
delete = "d"
confirm = "y"
cancel = "x"

# Optional: override Colour Roles of the chosen Colour Theme on this Client.
# [Theme]
# accent = "#1F6FEB"
# [Theme.Dark]
# accent = "#58A6FF"

# Only read by bin-sync-server -- bin-tui/bin-desktop ignore this section.
[Sync-Server]
bind_address = "0.0.0.0:50051"
database_uri = "sqlite:./sync-server.sqlite"
```

This example is kept in sync with the file shipped at `config/personal-ledger.conf`.


References

Per ADR-0014, Configuration covers only settings needed before the app (or its database) can run: the database location (a Client’s `[Personal-Ledger] file`, or the Sync Server’s own `[Sync-Server] database_uri`) and the telemetry level; connection-pool tuning (max/min connections, timeouts) is fixed in code rather than configurable. The gRPC bind address is Sync-Server-only. Everything else the user might tweak from inside a running Client — colour theme, date format, decimal/thousands separator, default Unit — is a Preference instead, stored in the Ledger’s own database, not here.

lib-config is shared by all three consumers (bin-tui, bin-desktop, bin-sync-server), but they don’t all see the same sections or search the same locations — see Configuration Hierarchy and Sync Server Section below.

## For developers

Curious how settings are structured in the codebase, or planning to change them? See the [Settings development documentation](development/settings.md).
