//! Integration tests for the `rd state set` and `rd state clear` subcommands.

mod common;

// ── rd state set ──────────────────────────────────────────────────────────────

/// `rd state set working --harness claude --pane %42` should write a state
/// file under `XDG_DATA_HOME` and exit 0.
#[tokio::test]
async fn set_state_writes_state_file() {
    let env = common::TmuxEnv::new().await;

    let output = env
        .rd()
        .args([
            "state",
            "set",
            "working",
            "--harness",
            "claude",
            "--pane",
            "%42",
        ])
        .output()
        .expect("failed to spawn rd state set");

    assert!(
        output.status.success(),
        "`rd state set` should exit 0; stderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );

    let state_path = env.state_file_path("%42");
    assert!(state_path.exists(), "state file should have been created");

    let contents = std::fs::read_to_string(&state_path).expect("read state file");
    assert!(
        contents.contains("working"),
        "state file should contain 'working'; got: {contents}"
    );
    assert!(
        contents.contains("claude"),
        "state file should contain harness 'claude'; got: {contents}"
    );
}

/// `rd state set idle --pane %10` without `--harness` should write `"unknown"`
/// as the harness field.
#[tokio::test]
async fn set_state_defaults_harness_to_unknown() {
    let env = common::TmuxEnv::new().await;

    let output = env
        .rd()
        .args(["state", "set", "idle", "--pane", "%10"])
        .output()
        .expect("failed to spawn rd state set");

    assert!(output.status.success());

    let contents = std::fs::read_to_string(env.state_file_path("%10")).expect("read state file");
    assert!(
        contents.contains(r#""unknown""#),
        "harness field should be 'unknown'; got: {contents}"
    );
}

/// `rd state set` with `$TMUX_PANE` unset and no `--pane` flag should exit 0
/// silently without creating any state file.
#[tokio::test]
async fn set_state_no_pane_exits_silently() {
    let env = common::TmuxEnv::new().await;

    let output = env
        .rd()
        .args(["state", "set", "idle"])
        .env_remove("TMUX_PANE")
        .output()
        .expect("failed to spawn rd state set");

    assert!(
        output.status.success(),
        "`rd state set` should exit 0 silently when TMUX_PANE is unset"
    );
    assert!(
        output.stdout.is_empty() && output.stderr.is_empty(),
        "should produce no output; stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    // No state files should have been written.
    let agents_dir = env.xdg_path.join("radr").join("agents");
    assert!(
        !agents_dir.exists() || std::fs::read_dir(&agents_dir).unwrap().count() == 0,
        "no state file should be written when TMUX_PANE is unset"
    );
}

/// `rd state set <unknown>` should exit non-zero with an error message.
#[tokio::test]
async fn set_state_unknown_state_exits_nonzero() {
    let env = common::TmuxEnv::new().await;

    let output = env
        .rd()
        .args(["state", "set", "flying", "--pane", "%1"])
        .output()
        .expect("failed to spawn rd state set");

    assert!(
        !output.status.success(),
        "`rd state set` should exit non-zero for unknown state"
    );
}

// ── rd state clear ────────────────────────────────────────────────────────────

/// `rd state clear --pane %42` should delete an existing state file and exit 0.
#[tokio::test]
async fn clear_state_removes_existing_state_file() {
    let env = common::TmuxEnv::new().await;

    // Write a state file first via state set.
    let write_out = env
        .rd()
        .args(["state", "set", "done", "--pane", "%42"])
        .output()
        .expect("spawn state set");
    assert!(write_out.status.success());
    assert!(env.state_file_path("%42").exists());

    // Now clear it.
    let output = env
        .rd()
        .args(["state", "clear", "--pane", "%42"])
        .output()
        .expect("failed to spawn rd state clear");

    assert!(
        output.status.success(),
        "`rd state clear` should exit 0; stderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );
    assert!(
        !env.state_file_path("%42").exists(),
        "state file should have been deleted by rd state clear"
    );
}

/// `rd state clear --pane %99` when no state file exists should exit 0 silently
/// (no-op).
#[tokio::test]
async fn clear_state_absent_file_exits_silently() {
    let env = common::TmuxEnv::new().await;

    let output = env
        .rd()
        .args(["state", "clear", "--pane", "%99"])
        .output()
        .expect("failed to spawn rd state clear");

    assert!(
        output.status.success(),
        "`rd state clear` should exit 0 when state file absent"
    );
}

/// `rd state clear` with `$TMUX_PANE` unset and no `--pane` flag exits 0
/// silently.
#[tokio::test]
async fn clear_state_no_pane_exits_silently() {
    let env = common::TmuxEnv::new().await;

    let output = env
        .rd()
        .args(["state", "clear"])
        .env_remove("TMUX_PANE")
        .output()
        .expect("failed to spawn rd state clear");

    assert!(
        output.status.success(),
        "`rd state clear` should exit 0 silently when TMUX_PANE is unset"
    );
}
