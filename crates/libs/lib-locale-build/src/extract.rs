//! Reads the variables a Fluent pattern uses, so accessors can be typed.

use std::collections::BTreeSet;

use fluent_syntax::ast::{
    CallArguments, Expression, InlineExpression, Pattern, PatternElement, VariantKey,
};

/// The Rust type an accessor takes for a Fluent variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    /// A plural count, passed as `i64`.
    Number,

    /// Any other value, passed as `&str`.
    Text,
}

/// Variables in order of first appearance in the text (so an accessor's parameters read in the
/// same order as the Message), each with its [`Kind`].
#[derive(Debug, Clone, Default)]
pub(crate) struct Params(Vec<(String, Kind)>);

impl Params {
    pub(crate) fn new() -> Self {
        Params::default()
    }

    /// A count wins over text if the variable is used both ways; the first position is kept.
    fn insert(&mut self, name: &str, kind: Kind) {
        match self.0.iter_mut().find(|(existing, _)| existing == name) {
            Some((_, existing)) => {
                if kind == Kind::Number {
                    *existing = Kind::Number;
                }
            }
            None => self.0.push((name.to_string(), kind)),
        }
    }

    pub(crate) fn get(&self, name: &str) -> Option<Kind> {
        self.0
            .iter()
            .find(|(existing, _)| existing == name)
            .map(|(_, kind)| *kind)
    }

