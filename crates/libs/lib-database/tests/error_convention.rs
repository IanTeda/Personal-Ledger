//! Enforces ADR-0060 and the `error.rs` convention across the whole workspace, every library crate
//! (`crates/libs/*`) and every bin crate (`crates/bins/*`). It lives in `lib-database` beside
//! `viper_dependencies.rs` because this is the one crate that already looks across the others,
//! not because the rule is about persistence.
//!
//! Two kinds of check:
//!
//! - **Structural**, on every crate: a `src/error.rs` holding exactly
//!   `pub type Result<T, E = Error> = std::result::Result<T, E>;` and no other `Result` alias in
//!   the crate, declared from `lib.rs` as a private `mod error` and re-exported as
//!   `pub use error::{Error, Result}`.
//! - **Usage** (ADR-0060), on every crate not in [`PENDING`]: a `Result` in a return type with two
//!   arguments fails unless its error is on [`VALUE_ERRORS`], a path-qualified `Result` fails
//!   unless it is `crate::Result`, `std::result::Result` appears nowhere but the alias, and no
//!   `use` brings in another crate's `Result`, under any name or through a glob. A package's own
//!   lib target counts as the crate itself, so a bin's `main.rs` may `use bin_x::Result`.
//!
//! The source is parsed with `syn`, so the test sees how a signature is spelt, not what it
//! resolves to. Signatures fixed from outside are recognised by that spelling: a method inside
//! `impl Trait for Type`, an error that is a generic parameter or its associated type (`D::Error`),
//! and `fmt::Result`. It follows the module tree from `lib.rs` and `main.rs`, so `tests/`,
//! `benches/`, `examples/`, `build.rs`, `#[cfg(test)]` items and `generated/` are never read.
//! A function written inside a `macro_rules!` body is a known gap: `syn` keeps a macro's body as
//! tokens, so the test never sees it.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "integration test crate: a failed setup should fail the test"
)]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use quote::ToTokens;
use syn::visit::{self, Visit};

/// The value errors (ADR-0060): each an enum beside one value type, naming only the ways a text
/// or number fails to become that value, so any crate may return it. Adding one is a reviewed
/// edit, judged against that definition.
const VALUE_ERRORS: &[&str] = &[
    "AccountTypeError",
    "BudgetPeriodError",
    "CategoryTypesError",
    "CoverError",
    "DateInputError",
    "DateStyleError",
    "HexColorError",
    "HybridLogicalClockError",
    "MoneyError",
    "NameError",
    "ParseError",
    "ParseLevelError",
    "RowIDError",
    "TransactionStatusError",
    "UnitKindError",
    "UrlSlugError",
];

/// The crates that still break the usage rules, by directory name: the ratchet of the Result map
/// (#1309). Each crate's slice deletes its own entry, a listed crate that passes fails the test
/// as stale, and the last slice deletes the list.
const PENDING: &[&str] = &["bin-desktop", "bin-sync-server"];

/// The one shape every crate's alias takes, as `syn` prints it.
const ALIAS: &str = "pub type Result < T , E = Error > = std :: result :: Result < T , E > ;";

/// A crate under check: its directory, the name it is reported by, the names its own source may
/// use for itself and the other crates a `use` may start from.
struct Crate {
    dir: PathBuf,
    name: String,
    own_names: BTreeSet<String>,
    other_crates: BTreeSet<String>,
}

impl Crate {
    fn at(dir: &Path) -> Self {
        let name = dir
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string();
        let manifest = fs::read_to_string(dir.join("Cargo.toml")).unwrap_or_default();
        let mut own_names: BTreeSet<String> = ["crate", "self", "super", "Self"]
            .into_iter()
            .map(String::from)
            .collect();
        own_names.insert(name.replace('-', "_"));
        own_names.extend(manifest_names(&manifest));
        let mut other_crates: BTreeSet<String> = ["std", "core", "alloc"]
            .into_iter()
            .map(String::from)
            .collect();
        other_crates.extend(manifest_dependencies(&manifest));
        Self {
            dir: dir.to_path_buf(),
            name,
            own_names,
            other_crates,
        }
    }
}

/// The `name`s under `[package]` and `[lib]`, with `-` read as `_` as rustc reads them.
fn manifest_names(manifest: &str) -> Vec<String> {
    let mut section = "";
    let mut names = Vec::new();
    for line in manifest.lines().map(str::trim) {
        if line.starts_with('[') {
            section = line;
        } else if (section == "[package]" || section == "[lib]")
            && let Some(value) = line
                .strip_prefix("name")
                .map(str::trim_start)
                .and_then(|rest| rest.strip_prefix('='))
        {
            names.push(value.trim().trim_matches('"').replace('-', "_"));
        }
    }
    names
}

