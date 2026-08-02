//! Integration tests for the `rd help` command.
//!
//! All tests run `rd` via a [`common::TmuxEnv`] which starts an isolated
//! tmux server inside a Docker container. `TMUX` and `TMUX_PANE` are stripped
//! from every invocation so `rd` always runs as if outside a tmux session.
mod common;

// ── rd help ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn help_subcommand_exits_zero() {
    let env = common::TmuxEnv::new().await;
    let status = env.rd().arg("help").status().expect("failed to spawn rd");
    assert!(status.success(), "`rd help` should exit 0, got {status}");
}

#[tokio::test]
async fn help_subcommand_prints_program_name() {
    let env = common::TmuxEnv::new().await;
    let output = env.rd().arg("help").output().expect("failed to spawn rd");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("rd"),
        "`rd help` stdout should contain the program name; got:\n{stdout}"
    );
}

#[tokio::test]
async fn help_subcommand_prints_about_text() {
    let env = common::TmuxEnv::new().await;
    let output = env.rd().arg("help").output().expect("failed to spawn rd");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("agent multiplexer"),
        "`rd help` stdout should contain 'agent multiplexer'; got:\n{stdout}"
    );
}

// ── rd --help ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn flag_help_exits_zero() {
    let env = common::TmuxEnv::new().await;
    let status = env.rd().arg("--help").status().expect("failed to spawn rd");
    assert!(status.success(), "`rd --help` should exit 0, got {status}");
}

#[tokio::test]
async fn flag_help_prints_usage() {
    let env = common::TmuxEnv::new().await;
    let output = env.rd().arg("--help").output().expect("failed to spawn rd");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.to_lowercase().contains("usage"),
        "`rd --help` stdout should contain 'usage'; got:\n{stdout}"
    );
}

// ── rd (no args) ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn no_args_exits_zero() {
    let env = common::TmuxEnv::new().await;
    let status = env.rd().status().expect("failed to spawn rd");
    assert!(
        status.success(),
        "`rd` with no arguments should exit 0, got {status}"
    );
}
