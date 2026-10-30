//! Keyboard tests for Settings › Ledger on an opened Ledger file (#922): THIS LEDGER's counts and
//! the base Unit guard's figures are the file's rows, not the stub Ledger's.

mod common;

use std::path::Path;

use common::{cold_shell, ledger_file_with, press, run_command, screen};
use lib_core::{AccountId, InstitutionId, UnitId};
use lib_database::{InstitutionsStore, TransactionsStore};

/// What the file holds once [`differing_file`] has changed it.
struct Counts {
    accounts: usize,
    transactions: usize,
    units: usize,
    institutions: usize,
    in_btc: usize,
}

/// A Ledger file at `path` whose every count differs from the stub's: one Unit, Account and
/// Institution more, the Account in btc, and one Transaction fewer. `None` when the seed has no
/// btc to copy.
fn differing_file(path: &Path) -> Option<Counts> {
    let mut counts = None;
    ledger_file_with(path, |storage| {
        let Some(mut xau) = storage.units.units().unit(UnitId::SEED_BTC) else {
            return;
        };
        xau.id = UnitId::new();
        xau.code = "xau".to_string();
        xau.name = "Gold".to_string();
        let unused = lib_units::UnitUse::default();
        storage.units.split_mut(&unused).units.upsert_unit(xau);

        let mut account = storage.accounts.accounts().accounts()[0].clone();
        account.id = AccountId::new();
        account.unit = UnitId::SEED_BTC;
        storage.accounts.accounts_mut().upsert_account(account);

        let mut institution = storage.institutions.institutions().institutions()[0].clone();
        institution.id = InstitutionId::new();
        institution.name = "Gold Vault".to_string();
        storage
            .institutions
            .institutions_mut()
            .upsert_institution(institution);

        let gone = storage.transactions.all_transactions()[0].id;
        storage
            .transactions
            .transactions_mut()
            .delete_transaction(gone);

        let accounts = storage.accounts.accounts().accounts();
        counts = Some(Counts {
            in_btc: accounts
                .iter()
                .filter(|account| account.unit == UnitId::SEED_BTC)
                .count(),
            accounts: accounts.len(),
            transactions: storage.transactions.all_transactions().len(),
            units: storage.units.units().units().len(),
            institutions: storage.institutions.institutions().institutions().len(),
        });
    });
    counts
}

/// A count as the TUI draws it, grouped in thousands.
fn drawn(count: usize) -> String {
    let digits = count.to_string();
    let mut grouped = String::new();
    for (i, digit) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    grouped
}

#[test]
fn this_ledger_counts_the_opened_file_s_rows() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let path = directory.path().join("household.pldb");
    let counts = differing_file(&path).expect("the seed has btc");
    let mut shell = cold_shell(None);
    run_command(&mut shell, &format!("open {}", path.display()));

    run_command(&mut shell, "settings ledger");
    let frame = screen(&mut shell);

    let labels = frame
        .lines()
        .position(|line| line.contains("transactions") && line.contains("institutions"))
        .expect("THIS LEDGER's labels are drawn");
    let row: Vec<&str> = frame
        .lines()
        .nth(labels + 1)
        .unwrap_or_default()
        .split(|c: char| c.is_whitespace() || c == '│')
        .filter(|cell| !cell.is_empty())
        .collect();
    let expected = [
        counts.accounts,
        counts.transactions,
        counts.units,
        counts.institutions,
    ]
    .map(drawn);
    assert_eq!(row, expected, "{frame}");
}

#[test]
fn the_base_unit_guard_quotes_the_opened_file_s_rows() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let path = directory.path().join("household.pldb");
    let counts = differing_file(&path).expect("the seed has btc");
    let mut shell = cold_shell(None);
    run_command(&mut shell, &format!("open {}", path.display()));
    run_command(&mut shell, "settings ledger");

    // base unit, open it, choose btc, commit: the guard opens on aud → btc.
    press(&mut shell, "G Enter Right Enter");
    let frame = screen(&mut shell);

    assert!(frame.contains("aud → btc"), "the guard is open:\n{frame}");
    let transactions = format!("{} · unchanged", drawn(counts.transactions));
    assert!(frame.contains(&transactions), "{transactions}:\n{frame}");
    let accounts = format!(
        "{} · {} already in btc",
        drawn(counts.accounts),
        drawn(counts.in_btc)
    );
    assert!(frame.contains(&accounts), "{accounts}:\n{frame}");
}
