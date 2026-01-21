use assert_cmd::cargo_bin;
use std::io::Write;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_rocksdb_persistence_recovery() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test_db");

    // 1. First run: Process a deposit
    let mut csv1 = tempfile::NamedTempFile::new().unwrap();
    writeln!(csv1, "type, client, tx, amount").unwrap();
    writeln!(csv1, "deposit, 1, 1, 100.0").unwrap();

    let mut cmd1 = Command::new(cargo_bin!("hc190aop"));
    cmd1.arg(csv1.path()).arg("--db-path").arg(&db_path);

    let output1 = cmd1.output().expect("Failed to execute command");
    assert!(output1.status.success());
    let stdout1 = String::from_utf8_lossy(&output1.stdout);
    assert!(stdout1.contains("1,100,0,100,false"));

    // 2. Second run: Process another deposit using the same DB path
    let mut csv2 = tempfile::NamedTempFile::new().unwrap();
    writeln!(csv2, "type, client, tx, amount").unwrap();
    writeln!(csv2, "deposit, 1, 2, 50.0").unwrap();

    let mut cmd2 = Command::new(cargo_bin!("hc190aop"));
    cmd2.arg(csv2.path()).arg("--db-path").arg(&db_path);

    let output2 = cmd2.output().expect("Failed to execute command");
    assert!(output2.status.success());
    let stdout2 = String::from_utf8_lossy(&output2.stdout);

    // Should have recovered 100.0 and added 50.0 = 150.0
    assert!(stdout2.contains("1,150,0,150,false"));
}
