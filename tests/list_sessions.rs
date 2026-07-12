//! Integration tests for the `dv list` subcommand.
//!
//! All tests run `dv` through a [`common::TmuxEnv`], which backs each test
//! with an isolated Docker container running `alpine:latest` + `tmux`.  The
//! container's tmux server is accessible from the host via `DV_TMUX_SOCKET`.

mod common;

// ── No agent panes → exit 0, no output ───────────────────────────────────────

/// With a running tmux server that has no agent panes, `dv list` should exit
/// 0 and produce no output.
#[tokio::test]
async fn list_no_agent_panes_exits_zero_with_no_output() {
    let env = common::TmuxEnv::new().await;

    let output = env
        .dv()
        .arg("list")
        .output()
        .expect("failed to spawn dv list");

    assert!(
        output.status.success(),
        "`dv list` should exit 0 when no agent panes exist; got: {:?}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr),
    );
    assert!(
        output.stdout.is_empty(),
        "`dv list` should produce no output when no agent panes exist; got:\n{}",
        String::from_utf8_lossy(&output.stdout),
    );
}

// ── tmux binary missing → exit non-zero, "tmux not found" ────────────────────

/// When the `tmux` binary is absent from PATH, `dv list` should exit non-zero
/// and print a human-readable error containing "tmux not found".
///
/// We override `PATH` with an empty directory so the host `dv` binary runs
/// but cannot find `tmux` when it shells out.
#[tokio::test]
async fn list_tmux_not_found_exits_nonzero_with_message() {
    let env = common::TmuxEnv::new().await;

    // An empty temporary directory contains no executables, so `tmux` will
    // not be found on the overridden PATH.
    let empty_bin = tempfile::tempdir().expect("create empty temp dir");

    let output = env
        .dv()
        .arg("list")
        .env("PATH", empty_bin.path())
        .output()
        .expect("failed to spawn dv list");

    assert!(
        !output.status.success(),
        "`dv list` should exit non-zero when tmux is not on PATH; got: {:?}",
        output.status,
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("tmux not found"),
        "`dv list` stderr should contain \"tmux not found\" when tmux is absent; got:\n{stderr}",
    );
}

// ── No server running → exit 0, no output ────────────────────────────────────

/// Pointing `dv list --server` at a socket path that does not exist simulates
/// a tmux server that is not running.  The command should exit 0 and produce
/// no output.
#[tokio::test]
async fn list_no_server_running_exits_zero_with_no_output() {
    let env = common::TmuxEnv::new().await;

    // A path inside the env's workspace that is guaranteed not to exist as a
    // tmux socket.
    let missing_socket = env.workspace_path.join("no-server.sock");

    let output = env
        .dv()
        .args(["list", "--server"])
        .arg(&missing_socket)
        .output()
        .expect("failed to spawn dv list");

    assert!(
        output.status.success(),
        "`dv list` should exit 0 when no tmux server is running; got: {:?}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr),
    );
    assert!(
        output.stdout.is_empty(),
        "`dv list` should produce no output when no tmux server is running; got:\n{}",
        String::from_utf8_lossy(&output.stdout),
    );
}