/// The crates the manifest's dependency tables name, as source spells them. `syn` can't tell a
/// crate from a local module at the start of a `use` path, so the import rule asks this.
fn manifest_dependencies(manifest: &str) -> Vec<String> {
    let mut in_dependencies = false;
    let mut names = Vec::new();
    for line in manifest.lines().map(str::trim) {
        if line.starts_with('[') {
            in_dependencies = line.ends_with("dependencies]") && !line.contains("dev-");
        } else if in_dependencies
            && let Some(key) = line.split(['=', '.']).next().map(str::trim)
            && !key.is_empty()
            && !key.starts_with('#')
        {
            names.push(key.replace('-', "_"));
        }
    }
    names
}

/// Every library and bin crate's directory, sorted.
fn workspace_crates() -> Vec<PathBuf> {
    let crates_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("lib-database sits at crates/libs/lib-database");
    let mut crates: Vec<PathBuf> = ["libs", "bins"]
        .iter()
        .flat_map(|group| fs::read_dir(crates_dir.join(group)).into_iter().flatten())
        .filter_map(|entry| Some(entry.ok()?.path()))
        .filter(|path| path.join("Cargo.toml").is_file())
        .collect();
    crates.sort();
    crates
}

fn is_cfg_test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path().is_ident("cfg")
            && attr.parse_args::<syn::Meta>().is_ok_and(|meta| {
                match meta {
                syn::Meta::Path(path) => path.is_ident("test"),
                syn::Meta::List(list) => {
                    list.path.is_ident("all")
                        && list.tokens.clone().into_iter().any(|token| {
                            matches!(token, proc_macro2::TokenTree::Ident(ident) if ident == "test")
                        })
                }
                syn::Meta::NameValue(_) => false,
            }
            })
    })
}

fn path_attr(attrs: &[syn::Attribute]) -> Option<String> {
    attrs.iter().find_map(|attr| match &attr.meta {
        syn::Meta::NameValue(pair) if pair.path.is_ident("path") => match &pair.value {
            syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(text),
                ..
            }) => Some(text.value()),
            _ => None,
        },
        _ => None,
    })
}

/// The file a `mod name;` declared with `attrs` inside `module_dir` loads.
fn module_file(module_dir: &Path, name: &str, attrs: &[syn::Attribute]) -> Option<PathBuf> {
    if let Some(path) = path_attr(attrs) {
        return Some(module_dir.join(path));
    }
    [
        module_dir.join(format!("{name}.rs")),
        module_dir.join(name).join("mod.rs"),
    ]
    .into_iter()
    .find(|path| path.is_file())
}

/// Adds to `out` the files the module tree under `items` loads, skipping `#[cfg(test)]` and
/// `generated` modules.
fn collect_modules(items: &[syn::Item], module_dir: &Path, out: &mut Vec<(PathBuf, syn::File)>) {
    for item in items {
        let syn::Item::Mod(module) = item else {
            continue;
        };
        let name = module.ident.to_string();
        if is_cfg_test(&module.attrs) || name == "generated" {
            continue;
        }
        match &module.content {
            Some((_, inner)) => collect_modules(inner, &module_dir.join(&name), out),
            None => {
                if let Some(file) = module_file(module_dir, &name, &module.attrs) {
                    load(&file, false, out);
                }
            }
        }
    }
}

/// Parses `file` and the modules it loads into `out`. A crate root's modules sit beside it; any
/// other file's sit in the directory named after it.
fn load(file: &Path, is_root: bool, out: &mut Vec<(PathBuf, syn::File)>) {
    if out.iter().any(|(seen, _)| seen == file) {
        return;
    }
    let source = fs::read_to_string(file).unwrap_or_default();
    let parsed = syn::parse_file(&source)
        .unwrap_or_else(|error| panic!("{} should parse: {error}", file.display()));
    let module_dir = match file.file_name().and_then(|name| name.to_str()) {
        _ if is_root => file.parent().map(Path::to_path_buf),
        Some("mod.rs") => file.parent().map(Path::to_path_buf),
        _ => file
            .parent()
            .zip(file.file_stem())
            .map(|(dir, stem)| dir.join(stem)),
    }
    .unwrap_or_default();
    out.push((file.to_path_buf(), parsed.clone()));
    collect_modules(&parsed.items, &module_dir, out);
}

