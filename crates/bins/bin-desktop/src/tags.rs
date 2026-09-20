//! Shared stub Tags, seeded from the TUI's Tag fixture (`crates/bins/bin-tui/src/tag/fixture.rs`
//! leads with "Japan Trip 2026") plus the neutral chip the Transactions mockup shows ("shared").
//! `gpui`-free; the future Tags map grows this module. Tag names are globally unique,
//! case-insensitively (glossary).

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    pub id: u32,
    pub name: String,
}

pub fn default_tags() -> Vec<Tag> {
    [
        "Japan Trip 2026",
        "shared",
        "tax deductible",
        "reimbursable",
    ]
    .iter()
    .zip(1u32..)
    .map(|(name, id)| Tag {
        id,
        name: name.to_string(),
    })
    .collect()
}

/// The id of the Tag named `name`, ignoring case, if any.
pub fn find_by_name(tags: &[Tag], name: &str) -> Option<u32> {
    tags.iter()
        .find(|tag| tag.name.eq_ignore_ascii_case(name))
        .map(|tag| tag.id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_unique_ignoring_case() {
        let tags = default_tags();
        let mut names: Vec<_> = tags.iter().map(|t| t.name.to_lowercase()).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), tags.len());
    }

    #[test]
    fn find_by_name_ignores_case() {
        let tags = default_tags();
        assert_eq!(find_by_name(&tags, "SHARED"), Some(2));
        assert_eq!(find_by_name(&tags, "missing"), None);
    }
}
