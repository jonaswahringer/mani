use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

fn mani() -> Command {
    Command::new(env!("CARGO_BIN_EXE_mani"))
}

fn write_executable(path: &Path, source: &str) {
    fs::write(path, source).unwrap();
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
}

fn isolated_path(bin: &Path) -> String {
    let inherited = std::env::var("PATH").unwrap_or_default();
    format!("{}:{inherited}", bin.display())
}

#[test]
fn short_prints_only_the_custom_introduction() {
    let home = tempfile::tempdir().unwrap();
    fs::create_dir_all(home.path().join("git")).unwrap();
    fs::write(
        home.path().join("git/rebase.md"),
        "# Rebase\n\nKeep commits tidy.\n\n## Continue\n\nLater content.\n",
    )
    .unwrap();

    let output = mani()
        .env("MANI_HOME", home.path())
        .args(["--short", "git", "rebase"])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "# Rebase\n\nKeep commits tidy.\n"
    );
}

#[test]
fn custom_override_fails_with_the_expected_path() {
    let home = tempfile::tempdir().unwrap();

    let output = mani()
        .env("MANI_HOME", home.path())
        .args(["--print", "--custom", "definitely-not-a-command"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("requested Custom source is unavailable"));
    assert!(stderr.contains("definitely-not-a-command.md"));
}

#[test]
fn config_check_reports_unknown_keys() {
    let home = tempfile::tempdir().unwrap();
    fs::write(home.path().join("config.toml"), "unknown = true\n").unwrap();

    let output = mani()
        .env("MANI_HOME", home.path())
        .args(["config", "check"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("config.toml:1:1"));
    assert!(stderr.contains("unknown field"));
}

#[test]
fn short_fallback_executes_the_exact_command_path_with_help() {
    let home = tempfile::tempdir().unwrap();
    let bin = tempfile::tempdir().unwrap();
    let arguments = home.path().join("arguments");
    write_executable(&bin.path().join("man"), "#!/bin/sh\nexit 1\n");
    write_executable(
        &bin.path().join("acme"),
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$MANI_TEST_ARGUMENTS\"\nprintf 'usage: acme deploy\\n'\n",
    );

    let output = mani()
        .env("MANI_HOME", home.path())
        .env("PATH", isolated_path(bin.path()))
        .env("MANI_TEST_ARGUMENTS", &arguments)
        .args(["--short", "acme", "deploy"])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(output.stdout, b"usage: acme deploy\n");
    assert_eq!(fs::read_to_string(arguments).unwrap(), "deploy\n--help\n");
}

#[test]
fn printed_official_source_prefers_the_installed_man_page() {
    let home = tempfile::tempdir().unwrap();
    let bin = tempfile::tempdir().unwrap();
    write_executable(
        &bin.path().join("man"),
        "#!/bin/sh\nif [ \"$1\" = '-w' ]; then\n  printf '/usr/share/man/man1/acme.1.gz\\n'\nelse\n  printf 'ACME(1)\\n\\nOFFICIAL WORDING\\n'\nfi\n",
    );
    write_executable(
        &bin.path().join("acme"),
        "#!/bin/sh\nprintf 'command fallback should not be printed\\n'\n",
    );

    let output = mani()
        .env("MANI_HOME", home.path())
        .env("PATH", isolated_path(bin.path()))
        .args(["--print", "--official", "acme"])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(output.stdout, b"ACME(1)\n\nOFFICIAL WORDING\n");
}

#[test]
fn print_prefers_a_custom_guide_and_keeps_the_complete_document() {
    let home = tempfile::tempdir().unwrap();
    fs::write(
        home.path().join("acme.md"),
        "# Acme\n\nIntroduction.\n\n## Details\n\nComplete content.\n",
    )
    .unwrap();

    let output = mani()
        .env("MANI_HOME", home.path())
        .args(["--print", "acme"])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        output.stdout,
        b"# Acme\n\nIntroduction.\n\n## Details\n\nComplete content.\n"
    );
}

#[test]
fn rejects_invalid_custom_guide_content() {
    let home = tempfile::tempdir().unwrap();
    fs::write(home.path().join("acme.md"), "# Acme\n\n<div>unsafe</div>\n").unwrap();

    let output = mani()
        .env("MANI_HOME", home.path())
        .args(["--print", "acme"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("raw HTML is not allowed")
    );
}

#[test]
fn ignore_config_recovers_from_a_broken_configuration() {
    let home = tempfile::tempdir().unwrap();
    fs::write(home.path().join("config.toml"), "unknown = true\n").unwrap();
    fs::write(home.path().join("acme.md"), "# Acme\n").unwrap();

    let output = mani()
        .env("MANI_HOME", home.path())
        .args(["--ignore-config", "--print", "acme"])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(output.stdout, b"# Acme\n");
}

#[test]
fn redirected_output_obeys_color_and_no_color_rules() {
    let home = tempfile::tempdir().unwrap();
    fs::write(home.path().join("acme.md"), "# Acme\n").unwrap();

    let automatic = mani()
        .env("MANI_HOME", home.path())
        .args(["--print", "acme"])
        .output()
        .unwrap();
    assert!(!automatic.stdout.contains(&0x1b));

    let forced = mani()
        .env("MANI_HOME", home.path())
        .env("NO_COLOR", "1")
        .args(["--color", "always", "--print", "acme"])
        .output()
        .unwrap();
    assert!(forced.stdout.contains(&0x1b));

    let disabled = mani()
        .env("MANI_HOME", home.path())
        .env("NO_COLOR", "1")
        .args(["--print", "acme"])
        .output()
        .unwrap();
    assert!(!disabled.stdout.contains(&0x1b));
}

#[test]
fn interactive_mode_rejects_redirected_terminal_streams() {
    let home = tempfile::tempdir().unwrap();
    fs::write(home.path().join("acme.md"), "# Acme\n").unwrap();

    let output = mani()
        .env("MANI_HOME", home.path())
        .arg("acme")
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("requires terminal stdin and stdout")
    );
}

#[test]
fn trailing_command_options_are_rejected_instead_of_becoming_mani_flags() {
    let home = tempfile::tempdir().unwrap();

    let output = mani()
        .env("MANI_HOME", home.path())
        .args(["--short", "git", "--version"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("lookup does not accept command options or operands: --version")
    );
}
