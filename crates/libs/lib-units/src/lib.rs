//! Units: the currencies, cryptocurrencies and other commodities a Ledger counts in (`GLOSSARY.md`'s
//! Unit), with their price sources. Pure and I/O-free: the desktop's Settings › Units page reads
//! them through `UnitsStore`, and the Type selector's labels stay in the desktop, which owns the
//! Message catalogue.

/// One row of the **Units** section's table (`docs/ux/desktop-mockups/16-settings/README.md`'s "2a resting
/// state" markup: CODE / NAME / FLAGS / SOURCE / TYPE / ACTIONS columns as of issue #189, up
/// from CODE / NAME / TYPE). Owned `String` fields, not `&'static str` --
/// issue #184's own Add unit dialog is this crate's first real typed-text input, so a row can
/// now hold text a person actually typed, not just compile-time dummy data. `kind` stays a
/// free-form string rather than `UnitKind`: the table just displays it, and legacy seeded rows
/// ("crypto", "etf") don't match any `UnitKind` label anyway. Issue #185's Edit dialog reads it
/// back through `UnitKind::from_label` to pre-fill the Type selector, but the row itself is
/// never typed narrower than a string -- the desktop's own dialog selector governs the Type control.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitRow {
    pub code: String,
    pub name: String,
    pub kind: String,
    /// The SOURCE column shorthand (issue #189) -- e.g. `"USD"` (aud's rate is sourced against
    /// USD as the reference currency), `"CoinGecko"`, `"Manual entry"`. A unit added via the Add
    /// unit dialog (issue #184) has no real price-source integration, so it defaults to `"Manual
    /// entry"`, the same fallback `vas` already uses.
    pub source: String,
    /// The FLAGS column's `base` pill (`.tag.tag-accent`) -- the ledger's own base unit. Only
    /// `aud` carries this in the seeded data; nothing in this map lets a user change which unit
    /// is base, so a dialog-created row always defaults to `false`.
    pub is_base: bool,
    /// The FLAGS column's `default` pill (`.tag.tag-outline`) -- the default unit for new
    /// entries (formerly the removed "Ledger & units" section's own standalone control, now
    /// folded into this one flag). Same "only `aud`, dialogs default to `false`" reasoning as
    /// [`Self::is_base`].
    pub is_default: bool,
}

/// The mockup's own three seeded rows, in its own order -- a function rather than a `const`
/// slice now that [`UnitRow`] owns its strings (`String` has no `const` constructor).
pub fn default_units() -> Vec<UnitRow> {
    vec![
        UnitRow {
            code: "aud".to_string(),
            name: "Australian Dollar".to_string(),
            kind: "currency".to_string(),
            source: "USD".to_string(),
            is_base: true,
            is_default: true,
        },
        UnitRow {
            code: "btc".to_string(),
            name: "Bitcoin".to_string(),
            kind: "crypto".to_string(),
            source: "CoinGecko".to_string(),
            is_base: false,
            is_default: false,
        },
        UnitRow {
            code: "vas".to_string(),
            name: "Vanguard Aus Shares".to_string(),
            kind: "etf".to_string(),
            source: "Manual entry".to_string(),
            is_base: false,
            is_default: false,
        },
    ]
}

/// One row of the **Units** section's own "Price Sources" subsection (issue #189's own README
/// revision) -- a table about the *relationship* to an external price source, not the unit
/// record itself, so it's keyed by the unit's `name`, not its `code` (the README's own "the one
/// place the unit is referenced by name rather than code" callout). Only `btc`/`vas` appear here
/// -- `aud` has no price-source relationship of this kind (its own SOURCE column value, "USD",
/// is a reference-currency note on the Units table itself, not an external feed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PriceSourceRow {
    pub unit_name: String,
    pub source: String,
    pub last_updated: String,
}

/// The mockup's own two seeded rows, in its own order.
pub fn default_price_sources() -> Vec<PriceSourceRow> {
    vec![
        PriceSourceRow {
            unit_name: "Bitcoin".to_string(),
            source: "CoinGecko \u{2014} auto, every 15 min".to_string(),
            last_updated: "5 minutes ago".to_string(),
        },
        PriceSourceRow {
            unit_name: "Vanguard Aus Shares".to_string(),
            source: "Manual entry".to_string(),
            last_updated: "\u{2014}".to_string(),
        },
    ]
}