/// Every source file the crate's lib and bin targets compile, parsed.
fn crate_sources(dir: &Path) -> Vec<(PathBuf, syn::File)> {
    let mut out = Vec::new();
    let mut roots = vec![dir.join("src/lib.rs"), dir.join("src/main.rs")];
    let mut bins: Vec<PathBuf> = fs::read_dir(dir.join("src/bin"))
        .into_iter()
        .flatten()
        .filter_map(|entry| Some(entry.ok()?.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "rs"))
        .collect();
    bins.sort();
    roots.extend(bins);
    for root in roots.into_iter().filter(|root| root.is_file()) {
        load(&root, true, &mut out);
    }
    out
}

fn spelt(tokens: &impl ToTokens) -> String {
    tokens.to_token_stream().to_string()
}

fn segments(path: &syn::Path) -> Vec<String> {
    path.segments.iter().map(|s| s.ident.to_string()).collect()
}

/// Whether `lib_rs` re-exports both `Error` and `Result` from its `error` module.
fn reexports_error(lib_rs: &syn::File) -> bool {
    lib_rs.items.iter().any(|item| {
        let syn::Item::Use(import) = item else {
            return false;
        };
        if !matches!(import.vis, syn::Visibility::Public(_)) {
            return false;
        }
        let mut names = Vec::new();
        use_leaves(&import.tree, &mut Vec::new(), &mut names);
        let from_error = |leaf: &str| {
            names.iter().any(|(path, name, _)| {
                name == leaf
                    && (path.as_slice() == ["error"] || path.as_slice() == ["crate", "error"])
            })
        };
        from_error("Error") && from_error("Result")
    })
}

/// Whether `lib_rs` declares `error` as bare `pub mod error`. That is the one public form a crate
/// root can declare: `pub(super)` and `pub(in <path>)` other than `crate` don't compile there, and
/// `pub(crate)`, `pub(self)` and `pub(in crate)` are crate-private, so only bare `pub` gives
/// `Error` a second public path.
fn declares_public_error_module(lib_rs: &syn::File) -> bool {
    lib_rs.items.iter().any(|item| {
        matches!(item, syn::Item::Mod(module)
            if module.ident == "error" && matches!(module.vis, syn::Visibility::Public(_)))
    })
}

/// A `use` leaf: the path before it, the name it imports (`*` for a glob) and the name it binds.
type Leaf = (Vec<String>, String, String);

fn use_leaves(tree: &syn::UseTree, prefix: &mut Vec<String>, out: &mut Vec<Leaf>) {
    match tree {
        syn::UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            use_leaves(&path.tree, prefix, out);
            prefix.pop();
        }
        syn::UseTree::Name(name) => {
            let name = name.ident.to_string();
            out.push((prefix.clone(), name.clone(), name));
        }
        syn::UseTree::Rename(rename) => out.push((
            prefix.clone(),
            rename.ident.to_string(),
            rename.rename.to_string(),
        )),
        syn::UseTree::Glob(_) => out.push((prefix.clone(), "*".into(), "*".into())),
        syn::UseTree::Group(group) => {
            for tree in &group.items {
                use_leaves(tree, prefix, out);
            }
        }
    }
}

/// Everything the structural check flags in the crate at `dir`, as `"<crate>: <what>"`.
fn structural_violations(dir: &Path) -> Vec<String> {
    let krate = Crate::at(dir);
    let name = &krate.name;
    let mut found = Vec::new();
    let error_rs = dir.join("src/error.rs");
    if !error_rs.is_file() {
        found.push(format!("{name}: has no src/error.rs"));
    }
    let lib_rs = dir.join("src/lib.rs");
    if lib_rs.is_file() {
        let parsed = syn::parse_file(&fs::read_to_string(&lib_rs).unwrap_or_default())
            .expect("lib.rs should parse");
        if !reexports_error(&parsed) {
            found.push(format!(
                "{name}: lib.rs lacks `pub use error::{{Error, Result}}`"
            ));
        }
        if declares_public_error_module(&parsed) {
            found.push(format!("{name}: lib.rs declares error as a public module"));
        }
    }
    let mut aliases_in_error_rs = 0;
    for (file, parsed) in crate_sources(dir) {
        let mut aliases = ResultAliases::default();
        aliases.visit_file(&parsed);
        let top_level = parsed.items.iter().filter_map(|item| match item {
            syn::Item::Type(alias) if alias.ident == "Result" => Some(alias),
            _ => None,
        });
        if file == error_rs {
            for alias in top_level {
                aliases_in_error_rs += 1;
                // Its doc comment is the crate's own; only the declaration has one shape.
                let mut bare = alias.clone();
                bare.attrs.clear();
                if spelt(&bare) != ALIAS {
                    found.push(format!(
                        "{name}: src/error.rs's alias is `{}`, not `{ALIAS}`",
                        spelt(&bare)
                    ));
                }
            }
            aliases.0 -= aliases_in_error_rs;
        }
        if aliases.0 > 0 {
            found.push(format!(
                "{name}: {} defines a Result alias outside src/error.rs",
                file.strip_prefix(dir).unwrap_or(&file).display()
            ));
        }
    }
    if error_rs.is_file() && aliases_in_error_rs != 1 {
        found.push(format!(
            "{name}: src/error.rs defines {aliases_in_error_rs} Result aliases, not one"
        ));
    }
    found
}

