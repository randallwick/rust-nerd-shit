use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rns"))
        .args(args)
        .output()
        .expect("run compiled CLI")
}

#[test]
fn help_is_plain_stdout_and_succeeds_without_a_terminal() {
    let output = run(&["--help"]);
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("Usage: rns"));
    assert!(text.contains("64 columns"));
    assert!(text.contains("--mute"));
    assert!(text.contains("Mute / unmute"));
    assert!(text.contains("--terminal"));
    assert!(text.contains("E                  Greet"));
    assert!(text.contains("native window"));
    assert!(!text.contains('\u{1b}'));
}

#[test]
fn version_is_plain_stdout() {
    let output = run(&["--version"]);
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    assert_eq!(
        output.stdout,
        format!("rns {}\n", env!("CARGO_PKG_VERSION")).as_bytes()
    );
}

#[test]
fn invalid_arguments_report_usage_error_on_stderr() {
    for args in [
        &["quest"][..],
        &["--unknown"],
        &["--help", "--version"],
        &["--mute", "--mute"],
        &["--terminal", "--terminal"],
        &["--help", "--terminal"],
    ] {
        let output = run(args);
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        let text = String::from_utf8(output.stderr).unwrap();
        assert!(text.contains("rns --help"));
        assert!(!text.contains('\u{1b}'));
    }
}

#[test]
fn redirected_gameplay_fails_without_escape_sequences() {
    let output = run(&["--terminal"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let text = String::from_utf8(output.stderr).unwrap();
    assert!(text.contains("interactive stdin and stdout"));
    assert!(!text.contains('\u{1b}'));
}

#[test]
fn mute_is_accepted_but_still_requires_an_interactive_terminal() {
    for args in [["--terminal", "--mute"], ["--mute", "--terminal"]] {
        let output = run(&args);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let error = String::from_utf8(output.stderr).unwrap();
        assert!(error.contains("interactive stdin and stdout"));
        assert!(!error.contains("audio"));
    }
}