    pub(crate) fn contains_key(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    pub(crate) fn keys(&self) -> impl Iterator<Item = &String> {
        self.0.iter().map(|(name, _)| name)
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (&String, &Kind)> {
        self.0.iter().map(|(name, kind)| (name, kind))
    }
}

const PLURAL_CATEGORIES: [&str; 6] = ["zero", "one", "two", "few", "many", "other"];

pub(crate) fn pattern(source: &Pattern<&str>, out: &mut Params) {
    for element in &source.elements {
        if let PatternElement::Placeable { expression } = element {
            expression_params(expression, out);
        }
    }
}

fn expression_params(source: &Expression<&str>, out: &mut Params) {
    match source {
        Expression::Inline(inline) => inline_params(inline, out),
        Expression::Select { selector, variants } => {
            // A selector whose keys are numbers or CLDR plural categories takes a count.
            let numeric = variants.iter().any(|variant| match &variant.key {
                VariantKey::NumberLiteral { .. } => true,
                VariantKey::Identifier { name } => PLURAL_CATEGORIES.contains(name),
            });
            match selector {
                InlineExpression::VariableReference { id } if numeric => {
                    out.insert(id.name, Kind::Number);
                }
                other => inline_params(other, out),
            }
            for variant in variants {
                pattern(&variant.value, out);
            }
        }
    }
}

fn inline_params(source: &InlineExpression<&str>, out: &mut Params) {
    match source {
        InlineExpression::VariableReference { id } => {
            out.insert(id.name, Kind::Text);
        }
        InlineExpression::FunctionReference { arguments, .. } => call_arguments(arguments, out),
        InlineExpression::TermReference {
            arguments: Some(arguments),
            ..
        } => call_arguments(arguments, out),
        InlineExpression::Placeable { expression } => expression_params(expression, out),
        _ => {}
    }
}

fn call_arguments(source: &CallArguments<&str>, out: &mut Params) {
    for positional in &source.positional {
        inline_params(positional, out);
    }
    for named in &source.named {
        inline_params(&named.value, out);
    }
}

/// Plural branches that never show their count. In a selector on `$n`, each CLDR category
/// branch must place `{ $n }`, because a category holds many numbers in other Locales (`one` is
/// also 21 in Russian), so a typed digit or a bare noun reads wrong there. An exact-number
/// branch (`[0] none`) matches one number only and may say it in words.
pub(crate) fn uncounted_plural_branches(source: &Pattern<&str>) -> Vec<String> {
    let mut out = Vec::new();
    uncounted_in_pattern(source, &mut out);
    out
}

fn uncounted_in_pattern(source: &Pattern<&str>, out: &mut Vec<String>) {
    for element in &source.elements {
        if let PatternElement::Placeable { expression } = element {
            uncounted_in_expression(expression, out);
        }
    }
}

fn uncounted_in_expression(source: &Expression<&str>, out: &mut Vec<String>) {
    match source {
        Expression::Select { selector, variants } => {
            // A keyword selector (`[card]`, `*[other]`) may name its default `other`, so only a
            // selector keyed wholly by numbers and categories is taken as plural.
            let plural = variants.iter().all(|variant| match &variant.key {
                VariantKey::NumberLiteral { .. } => true,
                VariantKey::Identifier { name } => PLURAL_CATEGORIES.contains(name),
            });
            if let InlineExpression::VariableReference { id } = selector
                && plural
            {
                for variant in variants {
                    if let VariantKey::Identifier { name } = &variant.key
                        && PLURAL_CATEGORIES.contains(name)
                        && !pattern_uses(&variant.value, id.name)
                    {
                        out.push(format!("`[{name}]` does not show `${}`", id.name));
                    }
                }
            }
            for variant in variants {
                uncounted_in_pattern(&variant.value, out);
            }
        }
        Expression::Inline(InlineExpression::Placeable { expression }) => {
            uncounted_in_expression(expression, out);
        }
        Expression::Inline(_) => {}
    }
}

/// Whether the pattern uses `$name` anywhere, a nested selector on it included.
fn pattern_uses(source: &Pattern<&str>, name: &str) -> bool {
    let mut params = Params::new();
    pattern(source, &mut params);
    params.contains_key(name)
}

/// Flattens a pattern (and each select variant, separately) to text, with a placeholder where a
/// placeable sits, so tags can be balanced across placeables (`<b>{ $x }</b>`).
fn flatten(source: &Pattern<&str>, out: &mut Vec<String>) {
    let mut current = String::new();
    for element in &source.elements {
        match element {
            PatternElement::TextElement { value } => current.push_str(value),
            PatternElement::Placeable { expression } => {
                current.push('\u{fffc}');
                flatten_expression(expression, out);
            }
        }
    }
    out.push(current);
}

fn flatten_expression(source: &Expression<&str>, out: &mut Vec<String>) {
    match source {
        Expression::Select { variants, .. } => {
            for variant in variants {
                flatten(&variant.value, out);
            }
        }
        Expression::Inline(InlineExpression::Placeable { expression }) => {
            flatten_expression(expression, out);
        }
        Expression::Inline(_) => {}
    }
}

/// The `<tag>` names a pattern uses. A tag is `<name>` ... `</name>` with a lowercase name;
/// any other `<` is literal text. Returns a description of the problem if tags are unbalanced.
pub(crate) fn tags(source: &Pattern<&str>) -> Result<BTreeSet<String>, String> {
    let mut texts = Vec::new();
    flatten(source, &mut texts);

    let mut found = BTreeSet::new();
    for text in texts {
        let mut stack: Vec<&str> = Vec::new();
        let mut rest = text.as_str();
        while let Some(at) = rest.find('<') {
            rest = &rest[at + 1..];
            let closing = rest.starts_with('/');
            let body = if closing { &rest[1..] } else { rest };
            let name_len = body
                .find(|ch: char| !(ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-'))
                .unwrap_or(body.len());
            let name = &body[..name_len];
            let valid = name.starts_with(|ch: char| ch.is_ascii_lowercase())
                && body[name_len..].starts_with('>');
            if !valid {
                continue;
            }
            if closing {
                if stack.pop() != Some(name) {
                    return Err(format!("`</{name}>` closes a tag that is not open"));
                }
            } else {
                stack.push(name);
                found.insert(name.to_string());
            }
        }
        if let Some(open) = stack.last() {
            return Err(format!("`<{open}>` is never closed"));
        }
    }
    Ok(found)
}