/// Counts the `type Result` aliases in a file, `#[cfg(test)]` items aside.
#[derive(Default)]
struct ResultAliases(usize);

impl<'ast> Visit<'ast> for ResultAliases {
    fn visit_item(&mut self, item: &'ast syn::Item) {
        if item_is_test(item) {
            return;
        }
        if let syn::Item::Type(alias) = item
            && alias.ident == "Result"
        {
            self.0 += 1;
        }
        visit::visit_item(self, item);
    }
}

fn item_attrs(item: &syn::Item) -> &[syn::Attribute] {
    match item {
        syn::Item::Const(item) => &item.attrs,
        syn::Item::Enum(item) => &item.attrs,
        syn::Item::Fn(item) => &item.attrs,
        syn::Item::Impl(item) => &item.attrs,
        syn::Item::Mod(item) => &item.attrs,
        syn::Item::Static(item) => &item.attrs,
        syn::Item::Struct(item) => &item.attrs,
        syn::Item::Trait(item) => &item.attrs,
        syn::Item::Type(item) => &item.attrs,
        syn::Item::Use(item) => &item.attrs,
        _ => &[],
    }
}

fn item_is_test(item: &syn::Item) -> bool {
    is_cfg_test(item_attrs(item))
}

/// Walks one file for the usage rules, collecting what breaks them.
struct Usage<'a> {
    own_names: &'a BTreeSet<String>,
    other_crates: &'a BTreeSet<String>,
    /// The generic parameters in scope: the enclosing impls' and traits' and the function's own.
    generics: Vec<BTreeSet<String>>,
    /// Whether the walk is inside a return type or a non-crate alias, where [`check_result`]
    /// already reports a qualified `Result`.
    in_signature: bool,
    found: Vec<String>,
}

impl<'a> Usage<'a> {
    fn new(own_names: &'a BTreeSet<String>, other_crates: &'a BTreeSet<String>) -> Self {
        Self {
            own_names,
            other_crates,
            generics: Vec::new(),
            in_signature: false,
            found: Vec::new(),
        }
    }

    fn is_generic(&self, name: &str) -> bool {
        self.generics.iter().any(|scope| scope.contains(name))
    }

    /// Checks every `Result` spelt in `ty`, the return type or aliased type of `what`.
    fn check_signature(&mut self, what: &str, ty: &syn::Type) {
        let mut results = ResultPaths::default();
        results.visit_type(ty);
        for path in results.0 {
            if let Some(why) = self.broken_result(&path) {
                self.found
                    .push(format!("{what} returns `{}`: {why}", spelt(ty)));
            }
        }
        self.in_signature = true;
        self.visit_type(ty);
        self.in_signature = false;
    }

