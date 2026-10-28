//! Writes Configuration back to `personal-ledger.conf` (#904), for the Settings rows that edit it.
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
//! added for a new key.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// The section Settings writes, as `normalise_ini` reads its header.
const SECTION: &str = "personal_ledger";

/// The `[Personal-Ledger]` keys Settings writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigKey {
    /// `terminal_colours`: draw with the terminal's own colours (ADR-0024).
    TerminalColours,
    /// `status_glyphs`: `unicode` or `ascii`.
    StatusGlyphs,
    /// `hint_bar`: whether the footer shows the key hints.
    HintBar,
}

impl ConfigKey {
    /// The key as it is written in the file.
    pub fn name(self) -> &'static str {
        match self {
            Self::TerminalColours => "terminal_colours",
            Self::StatusGlyphs => "status_glyphs",
            Self::HintBar => "hint_bar",
        }
    }

    /// The environment variable that sets the key, by `add_explicit_and_env`'s naming.
    pub fn env_var(self) -> String {
        format!(
            "{}_{}__{}",
            crate::ledger::LedgerConfig::env_prefix(),
            SECTION.to_uppercase(),
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
            .find(|path| {
                std::fs::read_to_string(path).is_ok_and(|text| sets_key(&text, key.name()))
            })
            .cloned()
            .map(Shadow::File)
    }

    /// Writes `value` to `key` in the target file, keeping the rest of the file as it was.
    ///
    /// # Errors
    /// Returns [`crate::Error::Validation`] when there is no target, and [`crate::Error::Write`]
    /// when the file or its directory can't be read or written.
    pub fn write(&self, key: ConfigKey, value: &str) -> crate::Result<()> {
        let path = self.target.as_deref().ok_or_else(|| {
            crate::Error::Validation("no user configuration file to write to".into())
        })?;
        write_key(path, key.name(), value)
    }
}

/// Sets `key = value` in `path`'s `[Personal-Ledger]` section, creating the file, its directory
/// and the section as needed.
fn write_key(path: &Path, key: &str, value: &str) -> crate::Result<()> {
    let io = |source| crate::Error::Write {
        path: path.to_path_buf(),
        source,
    };
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(io(error)),
    };
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(io)?;
    }
    // Written beside the file then renamed over it, so a failed write never leaves half a file.
    let partial = path.with_extension("conf.partial");
    std::fs::write(&partial, set_key(&text, key, value)).map_err(io)?;
    std::fs::rename(&partial, path).map_err(io)
}

/// `text` with `key = value` in its `[Personal-Ledger]` section: every line setting the key
/// rewritten in place, else one line added after the section's last entry, else the section
/// appended. Every other line is kept as it was.
fn set_key(text: &str, key: &str, value: &str) -> String {
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let entry = format!("{key} = {value}");

    let mut in_section = false;
    let mut found = false;
    // The index after the section's header or last entry, where a new key goes: not after a
    // trailing comment, which more likely heads the next section.
    let mut insert_at = None;
    for (index, line) in lines.iter_mut().enumerate() {
        if let Some(name) = section_name(line) {
            in_section = name == SECTION;
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
        if name.eq_ignore_ascii_case(key) {
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
                lines.push("[Personal-Ledger]".into());
                lines.push(entry);
            }
        }
    }

    let mut out = lines.join("\n");
    out.push('\n');
    out
}

