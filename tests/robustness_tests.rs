use assert_cmd::cargo_bin;
use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

#[test]
fn test_malformed_csv_handling() {
    let output_path = std::path::PathBuf::from("robustness_test.csv");
    let mut wtr = csv::Writer::from_path(&output_path).unwrap();
    wtr.write_record(&["type", "client", "tx", "amount"])
        .unwrap();

    // Valid deposit
    wtr.write_record(&["deposit", "1", "1", "1.0"]).unwrap();
    // Invalid type
    wtr.write_record(&["invalid", "1", "2", "1.0"]).unwrap();
    // Missing amount for deposit (required)
    wtr.write_record(&["deposit", "1", "3", ""]).unwrap();
    // Valid deposit again
    wtr.write_record(&["deposit", "1", "4", "2.0"]).unwrap();
    wtr.flush().unwrap();
    drop(wtr);

    let mut cmd = Command::new(cargo_bin!("hc190aop"));
    cmd.arg(&output_path);

    cmd.assert()
        .success()
        .stderr(predicate::str::contains("Error reading transaction"))
        .stdout(predicate::str::contains("1,3,0,3,false")); // 1.0 + 2.0 = 3.0

    std::fs::remove_file(output_path).ok();
}

#[test]
fn test_invalid_data_types() {
    let output_path = std::path::PathBuf::from("data_type_test.csv");
    let mut wtr = csv::Writer::from_path(&output_path).unwrap();
    wtr.write_record(&["type", "client", "tx", "amount"])
        .unwrap();

    // Text in amount field
    wtr.write_record(&["deposit", "1", "1", "not_a_number"])
        .unwrap();
    // Non-integer client ID
    wtr.write_record(&["deposit", "abc", "2", "1.0"]).unwrap();
    // Valid deposit
    wtr.write_record(&["deposit", "1", "3", "5.0"]).unwrap();
    wtr.flush().unwrap();
    drop(wtr);

    let mut cmd = Command::new(cargo_bin!("hc190aop"));
    cmd.arg(&output_path);

    cmd.assert()
        .success()
        .stderr(predicate::str::contains("Error reading transaction"))
        .stdout(predicate::str::contains("1,5,0,5,false"));

    std::fs::remove_file(output_path).ok();
}
