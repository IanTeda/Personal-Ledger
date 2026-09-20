//! Rich Messages: styled and clickable spans without any UI types.
//!
//! Two mechanisms, both returning [`Segment`]s the caller maps to its own styling:
//!
//! - **Tokens.** Every text argument to a rich accessor (a key name, a noun) is sent through
//!   Fluent as a private-use sentinel, `U+E000` + index + `U+E001`, so translators see a plain
//!   `{ $key }`. After formatting the text is split on the sentinels and each becomes a token
//!   segment carrying the original value and its index. Because a value never enters the
//!   formatted text, it cannot forge a tag or a sentinel: `<open>` inside an argument stays
//!   literal.
//! - **Tags.** A translated styled span is written `<open>open</open>` in the Catalogue and
//!   parsed into segments with a tag. Tags nest, the innermost wins, and any `<` that is not a
//!   well-formed, closed lowercase tag is literal text.

/// Opens a token sentinel.
const SENTINEL_START: char = '\u{e000}';

/// Closes a token sentinel.
const SENTINEL_END: char = '\u{e001}';

/// One run of a rich Message, with no styling attached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    /// The text to show. For a token, the argument's value.
    pub text: String,

    /// The enclosing `<tag>` name, if any (the innermost when they nest).
    pub tag: Option<String>,

    /// The index of the text argument this segment carries, in the order the arguments were
    /// passed to the accessor (order of first appearance in the Message for generated accessors). `None`
    /// for Message text.
    pub token: Option<usize>,
}

impl Segment {
    fn text(text: &str, tag: Option<&str>) -> Segment {
        Segment {
            text: text.to_string(),
            tag: tag.map(str::to_string),
            token: None,
        }
    }
}

/// The sentinel that stands for the token at `index` while a Message is formatted.
pub(crate) fn sentinel(index: usize) -> String {
    format!("{SENTINEL_START}{index}{SENTINEL_END}")
}

/// Splits formatted text into segments: tags first, then sentinels inside each tagged run,
/// which are replaced by the matching entry of `tokens`.
pub(crate) fn split(formatted: &str, tokens: &[&str]) -> Vec<Segment> {
    let mut tagged = Vec::new();
    parse_tags(formatted, None, &mut tagged);

    let mut segments: Vec<Segment> = Vec::new();
    for (text, tag) in tagged {
        split_sentinels(&text, tag.as_deref(), tokens, &mut segments);
    }
    segments
}

fn push_text(segments: &mut Vec<Segment>, text: &str, tag: Option<&str>) {
    if text.is_empty() {
        return;
    }
    if let Some(last) = segments.last_mut()
        && last.token.is_none()
        && last.tag.as_deref() == tag
    {
        last.text.push_str(text);
        return;
    }
    segments.push(Segment::text(text, tag));
}

fn split_sentinels(text: &str, tag: Option<&str>, tokens: &[&str], out: &mut Vec<Segment>) {
    let mut rest = text;
    while let Some(start) = rest.find(SENTINEL_START) {
        push_text(out, &rest[..start], tag);
        let after = &rest[start + SENTINEL_START.len_utf8()..];
        let Some(end) = after.find(SENTINEL_END) else {
            // An unterminated sentinel is dropped rather than shown.
            rest = after;
            continue;
        };
        let value = after[..end]
            .parse::<usize>()
            .ok()
            .and_then(|index| tokens.get(index).map(|value| (index, *value)));
        if let Some((index, value)) = value {
            out.push(Segment {
                text: value.to_string(),
                tag: tag.map(str::to_string),
                token: Some(index),
            });
        }
        rest = &after[end + SENTINEL_END.len_utf8()..];
    }
    push_text(out, rest, tag);
}

/// Appends `(text, tag)` runs, recursing into each well-formed `<name>...</name>`.
fn parse_tags(text: &str, tag: Option<&str>, out: &mut Vec<(String, Option<String>)>) {
    let mut rest = text;
    let mut literal = String::new();
    while let Some(at) = rest.find('<') {
        literal.push_str(&rest[..at]);
        let candidate = &rest[at..];
        match take_element(candidate) {
            Some((name, inner, remaining)) => {
                if !literal.is_empty() {
                    out.push((std::mem::take(&mut literal), tag.map(str::to_string)));
                }
                parse_tags(inner, Some(name), out);
                rest = remaining;
            }
            None => {
                literal.push('<');
                rest = &candidate[1..];
            }
        }
    }
    literal.push_str(rest);
    if !literal.is_empty() {
        out.push((literal, tag.map(str::to_string)));
    }
}