/// The Units table and the Price Sources subsection, held together so one service answers both.
pub struct UnitService {
    units: Vec<UnitRow>,
    price_sources: Vec<PriceSourceRow>,
}

impl UnitService {
    pub fn new(units: Vec<UnitRow>, price_sources: Vec<PriceSourceRow>) -> Self {
        Self {
            units,
            price_sources,
        }
    }

    pub fn units(&self) -> &[UnitRow] {
        &self.units
    }

    pub fn unit(&self, index: usize) -> Option<&UnitRow> {
        self.units.get(index)
    }

    pub fn price_sources(&self) -> &[PriceSourceRow] {
        &self.price_sources
    }

    /// Appends a unit from the Add unit dialog. It has no price-source integration yet, so its
    /// source is "Manual entry", and it can't be the base or default unit: nothing lets a user
    /// change which one that is.
    pub fn add_unit(&mut self, code: String, name: String, kind: String) {
        self.units.push(UnitRow {
            code,
            name,
            kind,
            source: MANUAL_SOURCE.to_string(),
            is_base: false,
            is_default: false,
        });
    }

    /// Rewrites the row at `index` from the Edit unit dialog. The dialog has no field for the
    /// source or the base and default flags, so those are kept from the row being edited.
    /// Returns `false` when `index` is out of bounds.
    pub fn edit_unit(&mut self, index: usize, code: String, name: String, kind: String) -> bool {
        let Some(existing) = self.units.get_mut(index) else {
            return false;
        };
        *existing = UnitRow {
            code,
            name,
            kind,
            source: existing.source.clone(),
            is_base: existing.is_base,
            is_default: existing.is_default,
        };
        true
    }

    /// Removes the row at `index` if it still holds `code`, the code the confirm dialog opened
    /// on. The dialog is modal, so this is a defensive check, not a normal path.
    pub fn remove_unit(&mut self, index: usize, code: &str) -> Option<UnitRow> {
        if self.units.get(index).is_some_and(|row| row.code == code) {
            Some(self.units.remove(index))
        } else {
            None
        }
    }
}

/// The SOURCE a dialog-created unit gets, the same fallback the seeded `vas` row uses.
const MANUAL_SOURCE: &str = "Manual entry";

#[cfg(test)]
mod tests {
    use super::*;

    fn service() -> UnitService {
        UnitService::new(default_units(), default_price_sources())
    }

    #[test]
    fn a_dialog_added_unit_is_manual_and_neither_base_nor_default() {
        let mut service = service();
        service.add_unit("eth".into(), "Ethereum".into(), "crypto".into());
        let added = service.units().last().expect("the unit was appended");
        assert_eq!(added.code, "eth");
        assert_eq!(added.source, "Manual entry");
        assert!(!added.is_base && !added.is_default);
    }

    #[test]
    fn editing_keeps_the_source_and_the_base_and_default_flags() {
        let mut service = service();
        assert!(service.edit_unit(0, "aud2".into(), "Aussie".into(), "currency".into()));
        let edited = service.unit(0).expect("row 0 exists");
        assert_eq!(edited.code, "aud2");
        assert_eq!(edited.source, "USD");
        assert!(edited.is_base && edited.is_default);
    }

    #[test]
    fn editing_past_the_end_changes_nothing() {
        let mut service = service();
        let before = service.units().to_vec();
        assert!(!service.edit_unit(99, "x".into(), "x".into(), "currency".into()));
        assert_eq!(service.units(), before.as_slice());
    }

    #[test]
    fn removing_checks_the_code_the_dialog_opened_on() {
        let mut service = service();
        assert!(service.remove_unit(1, "aud").is_none());
        assert_eq!(service.units().len(), 3);
        let removed = service.remove_unit(1, "btc").expect("btc is at row 1");
        assert_eq!(removed.code, "btc");
        assert_eq!(service.units().len(), 2);
    }
}
