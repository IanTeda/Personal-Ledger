//! Writes Configuration back to `personal-ledger.conf` (#904), for the Settings rows that edit it:
//! a device's settings (ADR-0053), each in its Settings page's section (ADR-0057), the TUI's own
//! Display keys among them. Only `[General]`, `[Display]` and `[Logs]` are ever written.
//!
//! A write goes to the user's file (`~/.config/personal-ledger/personal-ledger.conf` on Linux),
//! created with its directory when it doesn't exist. It is the one file that is the person's own
//! on every install: the system file needs root, and the working-directory file is the repository's
//! example when a binary runs from a checkout. Every source above it in the hierarchy (the
//! executable-directory, working-directory and `--config` files, then the environment) still wins,
//! so [`ConfigWriter::shadow`] names the one that does, for Settings to say so rather than appear
//! to save.
//!
//! The file is edited line by line rather than round-tripped through an INI parser: neither the
//! `config` crate nor `rust-ini` keeps comments on write, and the person's comments, key order and
//! other sections must survive a write untouched. Only the one key's line changes, or one line is
//! added for a new key, or one line removed.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// The sections Settings writes, one per THIS DEVICE page with keys (ADR-0057). `[Theme*]`,
/// `[Keybindings]` and `[Sync-Server]` are edited only by hand, so none is here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    /// `[General]`: Settings › General.
    General,
    /// `[Display]`: Settings › Display.
    Display,
    /// `[Logs]`: Settings › Logs.
    Logs,
}

impl Section {
    /// The section's header as a write adds it to a file that lacks it.
    pub fn header(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Display => "Display",
            Self::Logs => "Logs",
        }
    }

    /// The section's name as `normalise_ini` reads its header, and as its environment variables
    /// spell it, upper-cased.
    fn normalised(self) -> &'static str {
        match self {
            Self::General => "general",
            Self::Display => "display",
            Self::Logs => "logs",
        }
    }
}

/// The keys Settings writes, each in its page's [`Section`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigKey {
    /// `on_start`: `reopen` or `none`.
    OnStart,
    /// `start_view`: `dashboard`, `last`, `settings` or a noun.
    StartView,
    /// `toasts`: whether non-Error Toasts show.
    Toasts,
    /// `locale`: the Locale's BCP-47 tag; removed for System, the operating system's.
    Locale,
    /// `data`: the folder `:open` and `:new` start in.
    Data,
    /// `colour_theme`: the Colour Theme's id.
    ColourTheme,
    /// `colour_appearance`: `light`, `dark` or `system`.
    ColourAppearance,
    /// `date_style`: `default`, `short`, `medium`, `long` or `iso`.
    DateStyle,
    /// `row_density`: `compact`, `regular` or `roomy`.
    RowDensity,
    /// `status_glyphs`: `unicode` or `ascii`.
    StatusGlyphs,
    /// `hint_bar`: whether the footer shows the key hints.
    HintBar,
    /// `terminal_colours`: draw with the terminal's own colours (ADR-0024).
    TerminalColours,
    /// `log`: the tracing level.
    Log,
    /// `log_file_path`: a file the logs are also written to.
    LogFilePath,
}

impl ConfigKey {
    /// Every key Settings writes, in the order the file's sections and keys are documented.
    pub const ALL: [ConfigKey; 14] = [
        Self::OnStart,
        Self::StartView,
        Self::Toasts,
        Self::Locale,
        Self::Data,
        Self::ColourTheme,
        Self::ColourAppearance,
        Self::DateStyle,
        Self::RowDensity,
        Self::StatusGlyphs,
        Self::HintBar,
        Self::TerminalColours,
        Self::Log,
        Self::LogFilePath,
    ];