    /// Why the `Result` spelt `path` breaks the return-type rule, if it does.
    fn broken_result(&self, path: &syn::TypePath) -> Option<&'static str> {
        let names = segments(&path.path);
        if names.len() > 1 {
            let qualifier = &names[names.len() - 2];
            let own = names.len() == 2 && self.own_names.contains(&names[0]);
            if qualifier != "fmt" && !own {
                return Some("a qualified Result other than crate::Result");
            }
        }
        let last = path.path.segments.last()?;
        let syn::PathArguments::AngleBracketed(arguments) = &last.arguments else {
            return None;
        };
        let types: Vec<&syn::Type> = arguments
            .args
            .iter()
            .filter_map(|argument| match argument {
                syn::GenericArgument::Type(ty) => Some(ty),
                _ => None,
            })
            .collect();
        let [_, error] = types.as_slice() else {
            return None;
        };
        if self.error_is_allowed(error) {
            None
        } else {
            Some("its error is not on VALUE_ERRORS")
        }
    }

    fn error_is_allowed(&self, error: &syn::Type) -> bool {
        let syn::Type::Path(error) = error else {
            return false;
        };
        if let Some(qself) = &error.qself {
            return matches!(&*qself.ty, syn::Type::Path(ty)
                if ty.path.segments.first().is_some_and(|first| self.is_generic(&first.ident.to_string())));
        }
        let names = segments(&error.path);
        let first_is_generic = names.first().is_some_and(|first| self.is_generic(first));
        let last_is_value_error = names
            .last()
            .is_some_and(|last| VALUE_ERRORS.contains(&last.as_str()));
        first_is_generic || last_is_value_error
    }

    fn check_fn(&mut self, name: &syn::Ident, sig: &syn::Signature) {
        self.generics.push(generic_names(&sig.generics));
        if let syn::ReturnType::Type(_, ty) = &sig.output {
            self.check_signature(&format!("fn {name}"), ty);
        }
        for input in &sig.inputs {
            self.visit_fn_arg(input);
        }
    }

    fn check_use(&mut self, import: &syn::ItemUse) {
        let mut leaves = Vec::new();
        use_leaves(&import.tree, &mut Vec::new(), &mut leaves);
        for (path, name, bound) in leaves {
            let from_elsewhere = path
                .first()
                .is_some_and(|root| self.other_crates.contains(root));
            if !from_elsewhere {
                continue;
            }
            let imports_result = name == "Result" && path.last().is_none_or(|m| m != "fmt");
            let globs_result = name == "*"
                && (path.len() == 1 || path.last().is_some_and(|m| m == "error" || m == "result"));
            if imports_result || globs_result {
                self.found.push(format!(
                    "imports another crate's Result: `use {}::{name}`{}",
                    path.join("::"),
                    if bound == name {
                        String::new()
                    } else {
                        format!(" as {bound}")
                    }
                ));
            }
        }
    }
}

fn generic_names(generics: &syn::Generics) -> BTreeSet<String> {
    generics
        .type_params()
        .map(|param| param.ident.to_string())
        .collect()
}

/// Collects every path in a type whose last segment is `Result`.
#[derive(Default)]
struct ResultPaths(Vec<syn::TypePath>);

impl<'ast> Visit<'ast> for ResultPaths {
    fn visit_type_path(&mut self, path: &'ast syn::TypePath) {
        if path
            .path
            .segments
            .last()
            .is_some_and(|last| last.ident == "Result")
        {
            self.0.push(path.clone());
        }
        visit::visit_type_path(self, path);
    }
}

fn is_std_result(path: &syn::Path) -> bool {
    let names = segments(path);
    matches!(names.as_slice(), [root, module, last]
        if (root == "std" || root == "core") && module == "result" && last == "Result")
        || matches!(names.as_slice(), [module, last] if module == "result" && last == "Result")
}

impl<'ast> Visit<'ast> for Usage<'_> {
    fn visit_item(&mut self, item: &'ast syn::Item) {
        if !item_is_test(item) {
            visit::visit_item(self, item);
        }
    }

    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        self.check_fn(&item.sig.ident, &item.sig);
        self.visit_block(&item.block);
        self.generics.pop();
    }

    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        if is_cfg_test(&item.attrs) {
            return;
        }
        self.check_fn(&item.sig.ident, &item.sig);
        self.visit_block(&item.block);
        self.generics.pop();
    }

    fn visit_trait_item_fn(&mut self, item: &'ast syn::TraitItemFn) {
        if is_cfg_test(&item.attrs) {
            return;
        }
        self.check_fn(&item.sig.ident, &item.sig);
        if let Some(block) = &item.default {
            self.visit_block(block);
        }
        self.generics.pop();
    }

    fn visit_item_impl(&mut self, item: &'ast syn::ItemImpl) {
        self.generics.push(generic_names(&item.generics));
        if item.trait_.is_some() {
            // The trait fixes these signatures, so only the bodies are the crate's own.
            for member in &item.items {
                if let syn::ImplItem::Fn(method) = member
                    && !is_cfg_test(&method.attrs)
                {
                    self.visit_block(&method.block);
                }
            }
        } else {
            visit::visit_item_impl(self, item);
        }
        self.generics.pop();
    }

    fn visit_item_trait(&mut self, item: &'ast syn::ItemTrait) {
        self.generics.push(generic_names(&item.generics));
        visit::visit_item_trait(self, item);
        self.generics.pop();
    }

    fn visit_item_type(&mut self, item: &'ast syn::ItemType) {
        self.generics.push(generic_names(&item.generics));
        self.check_signature(&format!("type {}", item.ident), &item.ty);
        self.generics.pop();
    }

    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        self.check_use(item);
    }

    fn visit_path(&mut self, path: &'ast syn::Path) {
        if !self.in_signature && is_std_result(path) {
            self.found
                .push(format!("spells `{}` outside the Result alias", spelt(path)));
        }
        visit::visit_path(self, path);
    }
}

