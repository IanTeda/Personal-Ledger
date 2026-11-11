//! Writes Configuration back to `personal-ledger.conf` (#904), for the Settings rows that edit it:
//! a device's settings (ADR-0053), each in its Settings page's section (ADR-0057), the TUI's own
//! Display keys among them. Only `[General]`, `[Display]` and `[Logs]` are ever written.
//!
//! A write goes to the user's file (`~/.config/personal-ledger/personal-ledger.conf` on Linux),
//! created with its directory when it doesn't exist. It is the one file that is the person's own
//! on every install: the system file needs root, and the working-directory file is the repository's
//! example when a binary runs from a checkout. Every source above it in the hierarchy (the
//! executable-directory, working-directory and `--config` files, then the environment and the
//! flags) still wins, so the writer carries the parse's [`Overrides`], which name the one that
//! does, for Settings to say so rather than appear to save.
//!
//! The file is edited line by line rather than round-tripped through an INI parser: neither the
//! `config` crate nor `rust-ini` keeps comments on write, and the person's comments, key order and
//! other sections must survive a write untouched. Only the one key's line changes, or one line is
//! added for a new key, or one line removed.

use std::path::{Path, PathBuf};

use crate::keys::{ConfigKey, ConfigValue};
use crate::overrides::Overrides;

/// Where Settings writes Configuration, and what outranks it there: the overrides report the parse
/// recorded as it applied precedence.
#[derive(Debug, Clone, Default)]
pub struct ConfigWriter {
    /// The user's file; `None` where nothing is written (the Sync Server, or no home directory).
    target: Option<PathBuf>,
    /// Each key a source above the target sets, recorded by the parse and the flags.
    pub(crate) overrides: Overrides,
}

impl ConfigWriter {
    /// A writer to `target` that `overrides`' sources outrank. [`crate::Config::parse`] builds the
    /// Client's; a test builds one on a temp file.
    pub fn new(target: Option<PathBuf>, overrides: Overrides) -> Self {
        Self { target, overrides }
    }

    /// Each key a source above the target sets, with the highest that does: what a Settings row
    /// names when its write would change nothing.
    pub fn overrides(&self) -> &Overrides {
        &self.overrides
    }

    /// The file a write goes to.
    pub fn target(&self) -> Option<&Path> {
        self.target.as_deref()
    }

    /// The value the target file itself sets `key` to, quotes taken off, whatever outranks it;
    /// `None` when the file doesn't set it, doesn't exist or can't be read, or there is no target.
    /// Settings › General's Locale shows it, since a write goes there even while shadowed.
    pub fn value(&self, key: ConfigKey) -> Option<String> {
        let text = std::fs::read_to_string(self.target.as_deref()?).ok()?;
        key_value(&text, key)
    }

    /// The value the target file itself sets `key` to, read by the parser from that file alone,
    /// with nothing above it and no environment, as a restart with only that file would read it;
    /// `None` when the file doesn't set the key in its section, doesn't exist, or doesn't parse.
    pub fn stored(&self, key: ConfigKey) -> Option<ConfigValue> {
        let path = self.target.as_deref()?;
        if !crate::Config::keys_in_file(path).ok()?.contains(&key.id()) {
            return None;
        }
        let config = crate::Config::parse_file_alone(path).ok()?;
        ConfigValue::read(key, &config)
    }

    /// Writes `value` under its key in its section of the target file, spelt as the parser reads
    /// it back, keeping the rest of the file as it was.
    ///
    /// # Errors
    /// Returns [`crate::Error::Validation`] when there is no target,
    /// [`crate::Error::InvalidLocale`] for a Locale that isn't a well-formed tag, which the parser
    /// would refuse at the next start, and [`crate::Error::Write`] when the file or its directory
    /// can't be read or written.
    pub fn set(&self, value: ConfigValue) -> crate::Result<()> {
        let path = self.target_file()?;
        let value = match value {
            ConfigValue::Locale(tag) => {
                ConfigValue::Locale(crate::general::parse_locale_tag(&tag)?)
            }
            other => other,
        };
        let escaped = escape(&value.spelt());
        edit(path, |text| Some(set_key(text, value.key(), &escaped)))
    }

