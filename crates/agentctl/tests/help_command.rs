//! Integration tests for the `agentctl help` command.
//!
//! All tests run `agentctl` via a [`common::TmuxEnv`] which starts an isolated
//! tmux server inside a Docker container. `TMUX` and `TMUX_PANE` are stripped
//! from every invocation so `agentctl` always runs as if outside a tmux session.
mod common;

// ── agentctl help ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn help_subcommand_exits_zero() {
    let env = common::TmuxEnv::new().await;
    let status = env
        .agentctl()
        .arg("help")
        .status()
        .expect("failed to spawn agentctl");
    assert!(
        status.success(),
        "`agentctl help` should exit 0, got {status}"
    );
}

#[tokio::test]
async fn help_subcommand_prints_program_name() {
    let env = common::TmuxEnv::new().await;
    let output = env
        .agentctl()
        .arg("help")
        .output()
        .expect("failed to spawn agentctl");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("agentctl"),
        "`agentctl help` stdout should contain the program name; got:\n{stdout}"
    );
}

#[tokio::test]
async fn help_subcommand_prints_about_text() {
    let env = common::TmuxEnv::new().await;
    let output = env
        .agentctl()
        .arg("help")
        .output()
        .expect("failed to spawn agentctl");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("agent multiplexer"),
        "`agentctl help` stdout should contain 'agent multiplexer'; got:\n{stdout}"
    );
}

// ── agentctl --help ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn flag_help_exits_zero() {
    let env = common::TmuxEnv::new().await;
    let status = env
        .agentctl()
        .arg("--help")
        .status()
        .expect("failed to spawn agentctl");
    assert!(
        status.success(),
        "`agentctl --help` should exit 0, got {status}"
    );
}

#[tokio::test]
async fn flag_help_prints_usage() {
    let env = common::TmuxEnv::new().await;
    let output = env
        .agentctl()
        .arg("--help")
        .output()
        .expect("failed to spawn agentctl");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.to_lowercase().contains("usage"),
        "`agentctl --help` stdout should contain 'usage'; got:\n{stdout}"
    );
}

// ── agentctl (no args) ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn no_args_exits_zero() {
    let env = common::TmuxEnv::new().await;
    let status = env.agentctl().status().expect("failed to spawn agentctl");
    assert!(
        status.success(),
        "`agentctl` with no arguments should exit 0, got {status}"
    );
}
