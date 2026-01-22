use assert_cmd::cargo_bin;
use std::path::PathBuf;
use std::process::Command;

mod common;

#[test]
fn test_large_file() {
    let output_path = PathBuf::from("tests/fixtures/large_test.csv");
    if !output_path.exists() {
        common::generate_large_csv(&output_path, 10).expect("Failed to generate large CSV");
    }

    // In memory
    let output1 = Command::new(cargo_bin!("hc190aop"))
        .arg(&output_path)
        .output()
        .expect("Failed to execute command");
    assert!(output1.status.success());
    let stdout1 = String::from_utf8_lossy(&output1.stdout);

    // With DB, remove any existing DB first
    let db_path = PathBuf::from("tests/fixtures/test_db");
    if db_path.exists() {
        std::fs::remove_dir_all(&db_path).expect("Failed to remove existing test DB");
    }
    let output2 = Command::new(cargo_bin!("hc190aop"))
        .arg(&output_path)
        .arg("--db-path")
        .arg(db_path)
        .output()
        .expect("Failed to execute command");
    assert!(output2.status.success());
    let stdout2 = String::from_utf8_lossy(&output2.stdout);

    // Sort lines before comparison to handle different output orders
    let mut lines1: Vec<_> = stdout1.lines().collect();
    let mut lines2: Vec<_> = stdout2.lines().collect();
    lines1.sort_unstable();
    lines2.sort_unstable();

    assert_eq!(
        lines1, lines2,
        "Outputs differ between in-memory and DB modes (after sorting)"
    );
}