/// What the usage rules flag in one file's source, for a crate whose own names are `own_names`.
fn usage_violations_in(parsed: &syn::File, krate: &Crate, is_error_rs: bool) -> Vec<String> {
    let mut usage = Usage::new(&krate.own_names, &krate.other_crates);
    for item in &parsed.items {
        // The alias definition is the one place `std::result::Result` belongs; the structural
        // check holds it to its shape.
        if is_error_rs && matches!(item, syn::Item::Type(alias) if alias.ident == "Result") {
            continue;
        }
        usage.visit_item(item);
    }
    usage.found
}

/// Everything the usage rules flag in the crate at `dir`, as `"<crate>: <file>: <what>"`.
fn usage_violations(dir: &Path) -> Vec<String> {
    let krate = Crate::at(dir);
    let error_rs = krate.dir.join("src/error.rs");
    let mut found = Vec::new();
    for (file, parsed) in crate_sources(dir) {
        let relative = file
            .strip_prefix(dir)
            .unwrap_or(&file)
            .display()
            .to_string();
        for what in usage_violations_in(&parsed, &krate, file == error_rs) {
            found.push(format!("{}: {relative}: {what}", krate.name));
        }
    }
    found
}

#[test]
fn every_crate_follows_the_error_rs_convention() {
    let crates = workspace_crates();
    assert!(
        crates.len() > 30,
        "crates/libs/* and crates/bins/* should list the crates"
    );
    let found: Vec<String> = crates
        .iter()
        .flat_map(|dir| structural_violations(dir))
        .collect();
    assert!(
        found.is_empty(),
        "error.rs convention broken: every crate has a private src/error.rs holding `{ALIAS}`, \
         re-exported from lib.rs as `pub use error::{{Error, Result}}`. See /rust-style.\n  {}",
        found.join("\n  ")
    );
}

#[test]
fn every_crate_not_pending_returns_its_own_result() {
    let found: Vec<String> = workspace_crates()
        .iter()
        .filter(|dir| !PENDING.contains(&Crate::at(dir).name.as_str()))
        .flat_map(|dir| usage_violations(dir))
        .collect();
    assert!(
        found.is_empty(),
        "ADR-0060 broken: every fallible function returns its crate's `Result<T>`, or \
         `Result<T, E>` for a value error on VALUE_ERRORS, and no `use` imports another crate's \
         `Result`. See /rust-style.\n  {}",
        found.join("\n  ")
    );
}

/// The entries of `pending` that name no crate or a crate the usage rules already pass.
fn stale_entries<'a>(pending: &[&'a str]) -> Vec<&'a str> {
    let crates = workspace_crates();
    pending
        .iter()
        .copied()
        .filter(|pending| {
            crates
                .iter()
                .find(|dir| Crate::at(dir).name == *pending)
                .is_none_or(|dir| usage_violations(dir).is_empty())
        })
        .collect()
}

#[test]
fn pending_lists_only_crates_that_still_break_the_rules() {
    let stale = stale_entries(PENDING);
    assert!(
        stale.is_empty(),
        "PENDING holds crates that pass the usage rules or don't exist; delete them: {stale:?}"
    );
}

#[test]
fn a_passing_or_missing_crate_on_pending_is_stale() {
    assert_eq!(
        stale_entries(&["lib-viper", "lib-nowhere", "bin-desktop"]),
        vec!["lib-viper", "lib-nowhere"]
    );
}

/// A bin crate `bin_x` that depends on `lib_core` and `lib_shell`.
fn fixture_crate() -> Crate {
    let names = |list: &[&str]| list.iter().copied().map(String::from).collect();
    Crate {
        dir: PathBuf::new(),
        name: "bin-x".into(),
        own_names: names(&["crate", "self", "super", "Self", "bin_x"]),
        other_crates: names(&["std", "core", "lib_core", "lib_shell"]),
    }
}

fn flags(source: &str) -> Vec<String> {
    let parsed = syn::parse_file(source).expect("fixture parses");
    usage_violations_in(&parsed, &fixture_crate(), false)
}

#[test]
fn a_two_argument_result_fails_unless_its_error_is_a_value_error() {
    assert_eq!(flags("fn f() -> Result<u8, Error> { todo!() }").len(), 1);
    assert_eq!(flags("fn f() -> Result<u8, String> { todo!() }").len(), 1);
    assert_eq!(flags("fn f() -> Result<u8, ()> { todo!() }").len(), 1);
    assert_eq!(
        flags("fn f() -> Result<u8, crate::Error> { todo!() }").len(),
        1
    );
    assert_eq!(
        flags("struct S; impl S { fn f() -> Option<Result<u8, X>> { todo!() } }").len(),
        1
    );
    assert_eq!(
        flags("fn f() -> impl Future<Output = Result<u8, X>> { async { todo!() } }").len(),
        1
    );
    assert_eq!(flags("trait T { fn f(&self) -> Result<u8, X>; }").len(), 1);
    assert!(flags("fn f() -> Result<u8> { todo!() }").is_empty());
    assert!(flags("fn f() -> Result<u8, MoneyError> { todo!() }").is_empty());
    assert!(flags("fn f() -> Result<u8, lib_core::HexColorError> { todo!() }").is_empty());
}

