use assert_cmd::cargo_bin;
use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::io::Write;
use std::process::Command;
use tempfile::NamedTempFile;

#[test]
fn test_dispute_resolve_flow() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "type, client, tx, amount").unwrap();
    writeln!(file, "deposit, 1, 1, 10.0").unwrap();
    writeln!(file, "deposit, 1, 2, 5.0").unwrap();
    writeln!(file, "dispute, 1, 1, ").unwrap(); // Dispute tx 1 (10 held)
    writeln!(file, "resolve, 1, 1, ").unwrap(); // Resolve tx 1 (10 back to available)

    let mut cmd = Command::new(cargo_bin!("hc190aop"));
    cmd.arg(file.path());

    // Expected: 10.0 + 5.0 = 15.0 available, 0 held.
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("1,15,0,15,false"));
}

#[test]
fn test_dispute_chargeback_flow() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "type, client, tx, amount").unwrap();
    writeln!(file, "deposit, 1, 1, 10.0").unwrap();
    writeln!(file, "dispute, 1, 1, ").unwrap(); // 10 held
    writeln!(file, "chargeback, 1, 1, ").unwrap(); // 10 removed, locked

    let mut cmd = Command::new(cargo_bin!("hc190aop"));
    cmd.arg(file.path());

    // Expected: 10.0 deposited, then disputed (10 held), then chargeback (10 removed, locked).
    // Final: 0 available, 0 held, 0 total, locked=true.
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("1,0,0,0,true"));
}

#[test]
fn test_locked_account_rejection() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "type, client, tx, amount").unwrap();
    writeln!(file, "deposit, 1, 1, 10.0").unwrap();
    writeln!(file, "dispute, 1, 1, ").unwrap();
    writeln!(file, "chargeback, 1, 1, ").unwrap();
    writeln!(file, "deposit, 1, 2, 5.0").unwrap(); // Should be ignored because locked

    let mut cmd = Command::new(cargo_bin!("hc190aop"));
    cmd.arg(file.path());

    // Expected: Final state after chargeback remains 0,0,0,true despite the 5.0 deposit.
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("1,0,0,0,true"));
}

#[test]
fn test_ignore_invalid_dispute() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "type, client, tx, amount").unwrap();
    writeln!(file, "deposit, 1, 1, 10.0").unwrap();
    writeln!(file, "dispute, 1, 999, ").unwrap(); // Non-existent tx

    let mut cmd = Command::new(cargo_bin!("hc190aop"));
    cmd.arg(file.path());

    // Expected: 10.0 available, 0 held.
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("1,10,0,10,false"));
}
