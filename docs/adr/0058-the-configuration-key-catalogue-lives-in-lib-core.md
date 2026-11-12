# The Configuration key catalogue lives in lib-core

#1153 made `lib-config`'s `ConfigKey` and `ConfigValue` the one catalogue of the Configuration keys Settings writes. `lib-settings`, which holds the This device pages' Presenters, may not depend on `lib-config` (`deny.toml`), so it kept a second list, `DeviceKey`, and the Desktop's adapter mapped one to the other. Every new key paid for both lists and the mapping (#1158).

We're moving the catalogue's types into `lib-core`, which every crate may depend on, so `lib-config` and `lib-settings` name the same keys and values and nothing maps between them:

- **What moves:** `ConfigKey` (its name and section), `Section` (its header) and `ConfigValue` (its key), in `lib_core::configuration`.
- **What stays in `lib-config`:** everything about the file and the parse: the `KEYS` table, how a value is spelt in the file, how it is read from a parsed `Config`, and its environment variable.
- **Payload types follow:** a type a `ConfigValue` holds must be in `lib-core` too, since `lib-core` depends on no other workspace crate. `ColourAppearance` moves from `lib-colour-theme` and `Levels` from `lib-tracing`, each re-exported from its old crate. The orphan rule then forbids `lib-tracing`'s `From<Levels> for LevelFilter`, which becomes a function.
- **The Locale stays a tag:** `ConfigValue::Locale` holds a BCP-47 string, since `lib-core` can't name `lib_locale::Locale`; one rule in `lib-locale` turns a tag into a Locale choice.

The `deny.toml` rule stands: `lib-settings` still never depends on `lib-config`, and the Presenters still reach the file only through the `DeviceConfiguration` port.

## Considered options

- **A second list in `lib-settings`, mapped by the adapter** (`DeviceKey` plus a new `DeviceValue`) was rejected. It keeps `lib-core` small and moves nothing, but leaves two lists a test must hold in step, and a new key still pays for both and the mapping: the cost #1158 set out to remove.
- **Letting `lib-settings` depend on `lib-config`** was rejected. It would put the parse and file code in reach of the Presenters, which the VIPER Dependencies rule keeps I/O-free.
- **A new shared crate for the catalogue** was rejected. It would hold the same types as `lib-core` with one more crate and `deny.toml` entry to keep, and `lib-core` already holds the shared value types the catalogue names (`OnStart`, `StartView`, `DateStyle`, `RowDensity`, `StatusGlyphs`).
