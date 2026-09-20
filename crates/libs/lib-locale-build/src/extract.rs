//! Reads the variables a Fluent pattern uses, so accessors can be typed.

use std::collections::BTreeMap;

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

/// Variable name (as written in the Catalogue) to its [`Kind`].
pub(crate) type Params = BTreeMap<String, Kind>;

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
                    out.insert(id.name.to_string(), Kind::Number);
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
            out.entry(id.name.to_string()).or_insert(Kind::Text);
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
