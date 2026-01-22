use assert_cmd::cargo_bin;
use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::io::Write;
use std::process::Command;

#[cfg(not(feature = "storage-rocksdb"))]
#[test]
fn test_rocksdb_fallback_warning() {
    let mut csv = tempfile::NamedTempFile::new().unwrap();
    writeln!(csv, "type, client, tx, amount").unwrap();
    writeln!(csv, "deposit, 1, 1, 100.0").unwrap();

    let mut cmd = Command::new(cargo_bin!("hc190aop"));
    cmd.arg(csv.path()).arg("--db-path").arg("some_db");

    cmd.assert()
        .success()
        .stderr(predicate::str::contains("WARNING: Persistent storage requested via --db-path, but 'storage-rocksdb' feature is not enabled. Falling back to In-Memory storage."));
}

#[cfg(feature = "storage-rocksdb")]
#[test]
fn test_rocksdb_no_fallback_warning() {
    let mut csv = tempfile::NamedTempFile::new().unwrap();
    writeln!(csv, "type, client, tx, amount").unwrap();
    writeln!(csv, "deposit, 1, 1, 100.0").unwrap();

    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test_db");

    let mut cmd = Command::new(cargo_bin!("hc190aop"));
    cmd.arg(csv.path()).arg("--db-path").arg(&db_path);

    cmd.assert()
        .success()
        .stderr(predicate::str::contains("WARNING").not());
}
