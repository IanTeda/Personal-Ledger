//! The Institution Entity, its add rule and the stub seed.

use lib_core::InstitutionId;

/// One Institution: a row of the **Institutions** section's table
/// (`docs/ux/desktop-mockups/16-settings/README.md`'s "2a resting state" markup: INSTITUTION /
/// PRODUCTS columns).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Institution {
    pub id: InstitutionId,
    pub name: String,
    /// The PRODUCTS column, what the Institution offers (`GLOSSARY.md`'s Products), e.g.
    /// `"savings \u{b7} offset"` -- the mockup joins several with the same `\u{b7}` separator the
    /// scope notes use, as one free-form label (not a `Vec` of chips -- that multi-select shape
    /// belongs to the Add institution dialog's own input, not this read-only table cell). The Add
    /// institution form joins the same way when it builds a draft. Descriptive only: it never
    /// limits which Accounts may name the Institution.
    pub products: String,
}

/// What the Add institution dialog asks for: the name as typed and the joined Products label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstitutionDraft {
    pub name: String,
    pub products: String,
}

/// Why an Institution write was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InstitutionError {
    #[error("an institution needs a name")]
    NameRequired,
    /// Carries the name as the Institution holding it spells it.
    #[error("an institution named {0} already exists")]
    DuplicateName(String),
}

/// The mockup's own six seeded rows, in its own order, each with its fixed id (the mockup's static
/// scope note claims "7 institutions", but only six rows are actually drawn -- treated as a
/// mockup-authoring slip, not a seventh row to invent; the scope note counts the rows, so it
/// self-corrects). Ids match `lib-accounts`' stub, whose Accounts name these Institutions.
pub fn default_institutions() -> Vec<Institution> {
    [
        (
            InstitutionId::SEED_ANZ,
            "ANZ Banking Group",
            "savings \u{b7} offset",
        ),
        (InstitutionId::SEED_AMEX, "American Express", "credit card"),
        (
            InstitutionId::SEED_VANGUARD,
            "Vanguard Investments",
            "investment",
        ),
        (InstitutionId::SEED_WESTPAC, "Westpac Banking", "savings"),
        (
            InstitutionId::SEED_CRYPTO_EXCHANGE,
            "Cryptocurrency Exchange",
            "crypto",
        ),
        (
            InstitutionId::SEED_SUPER_FUND,
            "Superannuation Fund",
            "retirement",
        ),
    ]
    .into_iter()
    .map(|(id, name, products)| Institution {
        id,
        name: name.to_string(),
        products: products.to_string(),
    })
    .collect()
}

/// Institution `id`, if it's in `institutions`.
pub fn get(institutions: &[Institution], id: InstitutionId) -> Option<&Institution> {
    institutions.iter().find(|institution| institution.id == id)
}

/// The Institution whose name matches `name` once trimmed and case folded, as the add rule
/// compares them.
pub fn find_by_name<'a>(institutions: &'a [Institution], name: &str) -> Option<&'a Institution> {
    let name = name.trim().to_lowercase();
    institutions
        .iter()
        .find(|institution| institution.name.to_lowercase() == name)
}

/// Appends the Institution `draft` describes, with a fresh id, and returns it. The name is stored
/// trimmed and must be set and case-insensitively unique, so persistence can keep `name` unique;
/// `products` is the joined label the dialog built, stored as given. A refusal leaves
/// `institutions` untouched.
pub fn insert_institution(
    institutions: &mut Vec<Institution>,
    draft: &InstitutionDraft,
) -> Result<Institution, InstitutionError> {
    let name = draft.name.trim();
    if name.is_empty() {
        return Err(InstitutionError::NameRequired);
    }
    if let Some(holder) = find_by_name(institutions, name) {
        return Err(InstitutionError::DuplicateName(holder.name.clone()));
    }
    let institution = Institution {
        id: InstitutionId::new(),
        name: name.to_string(),
        products: draft.products.clone(),
    };
    institutions.push(institution.clone());
    Ok(institution)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft(name: &str) -> InstitutionDraft {
        InstitutionDraft {
            name: name.into(),
            products: "savings \u{b7} offset".into(),
        }
    }

    #[test]
    fn default_institutions_matches_the_mockups_own_six_seeded_rows() {
        let names: Vec<_> = default_institutions()
            .into_iter()
            .map(|institution| institution.name)
            .collect();
        assert_eq!(
            names,
            vec![
                "ANZ Banking Group",
                "American Express",
                "Vanguard Investments",
                "Westpac Banking",
                "Cryptocurrency Exchange",
                "Superannuation Fund",
            ]
        );
    }

    #[test]
    fn each_seeded_institution_has_its_own_fixed_id() {
        let seeded = default_institutions();
        assert_eq!(seeded, default_institutions(), "the seed is deterministic");
        let mut ids: Vec<_> = seeded.iter().map(|institution| institution.id).collect();
        ids.dedup();
        assert_eq!(ids.len(), seeded.len());
        assert_eq!(
            get(&seeded, InstitutionId::SEED_WESTPAC).map(|i| i.name.as_str()),
            Some("Westpac Banking")
        );
    }

    #[test]
    fn an_added_institution_is_appended_trimmed_with_its_joined_products() {
        let mut institutions = default_institutions();
        let before = institutions.len();
        let added =
            insert_institution(&mut institutions, &draft("  Bank X ")).expect("a free name");
        assert_eq!(added.name, "Bank X");
        assert_eq!(added.products, "savings \u{b7} offset");
        assert_eq!(institutions.len(), before + 1);
        assert_eq!(institutions.last(), Some(&added));
    }

    #[test]
    fn a_blank_or_taken_name_is_refused_and_nothing_is_added() {
        let mut institutions = default_institutions();
        let before = institutions.clone();
        assert_eq!(
            insert_institution(&mut institutions, &draft("   ")),
            Err(InstitutionError::NameRequired)
        );
        assert_eq!(
            insert_institution(&mut institutions, &draft(" american EXPRESS ")),
            Err(InstitutionError::DuplicateName("American Express".into()))
        );
        assert_eq!(institutions, before);
    }
}