    /// Removes `key`'s line from its section of the target file, keeping every other byte, so the
    /// key falls back to the sources below: Settings › General's Locale **System**. A file that
    /// doesn't exist or doesn't set the key is left alone.
    ///
    /// # Errors
    /// As [`Self::set`].
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

/// The value `text` sets `key` to in its section, the last line winning as the parser reads it,
/// with surrounding quotes taken off.
fn key_value(text: &str, key: ConfigKey) -> Option<String> {
    let mut in_section = false;
    let mut value = None;
    for line in text.lines() {
        if let Some(name) = section_name(line) {
            in_section = name == key.section().normalised();
            continue;
        }
        if !in_section || !line_key(line).is_some_and(|name| name.eq_ignore_ascii_case(key.name()))
        {
            continue;
        }
        if let Some((_, raw)) = line.split_once(['=', ':']) {
            let raw = raw.trim();
            let unquoted = raw
                .strip_prefix('"')
                .and_then(|inner| inner.strip_suffix('"'))
                .unwrap_or(raw);
            value = Some(unescape(unquoted));
        }
    }
    value
}

/// `value` as the parser reads it back: the INI parser takes `\` as an escape, so a Windows
/// path's `C:\Users\me\new` would otherwise come back with a newline in it.
fn escape(value: &str) -> String {
    value.replace('\\', "\\\\")
}

/// `raw` with the parser's escapes undone, the way `rust-ini` reads them, so [`ConfigWriter::value`]
/// shows what the parser takes: `\0`, `\a`, `\b`, `\t`, `\r`, `\n`, `\xHHHH`, and any other
/// escaped character as itself.
fn unescape(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            None => out.push('\\'),
            Some('0') => out.push('\0'),
            Some('a') => out.push('\x07'),
            Some('b') => out.push('\x08'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some('n') => out.push('\n'),
            Some('x') => {
                let code: String = chars.by_ref().take(4).collect();
                if let Some(decoded) = u32::from_str_radix(&code, 16).ok().and_then(char::from_u32)
                {
                    out.push(decoded);
                }
            }
            Some(other) => out.push(other),
        }
    }
    out
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
    use std::collections::HashMap;
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
        let writer = ConfigWriter::new(Some(path.clone()), Overrides::default());
        writer.set(ConfigValue::HintBar(false)).unwrap();
        writer.set(ConfigValue::Toasts(false)).unwrap();
        writer
            .set(ConfigValue::StatusGlyphs(
                crate::StatusGlyphs::AsciiFallback,
            ))
            .unwrap();
        writer
            .set(ConfigValue::Log(lib_tracing::Levels::WARN))
            .unwrap();
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

    /// Every value each key Settings writes can take, open-ended ones by example: a Windows path
    /// among the folders, and every built-in Colour Theme.
    fn every_value() -> Vec<ConfigValue> {
        use lib_colour_theme::{ColourAppearance, ColourTheme};
        use lib_core::StartView;
        let mut values = Vec::new();
        for on in [true, false] {
            values.extend([
                ConfigValue::Toasts(on),
                ConfigValue::HintBar(on),
                ConfigValue::TerminalColours(on),
            ]);
        }
        values.extend(crate::ON_STARTS.map(ConfigValue::OnStart));
        values.extend(
            [
                StartView::Dashboard,
                StartView::Last,
                StartView::Settings,
                StartView::Noun("units".into()),
            ]
            .map(ConfigValue::StartView),
        );
        values.extend(["en-GB", "en-US", "en-AU"].map(|tag| ConfigValue::Locale(tag.into())));
        for folder in [
            "/tmp/ledgers",
            r"C:\Users\me\new",
            "/tmp/with space/ledgers",
        ] {
            values.push(ConfigValue::Data(folder.into()));
            values.push(ConfigValue::LogFilePath(folder.into()));
        }
        values.extend(
            ColourTheme::built_in()
                .iter()
                .map(|theme| ConfigValue::ColourTheme(theme.id.into())),
        );
        values.extend(
            [
                ColourAppearance::Light,
                ColourAppearance::Dark,
                ColourAppearance::System,
            ]
            .map(ConfigValue::ColourAppearance),
        );
        values.extend(crate::DATE_STYLES.map(ConfigValue::DateStyle));
        values.extend(crate::RowDensity::ALL.map(ConfigValue::RowDensity));
        values.extend(crate::StatusGlyphs::ALL.map(ConfigValue::StatusGlyphs));
        values.extend(
            [
                lib_tracing::Levels::OFF,
                lib_tracing::Levels::ERROR,
                lib_tracing::Levels::WARN,
                lib_tracing::Levels::INFO,
                lib_tracing::Levels::DEBUG,
                lib_tracing::Levels::TRACE,
            ]
            .map(ConfigValue::Log),
        );
        values
    }

    #[test]
    fn every_value_of_every_key_is_written_and_read_back_by_the_parser() {
        let values = every_value();
        for key in ConfigKey::ALL {
            assert!(
                values.iter().any(|value| value.key() == key),
                "{key:?} has no value in the round trip"
            );
        }
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("personal-ledger.conf");
        let writer = ConfigWriter::new(Some(path.clone()), Overrides::default());
        for value in values {
            writer.set(value.clone()).unwrap();
            let config =
                crate::Config::parse_with_detector(Some(&path), Some(HashMap::new()), || None)
                    .unwrap();
            assert_eq!(
                ConfigValue::read(value.key(), &config),
                Some(value.clone()),
                "read back through the parser"
            );
            assert_eq!(
                writer.stored(value.key()),
                Some(value),
                "read back as stored"
            );
        }
    }

