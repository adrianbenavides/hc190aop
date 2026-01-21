use assert_cmd::cargo_bin;
use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

#[test]
fn test_boundary_numerical_values() {
    let output_path = std::path::PathBuf::from("boundary_test.csv");
    let mut wtr = csv::Writer::from_path(&output_path).unwrap();
    wtr.write_record(["type", "client", "tx", "amount"])
        .unwrap();

    // u16::MAX = 65535, u32::MAX = 4294967295
    wtr.write_record(["deposit", "65535", "4294967295", "1000000.0000"])
        .unwrap();
    wtr.flush().unwrap();
    drop(wtr);

    let mut cmd = Command::new(cargo_bin!("hc190aop"));
    cmd.arg(&output_path);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(
            "client,available,held,total,locked",
        ))
        .stdout(predicate::str::contains("65535,1000000,0,1000000,false"));

    std::fs::remove_file(output_path).ok();
}

#[test]
fn test_extreme_decimal_precision() {
    let output_path = std::path::PathBuf::from("precision_test.csv");
    let mut wtr = csv::Writer::from_path(&output_path).unwrap();
    wtr.write_record(["type", "client", "tx", "amount"])
        .unwrap();

    wtr.write_record(["deposit", "1", "1", "0.0001"]).unwrap();
    wtr.write_record(["deposit", "1", "2", "0.0001"]).unwrap();
    wtr.flush().unwrap();
    drop(wtr);

    let mut cmd = Command::new(cargo_bin!("hc190aop"));
    cmd.arg(&output_path);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("1,0.0002,0,0.0002,false"));

    std::fs::remove_file(output_path).ok();
}