/// Whether `text` sets `key` in its `[Personal-Ledger]` section.
fn sets_key(text: &str, key: &str) -> bool {
    let mut in_section = false;
    text.lines().any(|line| {
        if let Some(name) = section_name(line) {
            in_section = name == SECTION;
            return false;
        }
        in_section && line_key(line).is_some_and(|name| name.eq_ignore_ascii_case(key))
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
[Personal-Ledger]
log = \"debug\"
  terminal_colours = false

; Another section, untouched.
[keybindings]
back = \"esc\"
";

    #[test]
    fn an_existing_key_is_rewritten_in_place_and_nothing_else_changes() {
        assert_eq!(
            set_key(WRITTEN, "terminal_colours", "true"),
            WRITTEN.replace("  terminal_colours = false", "  terminal_colours = true")
        );
    }

    #[test]
    fn a_new_key_joins_the_end_of_its_section() {
        assert_eq!(
            set_key(WRITTEN, "hint_bar", "false"),
            WRITTEN.replace(
                "  terminal_colours = false\n",
                "  terminal_colours = false\nhint_bar = false\n"
            )
        );
    }

    #[test]
    fn a_missing_section_is_appended_and_any_header_spelling_counts() {
        assert_eq!(
            set_key("[keybindings]\nback = \"esc\"", "status_glyphs", "ascii"),
            "[keybindings]\nback = \"esc\"\n\n[Personal-Ledger]\nstatus_glyphs = ascii\n"
        );
        assert_eq!(
            set_key("[personal_ledger]\n", "hint_bar", "true"),
            "[personal_ledger]\nhint_bar = true\n"
        );
        assert_eq!(
            set_key("", "hint_bar", "true"),
            "[Personal-Ledger]\nhint_bar = true\n"
        );
    }

    #[test]
    fn a_commented_out_key_or_one_in_another_section_is_left_alone() {
        let text = "[Personal-Ledger]\n# hint_bar = true\n[other]\nhint_bar = true\n";
        assert_eq!(
            set_key(text, "hint_bar", "false"),
            "[Personal-Ledger]\nhint_bar = false\n# hint_bar = true\n[other]\nhint_bar = true\n"
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
        writer.write(ConfigKey::StatusGlyphs, "ascii").unwrap();
        writer.write(ConfigKey::TerminalColours, "true").unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "[Personal-Ledger]\nhint_bar = false\nstatus_glyphs = ascii\nterminal_colours = true\n"
        );
        assert!(!path.with_extension("conf.partial").exists());

        let config =
            crate::Config::parse_with_detector(Some(&path), Some(HashMap::new()), || None).unwrap();
        let section = config.personal_ledger_config();
        assert!(section.terminal_colours());
        assert!(!section.hint_bar());
        assert_eq!(section.status_glyphs(), crate::StatusGlyphs::Ascii);
    }

    #[test]
    fn an_existing_file_keeps_its_comments_and_other_keys() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("personal-ledger.conf");
        std::fs::write(&path, WRITTEN).unwrap();
        ConfigWriter::new(Some(path.clone()), Vec::new(), [])
            .write(ConfigKey::TerminalColours, "true")
            .unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(
            text.starts_with(
                "# The person's own notes, kept.\n[Personal-Ledger]\nlog = \"debug\"\n"
            )
        );
        assert!(text.contains("  terminal_colours = true\n"));
        assert!(text.ends_with("; Another section, untouched.\n[keybindings]\nback = \"esc\"\n"));
    }

    #[test]
    fn with_no_target_a_write_is_refused() {
        assert!(matches!(
            ConfigWriter::default().write(ConfigKey::HintBar, "true"),
            Err(crate::Error::Validation(_))
        ));
    }

    #[test]
    fn an_environment_variable_or_a_higher_file_shadows_the_target() {
        let dir = TempDir::new().unwrap();
        let cwd = dir.path().join("cwd.conf");
        let explicit = dir.path().join("explicit.conf");
        let missing = dir.path().join("missing.conf");
        std::fs::write(
            &cwd,
            "[personal-ledger]\nhint_bar = false\nstatus_glyphs = ascii\n",
        )
        .unwrap();
        std::fs::write(&explicit, "[Personal-Ledger]\nhint_bar = true\n").unwrap();
        let writer = ConfigWriter::new(
            Some(dir.path().join("user.conf")),
            vec![cwd.clone(), explicit.clone(), missing],
            [
                (
                    "personal_ledger_personal_ledger__terminal_colours".into(),
                    "true".into(),
                ),
                ("HOME".into(), "/home/someone".into()),
            ],
        );
        assert_eq!(
            writer.shadow(ConfigKey::TerminalColours),
            Some(Shadow::Env(
                "PERSONAL_LEDGER_PERSONAL_LEDGER__TERMINAL_COLOURS".into()
            ))
        );
        assert_eq!(
            writer.shadow(ConfigKey::HintBar),
            Some(Shadow::File(explicit))
        );
        assert_eq!(
            writer.shadow(ConfigKey::StatusGlyphs),
            Some(Shadow::File(cwd))
        );
        assert_eq!(
            ConfigWriter::new(Some(dir.path().join("user.conf")), Vec::new(), [])
                .shadow(ConfigKey::HintBar),
            None
        );
    }
}
