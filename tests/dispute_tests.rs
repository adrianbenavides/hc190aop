use assert_cmd::cargo_bin;
use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::io::Write;
use std::process::Command;
use tempfile::NamedTempFile;

#[test]
fn test_dispute_resolve_flow_multi_client() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "type, client, tx, amount").unwrap();
    writeln!(file, "deposit, 1, 1, 10.0").unwrap();
    writeln!(file, "deposit, 2, 3, 20.0").unwrap();
    writeln!(file, "deposit, 1, 2, 5.0").unwrap();
    writeln!(file, "dispute, 2, 3, ").unwrap();
    writeln!(file, "dispute, 1, 1, ").unwrap();
    writeln!(file, "resolve, 2, 3, ").unwrap();
    writeln!(file, "resolve, 1, 1, ").unwrap();

    let mut cmd = Command::new(cargo_bin!("hc190aop"));
    cmd.arg(file.path());

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("1,15,0,15,false"))
        .stdout(predicate::str::contains("2,20,0,20,false"));
}

#[test]
fn test_dispute_chargeback_flow_multi_client() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "type, client, tx, amount").unwrap();
    writeln!(file, "deposit, 2, 2, 50.0").unwrap();
    writeln!(file, "deposit, 1, 1, 10.0").unwrap();
    writeln!(file, "dispute, 2, 2, ").unwrap();
    writeln!(file, "dispute, 1, 1, ").unwrap();
    writeln!(file, "chargeback, 1, 1, ").unwrap();
    writeln!(file, "chargeback, 2, 2, ").unwrap();

    let mut cmd = Command::new(cargo_bin!("hc190aop"));
    cmd.arg(file.path());

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("1,0,0,0,true"))
        .stdout(predicate::str::contains("2,0,0,0,true"));
}

#[test]
fn test_locked_account_rejection_multi_client() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "type, client, tx, amount").unwrap();
    writeln!(file, "deposit, 1, 1, 10.0").unwrap();
    writeln!(file, "dispute, 1, 1, ").unwrap();
    writeln!(file, "chargeback, 1, 1, ").unwrap();
    writeln!(file, "deposit, 1, 2, 5.0").unwrap();
    writeln!(file, "deposit, 2, 3, 100.0").unwrap();

    let mut cmd = Command::new(cargo_bin!("hc190aop"));
    cmd.arg(file.path());

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("1,0,0,0,true"))
        .stdout(predicate::str::contains("2,100,0,100,false"));
}

#[test]
fn test_ignore_invalid_dispute_multi_client() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "type, client, tx, amount").unwrap();
    writeln!(file, "deposit, 1, 1, 10.0").unwrap();
    writeln!(file, "dispute, 1, 999, ").unwrap();
    writeln!(file, "deposit, 2, 2, 5.0").unwrap();
    writeln!(file, "dispute, 2, 1, ").unwrap(); // Wrong client ID for tx 1

    let mut cmd = Command::new(cargo_bin!("hc190aop"));
    cmd.arg(file.path());

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("1,10,0,10,false"))
        .stdout(predicate::str::contains("2,5,0,5,false"));
}
