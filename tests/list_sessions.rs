//! Integration tests for the `rd list` subcommand.
//!
//! All tests run `rd` through a [`common::TmuxEnv`], which backs each test
//! with an isolated Docker container running `alpine:latest` + `tmux`.  The
//! container's tmux server is accessible from the host via `RD_TMUX_SOCKET`.
//! State files are isolated via `XDG_DATA_HOME` pointing at a per-test dir.

mod common;

// ── No agent panes → exit 0, no output ───────────────────────────────────────

/// With a running tmux server that has no agent panes, `rd list` should exit
/// 0 and produce no output.
#[tokio::test]
async fn list_no_agent_panes_exits_zero_with_no_output() {
    let env = common::TmuxEnv::new().await;

    let output = env
        .rd()
        .arg("list")
        .output()
        .expect("failed to spawn rd list");

    assert!(
        output.status.success(),
        "`rd list` should exit 0 when no agent panes exist; got: {:?}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr),
    );
    assert!(
        output.stdout.is_empty(),
        "`rd list` should produce no output when no agent panes exist; got:\n{}",
        String::from_utf8_lossy(&output.stdout),
    );
}

// ── tmux binary missing → exit non-zero, "tmux not found" ────────────────────

/// When the `tmux` binary is absent from PATH, `rd list` should exit non-zero
/// and print a human-readable error containing "tmux not found".
///
/// We override `PATH` with an empty directory so the host `rd` binary runs
/// but cannot find `tmux` when it shells out.
#[tokio::test]
async fn list_tmux_not_found_exits_nonzero_with_message() {
    let env = common::TmuxEnv::new().await;

    // An empty temporary directory contains no executables, so `tmux` will
    // not be found on the overridden PATH.
    let empty_bin = tempfile::tempdir().expect("create empty temp dir");

    let output = env
        .rd()
        .arg("list")
        .env("PATH", empty_bin.path())
        .output()
        .expect("failed to spawn rd list");

    assert!(
        !output.status.success(),
        "`rd list` should exit non-zero when tmux is not on PATH; got: {:?}",
        output.status,
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("tmux not found"),
        "`rd list` stderr should contain \"tmux not found\" when tmux is absent; got:\n{stderr}",
    );
}

// ── No server running → exit 0, no output ────────────────────────────────────

/// Pointing `rd list --server` at a socket path that does not exist simulates
/// a tmux server that is not running.  The command should exit 0 and produce
/// no output.
#[tokio::test]
async fn list_no_server_running_exits_zero_with_no_output() {
    let env = common::TmuxEnv::new().await;

    // A path inside the env's workspace that is guaranteed not to exist as a
    // tmux socket.
    let missing_socket = env.workspace_path.join("no-server.sock");

    let output = env
        .rd()
        .args(["list", "--server"])
        .arg(&missing_socket)
        .output()
        .expect("failed to spawn rd list");

    assert!(
        output.status.success(),
        "`rd list` should exit 0 when no tmux server is running; got: {:?}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr),
    );
    assert!(
        output.stdout.is_empty(),
        "`rd list` should produce no output when no tmux server is running; got:\n{}",
        String::from_utf8_lossy(&output.stdout),
    );
}

// ── State column: pane has a valid state file → correct state shown ───────────

/// When a pane is running a known agent and has a matching state file, the
/// state column in `rd list` output should reflect the persisted state.
///
/// **macOS Docker Desktop skip**: Unix domain sockets cannot be connected to
/// from the macOS host when created inside a container via bind mount (VirtioFS
/// limitation).  `rd list` therefore sees 0 live panes and produces no output.
/// The equivalent logic is fully covered by the `format_panes_shows_state_from_map`
/// unit test.  This integration test passes correctly on Linux.
#[tokio::test]
#[ignore = "requires host → container Unix socket access (fails on macOS Docker Desktop)"]
async fn list_agent_pane_with_state_file_shows_state() {
    let env = common::TmuxEnv::new().await;

    // Create a fake "claude" pane in the container's tmux server.
    let pane_id = env.create_fake_agent_pane("claude").await;

    // Write a state file for that pane.
    env.write_state_file(&pane_id, "claude", "working");

    let output = env
        .rd()
        .arg("list")
        .output()
        .expect("failed to spawn rd list");

    assert!(
        output.status.success(),
        "`rd list` should exit 0; stderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("working"),
        "state column should contain 'working'; got:\n{stdout}",
    );
    assert!(
        stdout.contains("claude"),
        "agent column should contain 'claude'; got:\n{stdout}",
    );
}