    /// The key as it is written in the file.
    pub fn name(self) -> &'static str {
        match self {
            Self::OnStart => "on_start",
            Self::StartView => "start_view",
            Self::Toasts => "toasts",
            Self::Locale => "locale",
            Self::Data => "data",
            Self::ColourTheme => "colour_theme",
            Self::ColourAppearance => "colour_appearance",
            Self::DateStyle => "date_style",
            Self::RowDensity => "row_density",
            Self::StatusGlyphs => "status_glyphs",
            Self::HintBar => "hint_bar",
            Self::TerminalColours => "terminal_colours",
            Self::Log => "log",
            Self::LogFilePath => "log_file_path",
        }
    }

    /// The section the key lives in, its Settings page's.
    pub fn section(self) -> Section {
        match self {
            Self::OnStart | Self::StartView | Self::Toasts | Self::Locale | Self::Data => {
                Section::General
            }
            Self::ColourTheme
            | Self::ColourAppearance
            | Self::DateStyle
            | Self::RowDensity
            | Self::StatusGlyphs
            | Self::HintBar
            | Self::TerminalColours => Section::Display,
            Self::Log | Self::LogFilePath => Section::Logs,
        }
    }

    /// The environment variable that sets the key, by `add_explicit_and_env`'s naming:
    /// `PERSONAL_LEDGER_<SECTION>__<KEY>`.
    pub fn env_var(self) -> String {
        format!(
            "{}_{}__{}",
            crate::ledger::LedgerConfig::env_prefix(),
            self.section().normalised().to_uppercase(),
            self.name().to_uppercase()
        )
    }
}

/// The source that outranks the file a write goes to, so the write changes nothing until it goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Shadow {
    /// An environment variable, by name.
    Env(String),
    /// A configuration file above the user's in the hierarchy.
    File(PathBuf),
}

/// Where Settings writes Configuration, and what outranks it there. Captured as the configuration
/// is parsed, so it knows the files and environment that parse read.
#[derive(Debug, Clone, Default)]
pub struct ConfigWriter {
    /// The user's file; `None` where nothing is written (the Sync Server, or no home directory).
    target: Option<PathBuf>,
    /// The files above the target, lowest precedence first. Each counts only while it exists.
    above: Vec<PathBuf>,
    /// The environment's `PERSONAL_LEDGER_` variables, names lower-cased as `config` matches them.
    env: HashMap<String, String>,
}

impl ConfigWriter {
    /// A writer to `target` that `above`'s files and `env`'s `PERSONAL_LEDGER_` variables
    /// outrank. [`crate::Config::parse`] builds the Client's; a test builds one on a temp file.
    pub fn new(
        target: Option<PathBuf>,
        above: Vec<PathBuf>,
        env: impl IntoIterator<Item = (String, String)>,
    ) -> Self {
        let prefix = format!(
            "{}_",
            crate::ledger::LedgerConfig::env_prefix().to_lowercase()
        );
        let env = env
            .into_iter()
            .map(|(name, value)| (name.to_lowercase(), value))
            .filter(|(name, _)| name.starts_with(&prefix))
            .collect();
        Self { target, above, env }
    }

    /// The file a write goes to.
    pub fn target(&self) -> Option<&Path> {
        self.target.as_deref()
    }

    /// The highest source that sets `key` above the target, or `None` when a write would apply.
    pub fn shadow(&self, key: ConfigKey) -> Option<Shadow> {
        let var = key.env_var();
        if self.env.contains_key(&var.to_lowercase()) {
            return Some(Shadow::Env(var));
        }
        self.above
            .iter()
            .rev()
            .find(|path| std::fs::read_to_string(path).is_ok_and(|text| sets_key(&text, key)))
            .cloned()
            .map(Shadow::File)
    }

    /// Writes `value` to `key` in its section of the target file, keeping the rest of the file as
    /// it was.
    ///
    /// # Errors
    /// Returns [`crate::Error::Validation`] when there is no target, and [`crate::Error::Write`]
    /// when the file or its directory can't be read or written.
    pub fn write(&self, key: ConfigKey, value: &str) -> crate::Result<()> {
        edit(self.target_file()?, |text| Some(set_key(text, key, value)))
    }

    /// Removes `key`'s line from its section of the target file, keeping every other byte, so the
    /// key falls back to the sources below: Settings › General's Locale **System**. A file that
    /// doesn't exist or doesn't set the key is left alone.
    ///
    /// # Errors
    /// As [`Self::write`].
    pub fn remove(&self, key: ConfigKey) -> crate::Result<()> {
        edit(self.target_file()?, |text| remove_key(text, key))
    }

    /// The file a write goes to, or why there is none.
    fn target_file(&self) -> crate::Result<&Path> {
        self.target.as_deref().ok_or_else(|| {
            crate::Error::Validation("no user configuration file to write to".into())
        })
    }
}