#[test]
fn a_qualified_result_fails_unless_it_is_the_crates_own() {
    assert_eq!(flags("fn f() -> io::Result<()> { todo!() }").len(), 1);
    assert_eq!(
        flags("fn f() -> lib_shell::Result<()> { todo!() }").len(),
        1
    );
    assert_eq!(
        flags("fn f() -> std::result::Result<u8, MoneyError> { todo!() }").len(),
        1
    );
    assert!(flags("fn f() -> crate::Result<()> { todo!() }").is_empty());
    assert!(flags("fn f() -> bin_x::Result<()> { todo!() }").is_empty());
}

#[test]
fn std_result_belongs_only_in_the_alias() {
    assert_eq!(
        flags("fn f() { let r: std::result::Result<u8, u8> = Ok(1); }").len(),
        1
    );
    assert_eq!(
        flags("type Outcome = std::result::Result<u8, u8>;").len(),
        1
    );
    let error_rs = syn::parse_file(&format!("pub enum Error {{}}\n{ALIAS}")).expect("parses");
    assert!(usage_violations_in(&error_rs, &fixture_crate(), true).is_empty());
}

#[test]
fn a_use_of_another_crates_result_fails_but_its_error_passes() {
    assert_eq!(flags("use lib_shell::Result;").len(), 1);
    assert_eq!(flags("use std::io::Result as IoResult;").len(), 1);
    assert_eq!(flags("use lib_core::{Error, Result};").len(), 1);
    assert_eq!(flags("use lib_core::*;").len(), 1);
    assert_eq!(flags("use lib_core::error::*;").len(), 1);
    assert_eq!(flags("fn f() { use lib_core::Result; }").len(), 1);
    assert!(flags("use lib_shell::Error;").is_empty());
    assert!(flags("use crate::Result;").is_empty());
    assert!(flags("use super::{Error, Result};").is_empty());
    assert!(flags("use std::fmt::Result;").is_empty());
    assert!(flags("use lib_core::prelude::*;").is_empty());
    assert!(flags("mod error;\nuse error::Result;\nuse entity::*;").is_empty());
    let manifest = "[dependencies]\nlib_core = { path = \"x\" }\nthiserror.workspace = true\n\n[dev-dependencies]\nfake = \"1\"\n\n[target.'cfg(unix)'.dependencies]\nnix-ext = \"1\"\n";
    assert_eq!(
        manifest_dependencies(manifest),
        vec!["lib_core", "thiserror", "nix_ext"]
    );
}

#[test]
fn a_bins_main_may_use_its_own_lib_targets_result() {
    assert!(flags("use bin_x::Result;\nfn main() -> Result<()> { Ok(()) }").is_empty());
    let manifest = "[package]\nname = \"bin_demo\"\n\n[[bin]]\nname = \"demo\"\n\n[lib]\nname = \"bin_demo_lib\"\n";
    assert_eq!(manifest_names(manifest), vec!["bin_demo", "bin_demo_lib"]);
}

#[test]
fn signatures_fixed_from_outside_are_exempt() {
    assert!(
        flags("impl std::str::FromStr for S { type Err = X; fn from_str(s: &str) -> Result<Self, Self::Err> { todo!() } }")
            .is_empty()
    );
    assert!(
        flags("fn de<'de, D: Deserializer<'de>>(d: D) -> Result<u8, D::Error> { todo!() }")
            .is_empty()
    );
    assert!(flags("fn f<R, E>(g: impl Fn() -> Result<R, E>) -> Result<R, E> { g() }").is_empty());
    assert!(
        flags("struct S<E>(E); impl<E> S<E> { fn f(&self) -> Result<(), E> { todo!() } }")
            .is_empty()
    );
    assert!(
        flags("fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { todo!() }").is_empty()
    );
    assert!(
        flags("fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() }").is_empty()
    );
    assert_eq!(
        flags("impl T for S { fn f(&self) { fn inner() -> Result<u8, X> { todo!() } } }").len(),
        1,
        "a function nested in a trait method's body is still the crate's own"
    );
}

