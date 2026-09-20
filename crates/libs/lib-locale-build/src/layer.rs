//! Loads and checks one layer of Catalogue files.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use fluent_syntax::ast::Entry;
use fluent_syntax::parser;

use crate::SOURCE_LOCALE;
use crate::error::{Error, Result};
use crate::extract::{self, Kind, Params};

/// Prefixes reserved for a bin's own layer, which the shared layer must not use.
const BIN_PREFIXES: [&str; 2] = ["desktop-", "tui-"];

/// One Message value or attribute, as an accessor sees it.
#[derive(Debug, Clone)]
pub(crate) struct Item {
    /// The Message id.
    pub id: String,

    /// The attribute name, when this item is an attribute.
    pub attribute: Option<String>,

    pub params: Params,

    /// The `<tag>` names the text uses.
    pub tags: BTreeSet<String>,

    /// Whether the accessor returns segments (tags, or a `# @rich` comment on the Message)
    /// rather than a plain `String`.
    pub rich: bool,
}

impl Item {
    fn key(&self) -> String {
        match &self.attribute {
            Some(attribute) => format!("{}.{attribute}", self.id),
            None => self.id.clone(),
        }
    }

    /// The accessor function name: kebab-case to snake_case, attributes joined with `_`.
    pub fn accessor(&self) -> String {
        self.key().replace(['-', '.'], "_")
    }
}

/// A file's source, kept for embedding.
#[derive(Debug)]
pub(crate) struct File {
    pub name: String,
    pub path: PathBuf,
}

/// Every Locale in a layer, with its files and items.
#[derive(Debug)]
pub(crate) struct Layer {
    pub locales: BTreeMap<String, LocaleFiles>,
}

#[derive(Debug, Default)]
pub(crate) struct LocaleFiles {
    pub files: Vec<File>,
    pub items: BTreeMap<String, Item>,
}

impl Layer {
    /// Reads `<dir>/<locale>/*.ftl` and runs every check, returning all problems at once.
    pub fn load(dir: &Path, id_prefix: Option<&str>) -> Result<Layer> {
        let mut problems = Vec::new();
        let mut locales = BTreeMap::new();

        let mut locale_dirs: Vec<PathBuf> = Vec::new();
        for entry in fs::read_dir(dir)? {
            let path = entry?.path();
            if path.is_dir() {
                locale_dirs.push(path);
            }
        }
        locale_dirs.sort();

        for locale_dir in locale_dirs {
            let locale = file_name(&locale_dir);
            if locale == "en-XA" {
                problems.push(format!(
                    "{}: en-XA is generated from en-US at load time and must have no Catalogue files",
                    locale_dir.display()
                ));
                continue;
            }
            locales.insert(locale, load_locale(&locale_dir, &mut problems)?);
        }

        let layer = Layer { locales };
        layer.check(id_prefix, &mut problems);

        if problems.is_empty() {
            Ok(layer)
        } else {
            Err(Error::Generic(problems.join("\n")))
        }
    }

    /// The source Locale's items, which decide the accessors generated.
    pub fn source(&self) -> Option<&LocaleFiles> {
        self.locales.get(SOURCE_LOCALE)
    }

    fn check(&self, id_prefix: Option<&str>, problems: &mut Vec<String>) {
        let Some(source) = self.source() else {
            problems.push(format!(
                "missing the source Locale directory `{SOURCE_LOCALE}`"
            ));
            return;
        };

        for (locale, files) in &self.locales {
            for item in files.items.values() {
                match id_prefix {
                    Some(prefix) if !item.id.starts_with(prefix) => problems.push(format!(
                        "{locale}: id `{}` must start with `{prefix}`",
                        item.id
                    )),
                    None if BIN_PREFIXES
                        .iter()
                        .any(|prefix| item.id.starts_with(prefix)) =>
                    {
                        problems.push(format!(
                            "{locale}: id `{}` uses a bin-only prefix in the shared layer",
                            item.id
                        ))
                    }
                    _ => {}
                }
            }

            if locale == SOURCE_LOCALE {
                continue;
            }
            for (key, item) in &files.items {
                let Some(source_item) = source.items.get(key) else {
                    problems.push(format!("{locale}: `{key}` is not in {SOURCE_LOCALE}"));
                    continue;
                };
                compare_params(locale, key, &source_item.params, &item.params, problems);
                if source_item.tags != item.tags {
                    problems.push(format!(
                        "{locale}: `{key}` uses tags {:?} but {SOURCE_LOCALE} uses {:?}",
                        item.tags, source_item.tags
                    ));
                }
            }
        }

        let mut names: BTreeMap<String, String> = BTreeMap::new();
        for (key, item) in &source.items {
            let accessor = item.accessor();
            if is_keyword(&accessor) {
                problems.push(format!(
                    "`{key}` gives the accessor `{accessor}`, a Rust keyword"
                ));
            }
            for param in item.params.keys() {
                if is_keyword(&snake(param)) {
                    problems.push(format!(
                        "`{key}` uses `${param}`, a Rust keyword as a parameter"
                    ));
                }
            }
            if let Some(other) = names.insert(accessor.clone(), key.clone()) {
                problems.push(format!(
                    "`{other}` and `{key}` both give the accessor `{accessor}`"
                ));
            }
        }
    }
}

