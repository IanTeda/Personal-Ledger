//! Shared stub Payees, seeded from the names the TUI's Payee fixture uses
//! (`crates/bins/bin-tui/src/payee/fixture.rs`) plus the few the Transactions seed data needs.
//! `gpui`-free; the future Payees map grows this module. A Payee carries its former names (Payee
//! Aliases, ADR-0012) so search and filters can grow to match them; nothing matches on them yet.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Payee {
    pub id: u32,
    /// The current, canonical name.
    pub name: String,
    /// Former names, kept when the Payee was renamed.
    pub aliases: Vec<String>,
}

fn payee(id: u32, name: &str, aliases: &[&str]) -> Payee {
    Payee {
        id,
        name: name.to_string(),
        aliases: aliases.iter().map(|alias| alias.to_string()).collect(),
    }
}

pub fn default_payees() -> Vec<Payee> {
    vec![
        payee(1, "Woolworths", &["WOOLIES", "WW Metro"]),
        payee(2, "Coles Online", &["Coles Central"]),
        payee(3, "Aldi Kelvin Grove", &[]),
        payee(4, "Kura Sushi", &[]),
        payee(5, "Cafe Vittoria", &[]),
        payee(6, "BP", &[]),
        payee(7, "Uber", &[]),
        payee(8, "Bunnings Warehouse", &[]),
        payee(9, "Kmart", &[]),
        payee(10, "Officeworks", &[]),
        payee(11, "Origin Energy", &[]),
        payee(12, "Sydney Water", &[]),
        payee(13, "Sunrise Payroll", &[]),
        payee(14, "ANZ Banking Group", &[]),
        payee(15, "Hudson News", &[]),
        payee(16, "Don Quijote", &[]),
    ]
}

/// The id of the Payee named `name` (first match), if any.
pub fn find_by_name(payees: &[Payee], name: &str) -> Option<u32> {
    payees
        .iter()
        .find(|payee| payee.name == name)
        .map(|payee| payee.id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_and_names_are_unique() {
        let payees = default_payees();
        let mut ids: Vec<_> = payees.iter().map(|p| p.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), payees.len());
        let mut names: Vec<_> = payees.iter().map(|p| p.name.to_lowercase()).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), payees.len());
    }

    #[test]
    fn find_by_name_is_exact_and_aliases_are_not_names() {
        let payees = default_payees();
        assert_eq!(find_by_name(&payees, "Woolworths"), Some(1));
        assert_eq!(find_by_name(&payees, "WOOLIES"), None);
        assert!(payees[0].aliases.contains(&"WOOLIES".to_string()));
    }
}
