//! Every `(section, key)` `lib-config` parses, for the drift tests against the shipped
//! `config/personal-ledger.conf`. Section names are the normalised spelling (lower case, `_` for
//! `-`). `[Keybindings]`' command bindings are an open-ended map and are not listed: only
//! `super_key` is a fixed key.

/// The keys of each section, beside the structs that read them: a struct field added without a
/// row here fails `table_matches_the_structs`.
pub(crate) const KEYS: &[(&str, &[&str])] = &[
    (
        "general",
        &["on_start", "start_view", "toasts", "locale", "data", "file"],
    ),
    (
        "display",
        &[
            "colour_theme",
            "colour_appearance",
            "date_style",
            "row_density",
            "status_glyphs",
            "hint_bar",
            "terminal_colours",
        ],
    ),
    ("logs", &["log", "log_file_path"]),
    ("keybindings", &["super_key"]),
    ("sync_server", &["bind_address", "database_uri"]),
    ("theme", THEME_ROLES),
    ("theme.light", THEME_ROLES),
    ("theme.dark", THEME_ROLES),
];

/// The seven stored Colour Roles (`lib_colour_theme::ColourRole::ALL`, checked by a test).
const THEME_ROLES: &[&str] = &[
    "foreground",
    "background",
    "accent",
    "cursor",
    "muted",
    "positive",
    "negative",
];
