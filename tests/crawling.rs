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
#[ignore = "Thirtyfour seems doesn't handle such a case"]
fn basic_with_invalid_driver() {
    let mut cmd = Command::cargo_bin("doonop").unwrap();
    let assert = cmd
        .args(&["-b", "firefox"])
        .arg("http://localhost:8000/www/basic/index.html")
        .assert();
    assert.success().code(0).stderr(predicate::str::ends_with(
        "Statistics: visited 2, collected 2, errors 0, retries 0\n",
    ));
}
