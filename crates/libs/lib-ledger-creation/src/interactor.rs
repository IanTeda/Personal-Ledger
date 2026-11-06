//! Create a Ledger, the module's one Interactor: it checks the base Unit by Add unit's rules,
//! builds what every new Ledger starts with around it and creates the file through the port once.
//! A refusal or a failure writes nothing and raises the Error Toast that says why (ADR-0048);
//! the Client opens the new Ledger and says it was created.

use std::path::{Path, PathBuf};

use lib_core::UnitId;
use lib_locale::Currency;
use lib_toast::{Notice, Notify, ToastKind};
use lib_units::interactor::AddUnit;
use lib_units::{Precision, PriceLink, UnitKind, UnitRow, price_source_exists};

use crate::error::{Error, Result};
use crate::msg;
use crate::protocols::{CreationFailure, LedgerCreation};
use crate::starting::NewLedger;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateLedger {
    /// Where the Ledger file goes; nothing may be there yet.
    pub path: PathBuf,
    /// The base Unit, as Add unit would add it. It is the Ledger's only Unit, so it is quoted in
    /// itself.
    pub base_unit: AddUnit,
    /// The Personal Institution's name, the Client's "Personal" Message (ADR-0049).
    pub personal: String,
}

/// What a create made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerCreated {
    pub path: PathBuf,
    pub base_unit: UnitId,
}

impl CreateLedger {
    /// Creates the Ledger file, or raises the Error Toast that says why it didn't through
    /// `notify`.
    ///
    /// # Errors
    /// `CodeNeeded`, `NameNeeded` or `Unit` for a base Unit Add unit would refuse, and `Failed`
    /// when the port wrote no file.
    pub fn run(
        self,
        creation: &mut dyn LedgerCreation,
        notify: &mut dyn Notify,
    ) -> Result<LedgerCreated> {
        let path = self.path.clone();
        let base_unit = self.base_unit.id;
        let kind = self.base_unit.kind;
        let outcome = self.apply(creation);
        match &outcome {
            Ok(_) => tracing::info!(
                path = %path.display(),
                unit_id = %base_unit,
                ?kind,
                "Created a Ledger"
            ),
            Err(refusal) => {
                tracing::info!(path = %path.display(), %refusal, "Refused to create a Ledger");
                notify.notify(Notice::Toast(
                    ToastKind::Error,
                    lib_locale::msg::toast_ledger_create_failed(
                        &file_name(&path),
                        &reason(*refusal),
                    ),
                ));
            }
        }
        outcome
    }

    fn apply(self, creation: &mut dyn LedgerCreation) -> Result<LedgerCreated> {
        let unit = checked(self.base_unit)?;
        let base_unit = unit.id;
        creation.create_ledger(&self.path, &NewLedger::new(unit, &self.personal))?;
        Ok(LedgerCreated {
            path: self.path,
            base_unit,
        })
    }
}

/// The base Unit a new Ledger is prefilled with: `currency`, the Locale's (`Locale::currency`), on
/// Manual entry and quoted in itself, named in the Locale in effect.
pub fn base_unit_for(currency: Currency) -> AddUnit {
    let id = UnitId::new();
    AddUnit {
        id,
        code: currency.code().to_string(),
        name: currency.name(),
        kind: UnitKind::Currency,
        precision: Precision::of_kind(UnitKind::Currency),
        link: PriceLink::manual(id),
    }
}

/// The base Unit Add unit would add, as the Ledger's only Unit: no other Unit or stored Price
/// Source exists yet, so it is quoted in itself and priced by Manual entry or a built-in.
fn checked(draft: AddUnit) -> Result<UnitRow> {
    let code = draft.code.trim();
    let name = draft.name.trim();
    if code.is_empty() {
        return Err(Error::CodeNeeded);
    }
    if name.is_empty() {
        return Err(Error::NameNeeded);
    }
    if !price_source_exists(&[], draft.link.source_id) {
        return Err(lib_units::Error::PriceSourceGone.into());
    }
    if draft.link.quote != draft.id {
        return Err(lib_units::Error::QuoteGone.into());
    }
    Ok(UnitRow {
        id: draft.id,
        ..UnitRow::new(
            code.to_string(),
            name.to_string(),
            draft.kind,
            draft.precision,
            draft.link,
        )
    })
}

