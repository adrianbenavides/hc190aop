use assert_cmd::cargo_bin;
use std::path::PathBuf;
use std::process::Command;

mod common;

#[test]
fn test_large_file_streaming() {
    let output_path = PathBuf::from("tests/fixtures/large_test.csv");
    if !output_path.exists() {
        common::generate_large_csv(&output_path, 100).expect("Failed to generate large CSV");
    }
    let status = Command::new(cargo_bin!("hc190aop"))
        .arg(&output_path)
        .status()
        .expect("Failed to execute command");
    assert!(status.success(), "Binary failed to process 100MB file");
}