pub(crate) fn snake(name: &str) -> String {
    name.replace('-', "_")
}

fn compare_params(locale: &str, key: &str, source: &Params, other: &Params, out: &mut Vec<String>) {
    let missing: Vec<_> = source
        .keys()
        .filter(|name| !other.contains_key(*name))
        .collect();
    let extra: Vec<_> = other
        .keys()
        .filter(|name| !source.contains_key(*name))
        .collect();
    if !missing.is_empty() || !extra.is_empty() {
        out.push(format!(
            "{locale}: `{key}` variables differ from {SOURCE_LOCALE} (missing {missing:?}, extra {extra:?})"
        ));
        return;
    }
    for (name, kind) in source.iter() {
        if other.get(name) != Some(*kind) {
            let describe = |kind: &Kind| match kind {
                Kind::Number => "a plural selector",
                Kind::Text => "plain text",
            };
            out.push(format!(
                "{locale}: `{key}` uses `${name}` as {} but {SOURCE_LOCALE} uses it as {}",
                other.get(name).as_ref().map_or("nothing", describe),
                describe(kind)
            ));
        }
    }
}

fn load_locale(dir: &Path, problems: &mut Vec<String>) -> Result<LocaleFiles> {
    let mut paths: Vec<PathBuf> = Vec::new();
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.extension().is_some_and(|extension| extension == "ftl") {
            paths.push(path);
        }
    }
    paths.sort();

    let mut locale = LocaleFiles::default();
    for path in paths {
        let source = fs::read_to_string(&path)?;
        let resource = match parser::parse(source.as_str()) {
            Ok(resource) => resource,
            Err((resource, errors)) => {
                for error in errors {
                    let line = source[..error.pos.start.min(source.len())]
                        .matches('\n')
                        .count()
                        + 1;
                    problems.push(format!("{}:{line}: {}", path.display(), error.kind));
                }
                resource
            }
        };

        for entry in &resource.body {
            let Entry::Message(message) = entry else {
                continue;
            };
            let id = message.id.name.to_string();
            let marked_rich = message
                .comment
                .as_ref()
                .is_some_and(|comment| comment.content.iter().any(|line| line.contains("@rich")));
            let mut add = |attribute: Option<String>,
                           pattern: &fluent_syntax::ast::Pattern<&str>| {
                let mut params = Params::new();
                extract::pattern(pattern, &mut params);
                let tags = match extract::tags(pattern) {
                    Ok(tags) => tags,
                    Err(problem) => {
                        problems.push(format!("{}: `{id}`: {problem}", path.display()));
                        BTreeSet::new()
                    }
                };
                let rich = marked_rich || !tags.is_empty();
                let item = Item {
                    id: id.clone(),
                    attribute,
                    params,
                    tags,
                    rich,
                };
                let key = item.key();
                if locale.items.insert(key.clone(), item).is_some() {
                    problems.push(format!(
                        "{}: `{key}` is defined more than once",
                        path.display()
                    ));
                }
            };
            if let Some(value) = &message.value {
                add(None, value);
            }
            for attribute in &message.attributes {
                add(Some(attribute.id.name.to_string()), &attribute.value);
            }
        }

        let name = file_name(&path);
        locale.files.push(File { name, path });
    }
    Ok(locale)
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn is_keyword(name: &str) -> bool {
    const KEYWORDS: [&str; 52] = [
        "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum",
        "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move",
        "mut", "pub", "ref", "return", "self", "static", "struct", "super", "trait", "true",
        "type", "unsafe", "use", "where", "while", "abstract", "become", "box", "do", "final",
        "macro", "override", "priv", "try", "typeof", "unsized", "virtual", "yield", "gen", "Self",
    ];
    KEYWORDS.contains(&name)
}