/// The Toast's reason for `refusal`.
fn reason(refusal: Error) -> String {
    match refusal {
        Error::CodeNeeded => msg::ledger_creation_code_needed(),
        Error::NameNeeded => msg::ledger_creation_name_needed(),
        Error::Unit(lib_units::Error::PriceSourceGone) => msg::ledger_creation_price_source_gone(),
        Error::Unit(lib_units::Error::QuoteGone) => msg::ledger_creation_quote_gone(),
        Error::Unit(other) => {
            debug_assert!(false, "creating a Ledger never refuses with {other:?}");
            msg::ledger_creation_base_unit_invalid()
        }
        Error::Failed(CreationFailure::Exists) => msg::ledger_creation_exists(),
        Error::Failed(CreationFailure::Unwritable) => msg::ledger_creation_unwritable(),
    }
}

/// The name a Toast gives the file at `path`.
fn file_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .into_owned()
}

#[cfg(test)]
mod tests {
    use lib_core::PriceSourceId;
    use lib_units::COINGECKO;

    use super::*;
    use crate::fake::InMemoryLedgerCreation;

    fn path() -> PathBuf {
        PathBuf::from("/ledgers/teda.pldb")
    }

    /// The base Unit en-AU prefills.
    fn aud() -> AddUnit {
        let id = UnitId::new();
        AddUnit {
            id,
            code: "aud".into(),
            name: "Australian Dollar".into(),
            kind: UnitKind::Currency,
            precision: Precision::of_kind(UnitKind::Currency),
            link: PriceLink::manual(id),
        }
    }

    fn create(
        port: &mut InMemoryLedgerCreation,
        base_unit: AddUnit,
    ) -> (Result<LedgerCreated>, Vec<Notice>) {
        lib_locale::init_with_layers("en-US", &[msg::LAYER]);
        let mut notices = Vec::new();
        let outcome = CreateLedger {
            path: path(),
            base_unit,
            personal: "Personal".into(),
        }
        .run(port, &mut notices);
        (outcome, notices)
    }

    /// Runs a create that should be refused, checks it wrote nothing and returns the refusal and
    /// the Toast's text.
    fn refused(base_unit: AddUnit) -> (Error, String) {
        let mut port = InMemoryLedgerCreation::default();
        let (outcome, notices) = create(&mut port, base_unit);
        assert_eq!(port.calls, 0, "a refused base Unit never reaches the port");
        let refusal = outcome.expect_err("the base Unit should be refused");
        let [Notice::Toast(ToastKind::Error, text)] = notices.as_slice() else {
            panic!("one Error Toast, not {notices:?}");
        };
        (refusal, text.clone())
    }

    #[test]
    fn a_new_ledger_starts_with_the_personal_institution_the_base_unit_personal_spending_and_the_default_types()
     {
        let mut port = InMemoryLedgerCreation::default();
        let base = aud();

        let (outcome, notices) = create(&mut port, base.clone());

        let created = outcome.expect("a free path and a valid base Unit");
        assert_eq!(created.base_unit, base.id);
        assert!(notices.is_empty(), "the Client says it was created");
        assert_eq!(port.calls, 1, "the port is called once");
        let ledger = &port.ledgers[&path()];
        assert_eq!(ledger.personal.id, lib_core::InstitutionId::SEED_PERSONAL);
        assert_eq!(ledger.personal.name, "Personal");
        assert_eq!(ledger.personal.usual_unit, None, "no Usual unit");
        assert_eq!(
            (ledger.base_unit.id, ledger.base_unit.code.as_str()),
            (base.id, "aud")
        );
        assert!(ledger.base_unit.is_active && ledger.base_unit.prices.is_empty());
        assert_eq!(ledger.budget.unit, base.id);
        assert!(ledger.budget.is_default);
        assert!(ledger.budget.account_ids.is_empty(), "on no Accounts");
        assert!(ledger.budget.limits.is_empty(), "with no amounts");
        assert_eq!(ledger.document_types, lib_documents::types::default_types());
    }

