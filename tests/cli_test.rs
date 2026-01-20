use assert_cmd::cargo_bin;
use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

#[test]
fn test_cli_end_to_end() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::new(cargo_bin!());
    cmd.arg("tests/fixtures/test.csv");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(
            "client,available,held,total,locked",
        ))
        // Check for client 1
        .stdout(predicate::str::contains("1,1.5,0,1.5,false"))
        // Check for client 2
        .stdout(predicate::str::contains("2,2,0,2,false"));

    Ok(())
}
