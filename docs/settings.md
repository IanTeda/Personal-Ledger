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

__log_file_path:__

Optional path to a file that log output should also be written to, in addition to the console. Parent directories are created if they don’t already exist.

* Type: String (Optional)
* Default: unset (console output only)

Example:

```ini
[Personal-Ledger]
log_file_path = "/var/log/personal-ledger/personal-ledger.log"
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

`quit` is deliberately **not** configurable: `bin-tui` has three separate, hardcoded quit mechanisms (`Ctrl+C` hard-quit, `Q`/`q` graceful quit, and the `:quit` command), none of which read this section.

Individual bindings can be overridden by environment variable with the `PERSONAL_LEDGER_KEYBINDINGS__` prefix, e.g. `PERSONAL_LEDGER_KEYBINDINGS__BACK=ctrl+h` or `PERSONAL_LEDGER_KEYBINDINGS__SUPER_KEY=alt`.

Note that the `g`-leader navigation grammar described in [navigation-design.md](navigation-design.md) (`g d` for the dashboard, `g a` for accounts, and so on) is the intended design, but neither Client reads its bindings from this section yet — `lib-config` parses and validates the section, and the bins still use their own hardcoded shortcuts.

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
