use std::process::Command;
use std::{env, path::PathBuf};

fn cli_bin_path() -> PathBuf {
    if let Ok(path) = env::var("CARGO_BIN_EXE_axiomsync") {
        return PathBuf::from(path);
    }
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(|path| path.parent())
        .expect("workspace root")
        .join("target")
        .join("debug")
        .join(if cfg!(windows) {
            "axiomsync.exe"
        } else {
            "axiomsync"
        })
}

#[test]
fn cli_help_exposes_renewal_commands() {
    let output = Command::new(cli_bin_path())
        .arg("--help")
        .output()
        .expect("help");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    for command in [
        "connector",
        "project",
        "derive",
        "search",
        "runbook",
        "mcp",
        "web",
    ] {
        assert!(stdout.contains(command), "{command}");
    }
}
