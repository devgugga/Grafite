use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_grafite"))
}

#[test]
fn why_requires_a_path_argument() {
    let output = bin().arg("why").output().expect("binary runs");
    assert!(!output.status.success(), "missing argument must fail");
    assert!(output.stdout.is_empty(), "stdout must stay clean on error");
}

#[test]
fn unknown_subcommand_fails_without_touching_stdout() {
    let output = bin().arg("nonesuch").output().expect("binary runs");
    assert!(!output.status.success());
    assert!(output.stdout.is_empty(), "stdout must stay clean on error");
}
