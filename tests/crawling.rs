use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn no_urls() {
    let mut cmd = Command::cargo_bin("doonop").unwrap();
    let assert = cmd.assert();
    assert.success().code(0).stderr(predicate::str::ends_with(
        "Statistics: visited 0, collected 0, errors 0, retries 0\n",
    ));
}

#[test]
fn basic() {
    let mut cmd = Command::cargo_bin("doonop").unwrap();
    let assert = cmd
        .args(&["-b", "chrome"])
        .arg("http://localhost:8000/www/basic/index.html")
        .assert();
    assert.success().code(0).stderr(predicate::str::ends_with(
        "Statistics: visited 2, collected 2, errors 0, retries 0\n",
    ));
}

#[test]
fn using_side_file() {
    let mut cmd = Command::cargo_bin("doonop").unwrap();
    let assert = cmd
        .args(&["--check-file", "./tests/resources/default.side.json"])
        .args(&["--check-file-format", "side"])
        .args(&["-b", "chrome"])
        .arg("http://localhost:8000/www/basic/index.html")
        .assert();
    assert
        .success()
        .code(0)
        .stderr(predicate::str::ends_with(
            "Statistics: visited 2, collected 2, errors 0, retries 0\n",
        ))
        .stdout(predicate::str::contains("THE RESULT"));
}

#[test]
fn basic_with_invalid_driver() {
    let mut cmd = Command::cargo_bin("doonop").unwrap();
    let assert = cmd
        .args(&["-b", "firefox"])
        .arg("http://localhost:8000/www/basic/index.html")
        .assert();
    assert.success().code(0).stderr(predicate::str::ends_with(
        "Statistics: visited 0, collected 0, errors 0, retries 0\n",
    ));
}