/// If `text` starts with `<name>`, returns the name, the text up to its matching `</name>`
/// (nesting the same name is balanced), and what follows.
fn take_element(text: &str) -> Option<(&str, &str, &str)> {
    let body = text.strip_prefix('<')?;
    let name_len = body
        .find(|ch: char| !(ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-'))
        .unwrap_or(body.len());
    let name = &body[..name_len];
    if !name.starts_with(|ch: char| ch.is_ascii_lowercase()) || !body[name_len..].starts_with('>') {
        return None;
    }
    let content = &body[name_len + 1..];
    let open = format!("<{name}>");
    let close = format!("</{name}>");

    let mut depth = 1;
    let mut cursor = 0;
    while cursor < content.len() {
        let next_open = content[cursor..].find(&open).map(|i| i + cursor);
        let next_close = content[cursor..].find(&close).map(|i| i + cursor)?;
        match next_open {
            Some(opened) if opened < next_close => {
                depth += 1;
                cursor = opened + open.len();
            }
            _ => {
                depth -= 1;
                if depth == 0 {
                    return Some((
                        name,
                        &content[..next_close],
                        &content[next_close + close.len()..],
                    ));
                }
                cursor = next_close + close.len();
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(text: &str, tag: Option<&str>, token: Option<usize>) -> Segment {
        Segment {
            text: text.to_string(),
            tag: tag.map(str::to_string),
            token,
        }
    }

    #[test]
    fn plain_text_is_one_segment() {
        assert_eq!(
            split("Nothing here", &[]),
            vec![seg("Nothing here", None, None)]
        );
        assert!(split("", &[]).is_empty());
    }

    #[test]
    fn sentinels_round_trip_to_tokens() {
        let text = format!("Press {} to {}", sentinel(1), sentinel(0));
        assert_eq!(
            split(&text, &["continue", "Enter"]),
            vec![
                seg("Press ", None, None),
                seg("Enter", None, Some(1)),
                seg(" to ", None, None),
                seg("continue", None, Some(0)),
            ]
        );
    }

    #[test]
    fn tags_become_tagged_segments() {
        assert_eq!(
            split("Press <open>open</open> to see", &[]),
            vec![
                seg("Press ", None, None),
                seg("open", Some("open"), None),
                seg(" to see", None, None),
            ]
        );
    }

    #[test]
    fn tokens_inside_a_tag_inherit_it() {
        let text = format!("Go <link>to {}</link>.", sentinel(0));
        assert_eq!(
            split(&text, &["Accounts"]),
            vec![
                seg("Go ", None, None),
                seg("to ", Some("link"), None),
                seg("Accounts", Some("link"), Some(0)),
                seg(".", None, None),
            ]
        );
    }

    #[test]
    fn tags_nest_and_the_innermost_wins() {
        assert_eq!(
            split("<a>x <b>y</b> z</a>", &[]),
            vec![
                seg("x ", Some("a"), None),
                seg("y", Some("b"), None),
                seg(" z", Some("a"), None),
            ]
        );
    }

    #[test]
    fn other_angle_brackets_are_literal() {
        for text in [
            "a < b and c > d",
            "<3",
            "<Upper>x</Upper>",
            "<open>never closed",
            "</open>",
        ] {
            assert_eq!(split(text, &[]), vec![seg(text, None, None)], "{text}");
        }
    }

    #[test]
    fn a_value_cannot_forge_a_tag_or_a_sentinel() {
        let text = format!("Press {}", sentinel(0));
        let forged = "<open>x</open>\u{e000}0\u{e001}";
        assert_eq!(
            split(&text, &[forged]),
            vec![seg("Press ", None, None), seg(forged, None, Some(0))]
        );
    }

    #[test]
    fn stray_sentinels_are_dropped() {
        assert_eq!(
            split("a\u{e000}9\u{e001}b\u{e000}", &["x"]),
            vec![seg("ab", None, None)]
        );
    }
}