/// Replaces `path`'s text with what `change` makes of it, creating the file and its directory as
/// needed; `change` returning `None` leaves the file untouched.
fn edit(path: &Path, change: impl FnOnce(&str) -> Option<String>) -> crate::Result<()> {
    let io = |source| crate::Error::Write {
        path: path.to_path_buf(),
        source,
    };
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(io(error)),
    };
    let Some(changed) = change(&text) else {
        return Ok(());
    };
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(io)?;
    }
    // Written beside the file then renamed over it, so a failed write never leaves half a file.
    let partial = path.with_extension("conf.partial");
    std::fs::write(&partial, changed).map_err(io)?;
    std::fs::rename(&partial, path).map_err(io)
}

/// `text` with `key = value` in the key's section: every line setting the key rewritten in place,
/// else one line added after the section's last entry, else the section appended. Every other line
/// is kept as it was.
fn set_key(text: &str, key: ConfigKey, value: &str) -> String {
    let section = key.section();
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let entry = format!("{} = {value}", key.name());

    let mut in_section = false;
    let mut found = false;
    // The index after the section's header or last entry, where a new key goes: not after a
    // trailing comment, which more likely heads the next section.
    let mut insert_at = None;
    for (index, line) in lines.iter_mut().enumerate() {
        if let Some(name) = section_name(line) {
            in_section = name == section.normalised();
            if in_section {
                insert_at = Some(index + 1);
            }
            continue;
        }
        if !in_section {
            continue;
        }
        let Some(name) = line_key(line) else {
            continue;
        };
        if name.eq_ignore_ascii_case(key.name()) {
            let indent = &line[..line.len() - line.trim_start().len()];
            *line = format!("{indent}{entry}");
            found = true;
        }
        insert_at = Some(index + 1);
    }

    if !found {
        match insert_at {
            Some(index) => lines.insert(index, entry),
            None => {
                if lines.last().is_some_and(|line| !line.trim().is_empty()) {
                    lines.push(String::new());
                }
                lines.push(format!("[{}]", section.header()));
                lines.push(entry);
            }
        }
    }

    let mut out = lines.join("\n");
    out.push('\n');
    out
}

/// `text` without the lines setting `key` in its section, every other byte kept, or `None` when
/// no line sets it.
fn remove_key(text: &str, key: ConfigKey) -> Option<String> {
    let mut in_section = false;
    let mut removed = false;
    let mut out = String::with_capacity(text.len());
    for line in text.split_inclusive('\n') {
        if let Some(name) = section_name(line) {
            in_section = name == key.section().normalised();
        } else if in_section
            && line_key(line).is_some_and(|name| name.eq_ignore_ascii_case(key.name()))
        {
            removed = true;
            continue;
        }
        out.push_str(line);
    }
    removed.then_some(out)
}

/// Whether `text` sets `key` in its section.
fn sets_key(text: &str, key: ConfigKey) -> bool {
    let mut in_section = false;
    text.lines().any(|line| {
        if let Some(name) = section_name(line) {
            in_section = name == key.section().normalised();
            return false;
        }
        in_section && line_key(line).is_some_and(|name| name.eq_ignore_ascii_case(key.name()))
    })
}

/// A section header's name as `normalise_ini` reads it, or `None` for any other line.
fn section_name(line: &str) -> Option<String> {
    let inner = line.trim().strip_prefix('[')?.strip_suffix(']')?;
    Some(inner.trim().to_lowercase().replace('-', "_"))
}

