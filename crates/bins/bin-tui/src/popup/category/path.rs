//! `/`-separated category-path resolution, shared by every Category-domain popup with a
//! path-completing field: the Move popup's `new parent` and the New popup's `parent`
//! ("Categories: 5b move popup" built this first; "5c new popup" reuses it verbatim, per the
//! handoff's own "same widget as 5b").

use lib_core::RowID;

use crate::category::CategoryStore;

/// What a typed path currently resolves to.
pub enum Resolution {
    /// Every path segment matched an existing category — `RowID` is its id.
    Existing(RowID),
    /// Every segment but the last matched; the last (`name`) doesn't exist yet under the
    /// matched `parent` — `^n` can create it (the Move popup's own affordance; the New popup
    /// has no equivalent, its `parent` field only ever needs `Existing`).
    Creatable { parent: RowID, name: String },
    /// The path doesn't match anything, and doesn't leave exactly one creatable segment
    /// either (e.g. more than one missing level, or no matching root at all).
    Invalid,
}

/// Resolves `input` (a `/`-separated path, matched case-insensitively segment by segment
/// against root names then descendant names) against `store`. Only ever reports one missing
/// segment as creatable — a path missing more than one level in a row is `Invalid`, per the
/// handoff's own single-level "`^n` creates a missing parent inline" (not a whole missing
/// chain).
pub fn resolve(store: &dyn CategoryStore, input: &str) -> Resolution {
    let segments: Vec<&str> = input
        .split('/')
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .collect();
    let Some((first, rest)) = segments.split_first() else {
        return Resolution::Invalid;
    };

    let Some(mut current_id) = store
        .nodes()
        .iter()
        .find(|node| node.parent_id.is_none() && node.name.eq_ignore_ascii_case(first))
        .map(|node| node.id)
    else {
        return Resolution::Invalid;
    };

    for (index, segment) in rest.iter().enumerate() {
        let children = store.children(current_id);
        match children
            .iter()
            .find(|child| child.name.eq_ignore_ascii_case(segment))
        {
            Some(child) => current_id = child.id,
            None if index == rest.len() - 1 => {
                return Resolution::Creatable {
                    parent: current_id,
                    name: (*segment).to_string(),
                };
            }
            None => return Resolution::Invalid,
        }
    }

    Resolution::Existing(current_id)
}

/// Candidate category names sharing `input`'s last segment as a case-insensitive prefix,
/// sorted, deduplicated, capped to a handful — the `completion` row and what `Tab` accepts
/// the first of.
pub fn completions(store: &dyn CategoryStore, input: &str) -> Vec<String> {
    let last_segment = input.rsplit('/').next().unwrap_or("").to_lowercase();
    let mut names: Vec<String> = store
        .nodes()
        .iter()
        .filter(|node| node.name.to_lowercase().starts_with(&last_segment))
        .map(|node| node.name.clone())
        .collect();
    names.sort_by_key(|name| name.to_lowercase());
    names.dedup();
    names.truncate(4);
    names
}

/// `Tab`: completes `input`'s last path segment against the first matching category name
/// (alphabetical, case-insensitive) — a narrow-to-one-candidate step, not full argument
/// completion, mirroring the command popup's own `Tab` semantics. Returns the completed
/// string, or `input` unchanged if nothing matches.
pub fn tab_complete(store: &dyn CategoryStore, input: &str) -> String {
    let Some(candidate) = completions(store, input).into_iter().next() else {
        return input.to_string();
    };
    let mut segments: Vec<&str> = input.split('/').collect();
    segments.pop();
    segments.push(&candidate);
    segments.join("/")
}

/// `id`'s ancestor names, root to self, lowercase — the shared basis for both a spaced
/// display path and a compact `/`-joined input prefill.
pub fn ancestor_names(store: &dyn CategoryStore, id: RowID) -> Vec<String> {
    let mut names = Vec::new();
    let mut current = store.find(id);
    while let Some(node) = current {
        names.push(node.name.to_lowercase());
        current = node.parent_id.and_then(|parent_id| store.find(parent_id));
    }
    names.reverse();
    names
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::category::CategoryFixture;

    fn find_by_name(store: &CategoryFixture, name: &str) -> RowID {
        store
            .nodes()
            .iter()
            .find(|node| node.name == name)
            .unwrap_or_else(|| panic!("fixture should seed a category named {name}"))
            .id
    }

    #[test]
    fn resolves_an_existing_full_path() {
        let store = CategoryFixture::new();
        let transport = find_by_name(&store, "Transport");
        assert!(matches!(
            resolve(&store, "expenses/transport"),
            Resolution::Existing(id) if id == transport
        ));
    }

    #[test]
    fn resolves_a_creatable_missing_last_segment() {
        let store = CategoryFixture::new();
        let food = find_by_name(&store, "Food");
        match resolve(&store, "expenses/food/daily") {
            Resolution::Creatable { parent, name } => {
                assert_eq!(parent, food);
                assert_eq!(name, "daily");
            }
            _ => panic!("expected Creatable"),
        }
    }

    #[test]
    fn an_unresolvable_path_is_invalid() {
        let store = CategoryFixture::new();
        assert!(matches!(
            resolve(&store, "not/a/real/path/at/all"),
            Resolution::Invalid
        ));
    }

    #[test]
    fn tab_complete_completes_the_last_segment_to_the_first_alphabetical_match() {
        let store = CategoryFixture::new();
        assert_eq!(tab_complete(&store, "expenses/h"), "expenses/Health");
    }

    #[test]
    fn ancestor_names_walks_root_to_self_lowercase() {
        let store = CategoryFixture::new();
        let groceries = find_by_name(&store, "Groceries");
        assert_eq!(
            ancestor_names(&store, groceries),
            vec!["expenses", "food", "groceries"]
        );
    }
}