// ── State column: pane has no state file → "unknown" shown ───────────────────

/// When a pane is running a known agent but has no state file, the state
/// column should display "unknown".
///
/// **macOS Docker Desktop skip**: see note in `list_agent_pane_with_state_file_shows_state`.
#[tokio::test]
#[ignore = "requires host → container Unix socket access (fails on macOS Docker Desktop)"]
async fn list_agent_pane_without_state_file_shows_unknown() {
    let env = common::TmuxEnv::new().await;

    // Create a fake "claude" pane — no state file written.
    let _pane_id = env.create_fake_agent_pane("claude").await;

    let output = env
        .rd()
        .arg("list")
        .output()
        .expect("failed to spawn rd list");

    assert!(
        output.status.success(),
        "`rd list` should exit 0; stderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("unknown"),
        "state column should contain 'unknown' when no state file exists; got:\n{stdout}",
    );
}

// ── Stale validation: orphaned state file deleted after rd list ───────────────

/// When `rd list` runs and a state file exists for a pane ID that is not in the
/// live tmux pane set, that state file is deleted.
#[tokio::test]
async fn list_orphaned_state_file_is_deleted() {
    let env = common::TmuxEnv::new().await;

    // Write a state file for a pane ID that will never exist in tmux.
    let phantom_pane = "%9999";
    env.write_state_file(phantom_pane, "claude", "idle");
    assert!(
        env.state_file_path(phantom_pane).exists(),
        "state file should exist before rd list"
    );

    // Run rd list — no agent panes will be found, but stale cleanup runs.
    let output = env
        .rd()
        .arg("list")
        .output()
        .expect("failed to spawn rd list");

    assert!(
        output.status.success(),
        "`rd list` should exit 0; stderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );

    assert!(
        !env.state_file_path(phantom_pane).exists(),
        "orphaned state file should have been deleted by rd list"
    );
}

// ── Stale validation: contradictory harness state file deleted ────────────────

/// When `rd list` runs and a state file exists whose pane_id matches a live
/// pane but the harness does not match that pane's current command, the state
/// file is deleted (per the agent-state spec and tasks).
#[tokio::test]
async fn list_contradictory_harness_state_file_is_deleted() {
    let env = common::TmuxEnv::new().await;

    // Create a fake "claude" pane; its current command will be "claude".
    let pane_id = env.create_fake_agent_pane("claude").await;

    // Write a state file for that pane claiming harness="pi" — contradictory.
    env.write_state_file(&pane_id, "pi", "idle");
    assert!(
        env.state_file_path(&pane_id).exists(),
        "state file should exist before rd list"
    );

    let output = env
        .rd()
        .arg("list")
        .output()
        .expect("failed to spawn rd list");

    assert!(
        output.status.success(),
        "`rd list` should exit 0; stderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );

    assert!(
        !env.state_file_path(&pane_id).exists(),
        "contradictory state file should have been deleted by rd list"
    );
}

// ── Stale validation: "unknown" harness is NOT deleted ────────────────────────

/// When a state file has harness `"unknown"`, the harness-match check is
/// skipped and the file is not deleted even if the pane's current command
/// differs.
///
/// **macOS Docker Desktop skip**: see note in `list_agent_pane_with_state_file_shows_state`.
/// When the socket is inaccessible `rd list` sees 0 live panes and deletes
/// ALL state files as orphaned — the harness-skip logic can't be exercised.
/// The logic is covered by the `validate_keeps_unknown_harness_even_if_command_differs`
/// unit test.
#[tokio::test]
#[ignore = "requires host → container Unix socket access (fails on macOS Docker Desktop)"]
async fn list_unknown_harness_state_file_not_deleted() {
    let env = common::TmuxEnv::new().await;

    // Create a fake "claude" pane; its current command will be "claude".
    let pane_id = env.create_fake_agent_pane("claude").await;

    // Write a state file with harness="unknown" — match check is skipped.
    env.write_state_file(&pane_id, "unknown", "idle");
    assert!(
        env.state_file_path(&pane_id).exists(),
        "state file should exist before rd list"
    );

    let output = env
        .rd()
        .arg("list")
        .output()
        .expect("failed to spawn rd list");

    assert!(
        output.status.success(),
        "`rd list` should exit 0; stderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );

    assert!(
        env.state_file_path(&pane_id).exists(),
        "state file with harness='unknown' should NOT be deleted by rd list"
    );
}