/// The key an entry line sets, or `None` for a comment, a blank or anything else.
fn line_key(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    if trimmed.starts_with(['#', ';']) {
        return None;
    }
    let (key, _) = trimmed.split_once(['=', ':'])?;
    let key = key.trim();
    (!key.is_empty()).then_some(key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    const WRITTEN: &str = "\
# The person's own notes, kept.
[General]
toasts = false
  on_start = reopen

[Display]
date_style = iso
  terminal_colours = false

[Logs]
log = \"debug\"

; Another section, untouched.
[keybindings]
back = \"esc\"
";

    /// A key from each section, with a value it takes.
    const ONE_PER_SECTION: [(ConfigKey, &str); 3] = [
        (ConfigKey::Locale, "en-GB"),
        (ConfigKey::HintBar, "false"),
        (ConfigKey::LogFilePath, "/tmp/ledger.log"),
    ];

    #[test]
    fn every_key_names_its_section_and_its_environment_variable() {
        assert_eq!(
            ConfigKey::ALL.map(|key| (key.section(), key.name())),
            [
                (Section::General, "on_start"),
                (Section::General, "start_view"),
                (Section::General, "toasts"),
                (Section::General, "locale"),
                (Section::General, "data"),
                (Section::Display, "colour_theme"),
                (Section::Display, "colour_appearance"),
                (Section::Display, "date_style"),
                (Section::Display, "row_density"),
                (Section::Display, "status_glyphs"),
                (Section::Display, "hint_bar"),
                (Section::Display, "terminal_colours"),
                (Section::Logs, "log"),
                (Section::Logs, "log_file_path"),
            ]
        );
        assert_eq!(
            ConfigKey::Toasts.env_var(),
            "PERSONAL_LEDGER_GENERAL__TOASTS"
        );
        assert_eq!(
            ConfigKey::RowDensity.env_var(),
            "PERSONAL_LEDGER_DISPLAY__ROW_DENSITY"
        );
        assert_eq!(ConfigKey::Log.env_var(), "PERSONAL_LEDGER_LOGS__LOG");
    }

    #[test]
    fn an_existing_key_is_rewritten_in_place_in_each_section_and_nothing_else_changes() {
        assert_eq!(
            set_key(WRITTEN, ConfigKey::OnStart, "none"),
            WRITTEN.replace("  on_start = reopen", "  on_start = none")
        );
        assert_eq!(
            set_key(WRITTEN, ConfigKey::TerminalColours, "true"),
            WRITTEN.replace("  terminal_colours = false", "  terminal_colours = true")
        );
        assert_eq!(
            set_key(WRITTEN, ConfigKey::Log, "warn"),
            WRITTEN.replace("log = \"debug\"", "log = warn")
        );
    }

    #[test]
    fn a_new_key_joins_the_end_of_its_own_section() {
        let [(general, g), (display, d), (logs, l)] = ONE_PER_SECTION;
        assert_eq!(
            set_key(WRITTEN, general, g),
            WRITTEN.replace(
                "  on_start = reopen\n",
                "  on_start = reopen\nlocale = en-GB\n"
            )
        );
        assert_eq!(
            set_key(WRITTEN, display, d),
            WRITTEN.replace(
                "  terminal_colours = false\n",
                "  terminal_colours = false\nhint_bar = false\n"
            )
        );
        assert_eq!(
            set_key(WRITTEN, logs, l),
            WRITTEN.replace(
                "log = \"debug\"\n",
                "log = \"debug\"\nlog_file_path = /tmp/ledger.log\n"
            )
        );
    }

    #[test]
    fn a_missing_section_is_appended_under_its_header_keeping_the_rest() {
        let text = "# notes\n[keybindings]\nback = \"esc\"";
        for (key, value) in ONE_PER_SECTION {
            let header = key.section().header();
            assert_eq!(
                set_key(text, key, value),
                format!("{text}\n\n[{header}]\n{} = {value}\n", key.name()),
                "{header}"
            );
            assert_eq!(
                set_key("", key, value),
                format!("[{header}]\n{} = {value}\n", key.name())
            );
        }
    }

    #[test]
    fn any_spelling_of_a_section_header_counts() {
        assert_eq!(
            set_key("[general]\n", ConfigKey::Data, "/tmp"),
            "[general]\ndata = /tmp\n"
        );
        assert_eq!(
            set_key("[DISPLAY]\n", ConfigKey::HintBar, "true"),
            "[DISPLAY]\nhint_bar = true\n"
        );
    }

    #[test]
    fn a_commented_out_key_or_one_in_another_section_is_left_alone() {
        let text = "[Display]\n# hint_bar = true\n[Personal-Ledger]\nhint_bar = true\n";
        assert_eq!(
            set_key(text, ConfigKey::HintBar, "false"),
            "[Display]\nhint_bar = false\n# hint_bar = true\n[Personal-Ledger]\nhint_bar = true\n"
        );
    }

    #[test]
    fn a_missing_file_is_created_with_its_directory_and_read_back() {
        let dir = TempDir::new().unwrap();
        let path = dir
            .path()
            .join("personal-ledger")
            .join("personal-ledger.conf");
        let writer = ConfigWriter::new(Some(path.clone()), Vec::new(), []);
        writer.write(ConfigKey::HintBar, "false").unwrap();
        writer.write(ConfigKey::Toasts, "false").unwrap();
        writer.write(ConfigKey::StatusGlyphs, "ascii").unwrap();
        writer.write(ConfigKey::Log, "warn").unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "[Display]\nhint_bar = false\nstatus_glyphs = ascii\n\n\
             [General]\ntoasts = false\n\n[Logs]\nlog = warn\n"
        );
        assert!(!path.with_extension("conf.partial").exists());

        let config =
            crate::Config::parse_with_detector(Some(&path), Some(HashMap::new()), || None).unwrap();
        assert!(!config.display_config().hint_bar());
        assert_eq!(
            config.display_config().status_glyphs(),
            crate::StatusGlyphs::AsciiFallback
        );
        assert!(!config.general_config().toasts());
        assert_eq!(config.logs_config().log(), lib_tracing::Levels::WARN);
    }

    #[test]
    fn each_device_setting_writes_its_key_and_reads_back() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("personal-ledger.conf");
        let writer = ConfigWriter::new(Some(path.clone()), Vec::new(), []);
        writer.write(ConfigKey::Toasts, "false").unwrap();
        writer.write(ConfigKey::ColourTheme, "nord").unwrap();
        writer.write(ConfigKey::ColourAppearance, "light").unwrap();
        writer.write(ConfigKey::DateStyle, "iso").unwrap();
        writer.write(ConfigKey::OnStart, "none").unwrap();
        writer.write(ConfigKey::StartView, "units").unwrap();
        writer.write(ConfigKey::Locale, "en-GB").unwrap();
        writer.write(ConfigKey::Data, "/tmp/ledgers").unwrap();
        writer
            .write(ConfigKey::RowDensity, crate::RowDensity::Roomy.key())
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "[General]\ntoasts = false\non_start = none\nstart_view = units\nlocale = en-GB\n\
             data = /tmp/ledgers\n\n[Display]\ncolour_theme = nord\ncolour_appearance = light\n\
             date_style = iso\nrow_density = roomy\n"
        );

        let config =
            crate::Config::parse_with_detector(Some(&path), Some(HashMap::new()), || None).unwrap();
        let general = config.general_config();
        assert!(!general.toasts());
        assert_eq!(general.on_start(), lib_core::OnStart::NoLedger);
        assert_eq!(
            general.start_view(),
            &lib_core::StartView::Noun("units".into())
        );
        assert_eq!(general.locale(), Some("en-GB"));
        assert_eq!(general.data(), Path::new("/tmp/ledgers"));
        let display = config.display_config();
        assert_eq!(display.colour_theme(), Some("nord"));
        assert_eq!(
            display.colour_appearance(),
            lib_colour_theme::ColourAppearance::Light
        );
        assert_eq!(display.date_style(), Some(lib_core::DateStyle::Iso));
        assert_eq!(display.row_density(), crate::RowDensity::Roomy);

        writer.write(ConfigKey::DateStyle, "default").unwrap();
        let config =
            crate::Config::parse_with_detector(Some(&path), Some(HashMap::new()), || None).unwrap();
        assert_eq!(config.display_config().date_style(), None);
    }

    #[test]
    fn removing_the_locale_deletes_its_line_and_leaves_every_other_byte() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("personal-ledger.conf");
        let before = "# notes\r\n[General]\r\ntoasts = false\r\n  locale = en-GB\r\n\
                      # locale = en-AU\r\non_start = none\r\n\r\n[Other]\r\nlocale = fr\r\n";
        std::fs::write(&path, before).unwrap();
        let writer = ConfigWriter::new(Some(path.clone()), Vec::new(), []);
        writer.remove(ConfigKey::Locale).unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            before.replace("  locale = en-GB\r\n", "")
        );

        // Once gone, removing it again, or from a file with no newline at the end, changes nothing.
        let after = std::fs::read_to_string(&path).unwrap();
        writer.remove(ConfigKey::Locale).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), after);
        std::fs::write(&path, "[General]\ntoasts = false\nlocale = en-GB").unwrap();
        writer.remove(ConfigKey::Locale).unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "[General]\ntoasts = false\n"
        );
    }

    #[test]
    fn removing_from_a_missing_file_creates_nothing_and_with_no_target_is_refused() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("personal-ledger.conf");
        ConfigWriter::new(Some(path.clone()), Vec::new(), [])
            .remove(ConfigKey::Locale)
            .unwrap();
        assert!(!path.exists());
        assert!(matches!(
            ConfigWriter::default().remove(ConfigKey::Locale),
            Err(crate::Error::Validation(_))
        ));
    }

    #[test]
    fn a_setting_an_environment_variable_sets_is_shadowed_by_its_section_s_name() {
        let dir = TempDir::new().unwrap();
        let writer = ConfigWriter::new(
            Some(dir.path().join("user.conf")),
            Vec::new(),
            [
                ("PERSONAL_LEDGER_GENERAL__TOASTS".into(), "false".into()),
                (
                    "PERSONAL_LEDGER_DISPLAY__ROW_DENSITY".into(),
                    "compact".into(),
                ),
                ("personal_ledger_logs__log".into(), "warn".into()),
                // The old section's name no longer sets anything.
                (
                    "PERSONAL_LEDGER_PERSONAL_LEDGER__ON_START".into(),
                    "none".into(),
                ),
                ("HOME".into(), "/home/someone".into()),
            ],
        );
        assert_eq!(
            writer.shadow(ConfigKey::Toasts),
            Some(Shadow::Env("PERSONAL_LEDGER_GENERAL__TOASTS".into()))
        );
        assert_eq!(
            writer.shadow(ConfigKey::RowDensity),
            Some(Shadow::Env("PERSONAL_LEDGER_DISPLAY__ROW_DENSITY".into()))
        );
        assert_eq!(
            writer.shadow(ConfigKey::Log),
            Some(Shadow::Env("PERSONAL_LEDGER_LOGS__LOG".into()))
        );
        assert_eq!(writer.shadow(ConfigKey::OnStart), None);
        assert_eq!(writer.shadow(ConfigKey::StatusGlyphs), None);
    }

    #[test]
    fn a_higher_file_shadows_the_target_only_in_the_key_s_section() {
        let dir = TempDir::new().unwrap();
        let cwd = dir.path().join("cwd.conf");
        let explicit = dir.path().join("explicit.conf");
        let missing = dir.path().join("missing.conf");
        std::fs::write(
            &cwd,
            "[display]\nhint_bar = false\nstatus_glyphs = ascii\n[Personal-Ledger]\ntoasts = false\n",
        )
        .unwrap();
        std::fs::write(
            &explicit,
            "[Display]\nhint_bar = true\n[General]\nlocale = en-GB\n",
        )
        .unwrap();
        let writer = ConfigWriter::new(
            Some(dir.path().join("user.conf")),
            vec![cwd.clone(), explicit.clone(), missing],
            [],
        );
        assert_eq!(
            writer.shadow(ConfigKey::HintBar),
            Some(Shadow::File(explicit.clone()))
        );
        assert_eq!(
            writer.shadow(ConfigKey::StatusGlyphs),
            Some(Shadow::File(cwd))
        );
        assert_eq!(
            writer.shadow(ConfigKey::Locale),
            Some(Shadow::File(explicit))
        );
        assert_eq!(writer.shadow(ConfigKey::Toasts), None);
        assert_eq!(
            ConfigWriter::new(Some(dir.path().join("user.conf")), Vec::new(), [])
                .shadow(ConfigKey::HintBar),
            None
        );
    }

    #[test]
    fn an_existing_file_keeps_its_comments_and_other_sections() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("personal-ledger.conf");
        std::fs::write(&path, WRITTEN).unwrap();
        let writer = ConfigWriter::new(Some(path.clone()), Vec::new(), []);
        writer.write(ConfigKey::TerminalColours, "true").unwrap();
        writer.write(ConfigKey::StartView, "last").unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            WRITTEN
                .replace("  terminal_colours = false", "  terminal_colours = true")
                .replace(
                    "  on_start = reopen\n",
                    "  on_start = reopen\nstart_view = last\n"
                )
        );
    }

    #[test]
    fn with_no_target_a_write_is_refused() {
        assert!(matches!(
            ConfigWriter::default().write(ConfigKey::HintBar, "true"),
            Err(crate::Error::Validation(_))
        ));
    }
}