#[test]
fn test_code_is_skipped() {
    assert!(flags("#[cfg(test)] mod tests { fn f() -> Result<u8, X> { todo!() } }").is_empty());
    assert!(flags("#[cfg(test)] fn f() -> io::Result<()> { todo!() }").is_empty());
    assert!(flags("#[cfg(all(test, feature = \"x\"))] use lib_core::Result;").is_empty());
    assert_eq!(
        flags(
            "#[cfg(any(test, feature = \"contract-tests\"))] fn f() -> io::Result<()> { todo!() }"
        )
        .len(),
        1
    );
}

/// A throwaway crate under the temp directory, removed on drop.
struct TempCrate(PathBuf);

impl TempCrate {
    fn new(tag: &str, files: &[(&str, &str)]) -> Self {
        let dir =
            std::env::temp_dir().join(format!("error_convention_{tag}_{}", std::process::id()));
        fs::create_dir_all(dir.join("src")).expect("temp crate dir");
        fs::write(dir.join("Cargo.toml"), "[package]\nname = \"demo\"\n").expect("manifest");
        for (path, source) in files {
            let path = dir.join(path);
            fs::create_dir_all(path.parent().expect("a file sits in a dir")).expect("module dir");
            fs::write(path, source).expect("source");
        }
        Self(dir)
    }
}

impl Drop for TempCrate {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).ok();
    }
}

#[test]
fn the_structural_check_flags_a_missing_error_file_and_re_export() {
    let krate = TempCrate::new("missing", &[("src/lib.rs", "pub fn f() {}\n")]);
    let found = structural_violations(&krate.0);
    assert_eq!(found.len(), 2, "{found:?}");
}

#[test]
fn the_structural_check_flags_a_public_error_module() {
    let krate = TempCrate::new(
        "public",
        &[
            (
                "src/lib.rs",
                "pub mod error;\npub use error::{Error, Result};\n",
            ),
            ("src/error.rs", &format!("pub enum Error {{}}\n{ALIAS}\n")),
        ],
    );
    let found = structural_violations(&krate.0);
    assert_eq!(
        found,
        vec![
            "error_convention_public: lib.rs declares error as a public module"
                .replace("error_convention_public", &Crate::at(&krate.0).name)
        ]
    );
}

#[test]
fn the_structural_check_holds_every_alias_to_one_shape() {
    let krate = TempCrate::new(
        "shape",
        &[
            (
                "src/lib.rs",
                "mod error;\nmod other;\npub use crate::error::{Error, Result};\n",
            ),
            (
                "src/error.rs",
                "pub enum Error {}\npub type Result<T> = std::result::Result<T, Error>;\n",
            ),
            (
                "src/other.rs",
                "pub type Result<T> = crate::Result<T>;\n#[cfg(test)] mod tests { type Result = (); }\n",
            ),
        ],
    );
    let found = structural_violations(&krate.0);
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(found[0].contains("src/error.rs's alias is"), "{found:?}");
    assert!(
        found[1].contains("src/other.rs defines a Result alias"),
        "{found:?}"
    );
}

#[test]
fn the_structural_check_passes_the_convention_and_reads_a_bin_without_lib_rs() {
    let lib = TempCrate::new(
        "lib_ok",
        &[
            (
                "src/lib.rs",
                "mod error;\npub use error::{\n    Error,\n    Result,\n};\n",
            ),
            ("src/error.rs", &format!("pub enum Error {{}}\n{ALIAS}\n")),
        ],
    );
    assert!(structural_violations(&lib.0).is_empty());
    let bin = TempCrate::new(
        "bin_ok",
        &[
            (
                "src/main.rs",
                "mod error;\nuse error::Result;\nfn main() -> Result<()> { Ok(()) }\n",
            ),
            ("src/error.rs", &format!("pub enum Error {{}}\n{ALIAS}\n")),
        ],
    );
    assert!(structural_violations(&bin.0).is_empty());
    assert!(usage_violations(&bin.0).is_empty());
}

#[test]
fn the_module_tree_skips_test_and_generated_modules() {
    let krate = TempCrate::new(
        "tree",
        &[
            (
                "src/lib.rs",
                "mod a;\n#[cfg(test)] mod tests;\nmod generated;\n",
            ),
            ("src/a.rs", "mod b;\n"),
            ("src/a/b.rs", "pub fn f() -> io::Result<()> { todo!() }\n"),
            ("src/tests.rs", "pub fn f() -> io::Result<()> { todo!() }\n"),
            (
                "src/generated/mod.rs",
                "pub fn f() -> io::Result<()> { todo!() }\n",
            ),
        ],
    );
    let found = usage_violations(&krate.0);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("src/a/b.rs"), "{found:?}");
}
