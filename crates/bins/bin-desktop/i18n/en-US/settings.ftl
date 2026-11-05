## Settings, Display: the date style control. The read-only Locale is on General.

desktop-settings-general-locale-from-system = { $locale }, from your system
desktop-settings-general-locale-from-config = { $locale }, from config
desktop-settings-general-locale-from-default = { $locale }, the default
desktop-settings-general-locale-fallback = { $requested } is not supported, so { $locale } is used.

## Settings' pages. The frame, its index and each page's name, description and scope note are
## `lib-settings`' Messages. Terms such as Configuration, Preferences and Change Sets are the ones
## `GLOSSARY.md` defines.

## General (DESKTOP-178): On start, Start view, the read-only Ledger folder and the THIS DEVICE box, whose
## rows are the Ledger On start reopens and the config file Settings writes to.

## Start view: the View the Desktop lands on as it starts. The nouns' own names are the rail's.
desktop-settings-general-start-view-last = Last view
## `$view` is the start_view value the configuration file holds, which isn't one of the choices.

## A device's settings in personal-ledger.conf (ADR-0053). `$key` is the setting's key in the file,
## such as on_start; `$source` is the environment variable or configuration file that sets it.

## Display. `$glyphs` is the run of status glyphs the option draws.

## `$roles` is the overridden Colour Role keys, comma-separated.

desktop-settings-display-colour-overrides = { $roles } overridden by [theme] in your configuration file. Restart to change them.

## A `[theme]` pair below its contrast rule. `$subject` and `$surface` are Colour Role or
## calculated colour keys; `$ratio` and `$required` are contrast ratios.

desktop-settings-display-colour-contrast = { $subject } on { $surface } is { $ratio }:1, needs { $required }:1

## The pages that moved in from the primary rail, until each is rebuilt to the handoff.

desktop-settings-page-placeholder = Not yet rebuilt as a Settings page.

## The status-line legend's fold key on a tree page (Inventory).