    #[test]
    fn the_base_units_code_and_name_are_trimmed() {
        let mut port = InMemoryLedgerCreation::default();

        let (outcome, _) = create(
            &mut port,
            AddUnit {
                code: " aud ".into(),
                name: "Australian Dollar\t".into(),
                ..aud()
            },
        );

        outcome.expect("a valid base Unit");
        let unit = &port.ledgers[&path()].base_unit;
        assert_eq!(
            (unit.code.as_str(), unit.name.as_str()),
            ("aud", "Australian Dollar")
        );
    }

    #[test]
    fn a_base_unit_on_a_built_in_source_is_taken() {
        let mut port = InMemoryLedgerCreation::default();
        let base = aud();
        let id = base.id;

        let (outcome, _) = create(
            &mut port,
            AddUnit {
                kind: UnitKind::Crypto,
                link: PriceLink {
                    source_id: COINGECKO,
                    symbol: "bitcoin".into(),
                    quote: id,
                },
                ..base
            },
        );

        assert!(outcome.is_ok(), "{outcome:?}");
    }

    #[test]
    fn a_base_unit_with_no_code_is_refused() {
        let (refusal, text) = refused(AddUnit {
            code: "  ".into(),
            ..aud()
        });

        assert_eq!(refusal, Error::CodeNeeded);
        assert_eq!(
            text,
            "Couldn't create ledger teda.pldb: its base unit needs a code"
        );
    }

    #[test]
    fn a_base_unit_with_no_name_is_refused() {
        let (refusal, _) = refused(AddUnit {
            name: String::new(),
            ..aud()
        });

        assert_eq!(refusal, Error::NameNeeded);
    }

    #[test]
    fn a_base_unit_quoted_in_another_unit_is_refused() {
        let (refusal, _) = refused(AddUnit {
            link: PriceLink::manual(UnitId::new()),
            ..aud()
        });

        assert_eq!(refusal, Error::Unit(lib_units::Error::QuoteGone));
    }

    #[test]
    fn a_base_unit_on_a_custom_source_is_refused_as_none_exists_yet() {
        let base = aud();
        let id = base.id;
        let (refusal, _) = refused(AddUnit {
            link: PriceLink {
                source_id: PriceSourceId::new(),
                symbol: "x".into(),
                quote: id,
            },
            ..base
        });

        assert_eq!(refusal, Error::Unit(lib_units::Error::PriceSourceGone));
    }

    #[test]
    fn a_path_already_taken_raises_an_error_toast_and_writes_nothing() {
        let mut port = InMemoryLedgerCreation::default();
        port.occupied.insert(path());

        let (outcome, notices) = create(&mut port, aud());

        assert_eq!(outcome, Err(Error::Failed(CreationFailure::Exists)));
        assert!(port.ledgers.is_empty());
        assert_eq!(
            notices,
            [Notice::Toast(
                ToastKind::Error,
                "Couldn't create ledger teda.pldb: a file of that name is already there".into()
            )]
        );
    }

    #[test]
    fn a_failed_write_raises_an_error_toast() {
        let mut port = InMemoryLedgerCreation {
            broken: true,
            ..InMemoryLedgerCreation::default()
        };

        let (outcome, notices) = create(&mut port, aud());

        assert_eq!(outcome, Err(Error::Failed(CreationFailure::Unwritable)));
        assert_eq!(port.calls, 1);
        assert_eq!(
            notices,
            [Notice::Toast(
                ToastKind::Error,
                "Couldn't create ledger teda.pldb: it can't be written there".into()
            )]
        );
    }

    #[test]
    fn each_locales_prefill_is_its_currency_on_manual_entry_and_creates_a_ledger() {
        lib_locale::init_with_layers("en-US", &[msg::LAYER]);
        for (locale, code, name) in [
            (lib_locale::Locale::EnUs, "usd", "US Dollar"),
            (lib_locale::Locale::EnGb, "gbp", "British Pound"),
            (lib_locale::Locale::EnAu, "aud", "Australian Dollar"),
            (lib_locale::Locale::EnXa, "usd", "US Dollar"),
        ] {
            let base = base_unit_for(locale.currency());
            assert_eq!(
                (base.code.as_str(), base.name.as_str(), base.kind),
                (code, name, UnitKind::Currency),
                "{locale}"
            );
            assert_eq!(base.link, PriceLink::manual(base.id));
            let mut port = InMemoryLedgerCreation::default();
            assert!(create(&mut port, base).0.is_ok(), "{locale}");
        }
    }
}