    #[test]
    fn a_locale_the_parser_would_refuse_is_not_written_and_a_tag_is_written_canonically() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("personal-ledger.conf");
        let writer = ConfigWriter::new(Some(path.clone()), Overrides::default());
        assert!(matches!(
            writer.set(ConfigValue::Locale("not a tag!".into())),
            Err(crate::Error::InvalidLocale { .. })
        ));
        assert!(!path.exists());
        writer.set(ConfigValue::Locale("en-gb".into())).unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "[General]\nlocale = en-GB\n"
        );
    }

    #[test]
    fn stored_reads_only_the_key_s_own_section_of_the_target() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("personal-ledger.conf");
        let writer = ConfigWriter::new(Some(path.clone()), Overrides::default());
        assert_eq!(writer.stored(ConfigKey::Toasts), None, "no file yet");
        std::fs::write(
            &path,
            "[Display]\ntoasts = false\n[General]\non_start = none\n",
        )
        .unwrap();
        assert_eq!(writer.stored(ConfigKey::Toasts), None);
        assert_eq!(
            writer.stored(ConfigKey::OnStart),
            Some(ConfigValue::OnStart(lib_core::OnStart::NoLedger))
        );
    }

    #[test]
    fn a_windows_data_folder_survives_the_parser_and_reads_back() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("personal-ledger.conf");
        let writer = ConfigWriter::new(Some(path.clone()), Overrides::default());
        let folder = r"C:\Users\me\new";
        writer.set(ConfigValue::Data(folder.into())).unwrap();

        let config =
            crate::Config::parse_with_detector(Some(&path), Some(HashMap::new()), || None).unwrap();
        assert_eq!(config.general_config().data(), Path::new(folder));
        assert_eq!(writer.value(ConfigKey::Data).as_deref(), Some(folder));
    }

    #[test]
    fn value_undoes_the_parser_s_escapes_as_the_parser_does() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("personal-ledger.conf");
        std::fs::write(&path, "[General]\ndata = C:\\Users\\x0041\\\\b\n").unwrap();
        let writer = ConfigWriter::new(Some(path.clone()), Overrides::default());
        let config =
            crate::Config::parse_with_detector(Some(&path), Some(HashMap::new()), || None).unwrap();
        let parsed = config.general_config().data().display().to_string();
        assert_eq!(writer.value(ConfigKey::Data), Some(parsed));
    }

    #[test]
    fn removing_the_locale_deletes_its_line_and_leaves_every_other_byte() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("personal-ledger.conf");
        let before = "# notes\r\n[General]\r\ntoasts = false\r\n  locale = en-GB\r\n\
                      # locale = en-AU\r\non_start = none\r\n\r\n[Other]\r\nlocale = fr\r\n";
        std::fs::write(&path, before).unwrap();
        let writer = ConfigWriter::new(Some(path.clone()), Overrides::default());
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
    fn value_reads_the_key_from_its_own_section_of_the_target_only() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("personal-ledger.conf");
        let writer = ConfigWriter::new(Some(path.clone()), Overrides::default());
        assert_eq!(writer.value(ConfigKey::Locale), None, "no file yet");

        std::fs::write(
            &path,
            "[Other]\nlocale = fr\n[General]\n# locale = en-AU\n  locale = \"en-GB\"\n",
        )
        .unwrap();
        assert_eq!(writer.value(ConfigKey::Locale).as_deref(), Some("en-GB"));
        assert_eq!(writer.value(ConfigKey::Toasts), None);
        assert_eq!(ConfigWriter::default().value(ConfigKey::Locale), None);
    }

    #[test]
    fn removing_from_a_missing_file_creates_nothing_and_with_no_target_is_refused() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("personal-ledger.conf");
        ConfigWriter::new(Some(path.clone()), Overrides::default())
            .remove(ConfigKey::Locale)
            .unwrap();
        assert!(!path.exists());
        assert!(matches!(
            ConfigWriter::default().remove(ConfigKey::Locale),
            Err(crate::Error::Validation(_))
        ));
    }

    #[test]
    fn an_existing_file_keeps_its_comments_and_other_sections() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("personal-ledger.conf");
        std::fs::write(&path, WRITTEN).unwrap();
        let writer = ConfigWriter::new(Some(path.clone()), Overrides::default());
        writer.set(ConfigValue::TerminalColours(true)).unwrap();
        writer
            .set(ConfigValue::StartView(lib_core::StartView::Last))
            .unwrap();
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
            ConfigWriter::default().set(ConfigValue::HintBar(true)),
            Err(crate::Error::Validation(_))
        ));
    }
}
