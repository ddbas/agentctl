//! Integration tests for the `dv help` command.
//!
//! All tests run `dv` via a [`common::TmuxEnv`] which starts an isolated
//! tmux server inside a Docker container. `TMUX` and `TMUX_PANE` are stripped
//! from every invocation so `dv` always runs as if outside a tmux session.
mod common;

// ── dv help ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn help_subcommand_exits_zero() {
    let env = common::TmuxEnv::new().await;
    let status = env.dv().arg("help").status().expect("failed to spawn dv");
    assert!(status.success(), "`dv help` should exit 0, got {status}");
}

#[tokio::test]
async fn help_subcommand_prints_program_name() {
    let env = common::TmuxEnv::new().await;
    let output = env.dv().arg("help").output().expect("failed to spawn dv");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("dv"),
        "`dv help` stdout should contain the program name; got:\n{stdout}"
    );
}

#[tokio::test]
async fn help_subcommand_prints_about_text() {
    let env = common::TmuxEnv::new().await;
    let output = env.dv().arg("help").output().expect("failed to spawn dv");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("drovr"),
        "`dv help` stdout should contain 'drovr'; got:\n{stdout}"
    );
}

// ── dv --help ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn flag_help_exits_zero() {
    let env = common::TmuxEnv::new().await;
    let status = env.dv().arg("--help").status().expect("failed to spawn dv");
    assert!(status.success(), "`dv --help` should exit 0, got {status}");
}

#[tokio::test]
async fn flag_help_prints_usage() {
    let env = common::TmuxEnv::new().await;
    let output = env.dv().arg("--help").output().expect("failed to spawn dv");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.to_lowercase().contains("usage"),
        "`dv --help` stdout should contain 'usage'; got:\n{stdout}"
    );
}

// ── dv (no args) ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn no_args_exits_zero() {
    let env = common::TmuxEnv::new().await;
    let status = env.dv().status().expect("failed to spawn dv");
    assert!(
        status.success(),
        "`dv` with no arguments should exit 0, got {status}"
    );
}
