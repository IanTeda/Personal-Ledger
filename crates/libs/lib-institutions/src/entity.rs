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
    /// The Institution being edited is no longer on the Ledger.
    #[error("the institution is not on the Ledger")]
    NotFound,
}

/// The Products every Personal Institution is written with, as the Add Dialog's Cash chip joins it.
/// Stored as given, like any Products label.
const PERSONAL_PRODUCTS: &str = "cash";

/// The Personal Institution (`GLOSSARY.md`, ADR-0049): the person keeping the Ledger, whom a Cash
/// Account names. Every new Ledger is written with it under `name`, the "Personal" Message in the
/// Locale the Ledger was made in; from then on it is an ordinary Institution, renamed, edited and
/// deleted like any other.
pub fn personal_institution(name: &str) -> Institution {
    Institution {
        id: InstitutionId::SEED_PERSONAL,
        name: name.to_string(),
        products: PERSONAL_PRODUCTS.to_string(),
    }
}

/// The Personal Institution, then the mockup's own six seeded rows in its own order, each with its
/// fixed id: seven, as the mockup's scope note counts. Ids match `lib-accounts`' stub, whose
/// Accounts name these Institutions.
pub fn default_institutions() -> Vec<Institution> {
    std::iter::once(personal_institution("Personal"))
        .chain(
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
            }),
        )
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

/// The trimmed name `draft` asks for, if it is set and no Institution but `own` holds it. `own` is
/// the Institution being edited, so a change of capitals only is not a clash with itself.
fn checked_name<'a>(
    institutions: &[Institution],
    draft: &'a InstitutionDraft,
    own: Option<InstitutionId>,
) -> Result<&'a str, InstitutionError> {
    let name = draft.name.trim();
    if name.is_empty() {
        return Err(InstitutionError::NameRequired);
    }
    let lowered = name.to_lowercase();
    if let Some(holder) = institutions.iter().find(|institution| {
        Some(institution.id) != own && institution.name.to_lowercase() == lowered
    }) {
        return Err(InstitutionError::DuplicateName(holder.name.clone()));
    }
    Ok(name)
}

/// Appends the Institution `draft` describes, with a fresh id, and returns it. The name is stored
/// trimmed and must be set and case-insensitively unique, so persistence can keep `name` unique;
/// `products` is the joined label the dialog built, stored as given. A refusal leaves
/// `institutions` untouched.
pub fn insert_institution(
    institutions: &mut Vec<Institution>,
    draft: &InstitutionDraft,
) -> Result<Institution, InstitutionError> {
    let name = checked_name(institutions, draft, None)?;
    let institution = Institution {
        id: InstitutionId::new(),
        name: name.to_string(),
        products: draft.products.clone(),
    };
    institutions.push(institution.clone());
    Ok(institution)
}

/// Rewrites Institution `id` in place to what `draft` describes, keeping its id and its place in
/// the order. The name rule is the add rule's, skipping the Institution's own row. A refusal leaves
/// `institutions` untouched.
pub fn edit_institution(
    institutions: &mut [Institution],
    id: InstitutionId,
    draft: &InstitutionDraft,
) -> Result<(), InstitutionError> {
    if get(institutions, id).is_none() {
        return Err(InstitutionError::NotFound);
    }
    let name = checked_name(institutions, draft, Some(id))?.to_string();
    let institution = institutions
        .iter_mut()
        .find(|institution| institution.id == id)
        .ok_or(InstitutionError::NotFound)?;
    institution.name = name;
    institution.products = draft.products.clone();
    Ok(())
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
    fn default_institutions_is_the_personal_institution_then_the_mockups_six_rows() {
        let names: Vec<_> = default_institutions()
            .into_iter()
            .map(|institution| institution.name)
            .collect();
        assert_eq!(
            names,
            vec![
                "Personal",
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
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), seeded.len());
        assert_eq!(
            get(&seeded, InstitutionId::SEED_WESTPAC).map(|i| i.name.as_str()),
            Some("Westpac Banking")
        );
    }

    #[test]
    fn the_personal_institution_takes_its_fixed_id_the_name_given_and_cash() {
        let personal = personal_institution("Persönlich");
        assert_eq!(personal.id, InstitutionId::SEED_PERSONAL);
        assert_eq!(personal.name, "Persönlich");
        assert_eq!(personal.products, "cash");
        assert_eq!(default_institutions()[0].id, InstitutionId::SEED_PERSONAL);
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

    #[test]
    fn an_edit_rewrites_the_row_in_place_and_a_change_of_capitals_is_accepted() {
        let mut institutions = default_institutions();

        edit_institution(
            &mut institutions,
            InstitutionId::SEED_WESTPAC,
            &draft(" westpac banking "),
        )
        .expect("its own name, in other capitals");

        assert_eq!(institutions[4].name, "westpac banking");
        assert_eq!(institutions[4].products, "savings \u{b7} offset");
        assert_eq!(institutions.len(), 7);
    }

    #[test]
    fn an_edit_refuses_a_blank_name_another_holds_or_a_missing_row_and_changes_nothing() {
        let mut institutions = default_institutions();
        let before = institutions.clone();

        assert_eq!(
            edit_institution(&mut institutions, InstitutionId::SEED_WESTPAC, &draft("  ")),
            Err(InstitutionError::NameRequired)
        );
        assert_eq!(
            edit_institution(
                &mut institutions,
                InstitutionId::SEED_WESTPAC,
                &draft("anz banking group")
            ),
            Err(InstitutionError::DuplicateName("ANZ Banking Group".into()))
        );
        assert_eq!(
            edit_institution(&mut institutions, InstitutionId::new(), &draft("Bank X")),
            Err(InstitutionError::NotFound)
        );
        assert_eq!(
            edit_institution(&mut institutions, InstitutionId::new(), &draft("  ")),
            Err(InstitutionError::NotFound),
            "a missing row is reported before its draft's name"
        );
        assert_eq!(institutions, before);
    }
}
