# The configuration file's sections mirror the Settings pages

ADR-0053 made every device setting Configuration in the `[Personal-Ledger]` section of `personal-ledger.conf`. That one section mixed Settings › General, Display and Logs, so a user reading the file could not tell which page a key belonged to.

We're giving each THIS DEVICE Settings page its own section, holding exactly the keys that page shows, and keeping a separate section for each area the user edits only by hand. The change breaks existing files: `[Personal-Ledger]` is neither read nor migrated, and there is no alias.

| Section | Keys | Edited from |
| --- | --- | --- |
| `[General]` | `on_start`, `start_view`, `toasts`, `locale`, `data`, `file` | Settings › General (`file` only by hand or `--file`) |
| `[Display]` | `colour_theme`, `colour_appearance`, `date_style`, `row_density`, `status_glyphs`, `hint_bar`, `terminal_colours` | Settings › Display (`hint_bar` and `terminal_colours` read only by the TUI) |
| `[Logs]` | `log`, `log_file_path` | Settings › Logs; read by all three binaries, the Sync Server included |
| `[Theme]`, `[Theme.Light]`, `[Theme.Dark]` | Colour Role overrides | by hand only |
| `[Keybindings]` | `super_key` and the command names | by hand only |
| `[Sync-Server]` | `bind_address`, `database_uri` | by hand only; the Sync Server binary's own section |

- **The naming rule.** A section is either a Settings page, named in title case with hyphens for spaces (`[AI-Connections]`), or a hand-edit area that no page shows. The parser ignores case and treats `-` and `_` as the same, so `[ai_connections]` reads the same section. A page gets a section only once it has a key, so AI Connections and About have none yet.
- **`[Sync-Server]` is not Settings › Sync Server.** That page is THIS LEDGER and keeps its settings in the Ledger, never in this file. The section is the server binary's own and keeps its name.
- **Keys keep their names**; only their section moves. Environment variables follow the sections, by `PERSONAL_LEDGER_<SECTION>__<KEY>` (`PERSONAL_LEDGER_GENERAL__TOASTS`, `PERSONAL_LEDGER_LOGS__LOG`). `--data`, `--file`, `--log` and `--locale` keep their names and override their keys' new sections.
- **`config` is no longer a key.** Only `--config` chooses an explicit file, because a file that names another file is circular.
- **Settings writes only what a page edits.** Every change on a THIS DEVICE page, in either Client, is written to the user's configuration file at once, into the section of the key it changes. `ConfigWriter` never writes `[Theme*]`, `[Keybindings]` or `[Sync-Server]`. A running Client reads the file only when it starts, so a change made in one Client reaches another at that Client's next start.
- **The Locale and the Data folder become editable on General.** The Locale is a picker of System and the supported Locales. Choosing System removes the `locale` key. A chosen Locale is written at once but applies at the Client's next start. The Data folder is chosen with the `:open` explorer in a folder-picking mode, from General or from anywhere with `:data`. It applies at once and moves no files. As before, a value shadowed by an environment variable, a flag or a higher file is shown as such.

This supersedes ADR-0053 on the section the keys live in, and ADR-0021 and `GLOSSARY.md`'s **Locale** on there being no in-app switch. The Locale is still Configuration, not a Preference.

## Considered options

- **Keeping `[Personal-Ledger]` with an alias or migration** was rejected. The app is still in concept and has no users' files to protect, and an alias would leave two spellings of every key for good.
- **Nesting the Colour Role overrides as `[Display.Theme.Light]`** was rejected. No page edits them, and the environment variables would grow to `PERSONAL_LEDGER_DISPLAY__THEME__LIGHT__BORDER`.
- **A `[Sync-Server] log` key** was rejected in favour of one `[Logs]` section that every binary reads.
- **A live Locale switch** was rejected for now. Reloading the Catalogues and every formatter mid-session costs more than a restart is worth.
- **Watching the file so another running Client sees a change at once** was rejected for the same reason.
