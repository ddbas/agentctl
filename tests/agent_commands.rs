//! Integration tests for the `agentctl agent event` and `agentctl agent clear` subcommands.

mod common;

// ── agentctl agent event ────────────────────────────────────────────────────────────

/// `agentctl agent event RUN_STARTED --id r1 --harness claude --pane %42` should
/// write a state file under `XDG_DATA_HOME` and exit 0.
#[tokio::test]
async fn run_started_writes_state_file() {
    let env = common::TmuxEnv::new().await;

    // A real harness always dispatches SESSION_STARTED before any RUN_STARTED;
    // mirror that here so the FSM starts from `idle` rather than `unknown`.
    let session_output = env
        .agentctl()
        .args([
            "agent",
            "event",
            "SESSION_STARTED",
            "--id",
            "s1",
            "--harness",
            "claude",
            "--pane",
            "%42",
        ])
        .output()
        .expect("failed to spawn agentctl agent event SESSION_STARTED");
    assert!(session_output.status.success());

    let output = env
        .agentctl()
        .args([
            "agent",
            "event",
            "RUN_STARTED",
            "--id",
            "r1",
            "--harness",
            "claude",
            "--pane",
            "%42",
        ])
        .output()
        .expect("failed to spawn agentctl agent event");

    assert!(
        output.status.success(),
        "`agentctl agent event RUN_STARTED` should exit 0; stderr: {}",
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

    let events_path = env.events_file_path("%42");
    assert!(
        events_path.exists(),
        "events.jsonl should have been created"
    );
    let events_contents = std::fs::read_to_string(&events_path).expect("read events file");
    assert!(events_contents.contains("RUN_STARTED"));
}

/// `agentctl agent event SESSION_STARTED --id s1 --pane %10` without `--harness`
/// should write `"unknown"` as the harness field, and status `idle`.
#[tokio::test]
async fn session_started_defaults_harness_to_unknown() {
    let env = common::TmuxEnv::new().await;

    let output = env
        .agentctl()
        .args([
            "agent",
            "event",
            "SESSION_STARTED",
            "--id",
            "s1",
            "--pane",
            "%10",
        ])
        .output()
        .expect("failed to spawn agentctl agent event");

    assert!(output.status.success());

    let contents = std::fs::read_to_string(env.state_file_path("%10")).expect("read state file");
    assert!(
        contents.contains(r#""unknown""#),
        "harness field should be 'unknown'; got: {contents}"
    );
    assert!(
        contents.contains("idle"),
        "status field should be 'idle'; got: {contents}"
    );
}

/// `agentctl agent event` with `$TMUX_PANE` unset and no `--pane` flag should exit 0
/// silently without creating any state file.
#[tokio::test]
async fn no_pane_exits_silently() {
    let env = common::TmuxEnv::new().await;

    let output = env
        .agentctl()
        .args(["agent", "event", "SESSION_STARTED", "--id", "s1"])
        .env_remove("TMUX_PANE")
        .output()
        .expect("failed to spawn agentctl agent event");

    assert!(
        output.status.success(),
        "`agentctl agent event` should exit 0 silently when TMUX_PANE is unset"
    );
    assert!(
        output.stdout.is_empty() && output.stderr.is_empty(),
        "should produce no output; stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    // No state files should have been written.
    let agents_dir = env.xdg_path.join("agentctl").join("agents");
    assert!(
        !agents_dir.exists() || std::fs::read_dir(&agents_dir).unwrap().count() == 0,
        "no state file should be written when TMUX_PANE is unset"
    );
}

/// `agentctl agent event <unknown-name>` should exit non-zero with an error message.
#[tokio::test]
async fn unknown_event_name_exits_nonzero() {
    let env = common::TmuxEnv::new().await;

    let output = env
        .agentctl()
        .args(["agent", "event", "FLYING", "--id", "x", "--pane", "%1"])
        .output()
        .expect("failed to spawn agentctl agent event");

    assert!(
        !output.status.success(),
        "`agentctl agent event` should exit non-zero for unknown event name"
    );
}

/// `agentctl agent event RUN_STARTED --id r1 --type custom` should exit non-zero
/// since `RUN_STARTED` does not accept `--type`.
#[tokio::test]
async fn type_on_run_started_exits_nonzero() {
    let env = common::TmuxEnv::new().await;

    let output = env
        .agentctl()
        .args([
            "agent",
            "event",
            "RUN_STARTED",
            "--id",
            "r1",
            "--type",
            "custom",
            "--pane",
            "%1",
        ])
        .output()
        .expect("failed to spawn agentctl agent event");

    assert!(!output.status.success(), "RUN_STARTED should reject --type");
}

/// `agentctl agent event` without `--id` should exit non-zero and write nothing.
#[tokio::test]
async fn missing_id_exits_nonzero_and_writes_nothing() {
    let env = common::TmuxEnv::new().await;

    let output = env
        .agentctl()
        .args(["agent", "event", "RUN_STARTED", "--pane", "%77"])
        .output()
        .expect("failed to spawn agentctl agent event");

    assert!(!output.status.success(), "missing --id should be rejected");
    assert!(
        !env.state_file_path("%77").exists(),
        "no state file should be written when --id is missing"
    );
}

/// The full session/run/blocked/run lifecycle produces the expected statuses.
#[tokio::test]
async fn full_lifecycle_transitions_through_expected_statuses() {
    let env = common::TmuxEnv::new().await;
    let pane = "%55";

    let dispatch = |args: Vec<&str>| {
        let mut full = vec!["agent", "event"];
        full.extend(args);
        full.push("--pane");
        full.push(pane);
        env.agentctl()
            .args(full)
            .output()
            .expect("spawn agentctl agent event")
    };

    assert!(
        dispatch(vec!["SESSION_STARTED", "--id", "s1"])
            .status
            .success()
    );
    assert!(
        std::fs::read_to_string(env.state_file_path(pane))
            .unwrap()
            .contains("idle")
    );

    assert!(dispatch(vec!["RUN_STARTED", "--id", "r1"]).status.success());
    assert!(
        std::fs::read_to_string(env.state_file_path(pane))
            .unwrap()
            .contains("working")
    );

    assert!(
        dispatch(vec![
            "TOOL_CALL_START",
            "--id",
            "t1",
            "--type",
            "elicitation"
        ])
        .status
        .success()
    );
    assert!(
        std::fs::read_to_string(env.state_file_path(pane))
            .unwrap()
            .contains("blocked")
    );

    assert!(
        dispatch(vec![
            "TOOL_CALL_RESULT",
            "--id",
            "t1",
            "--type",
            "elicitation"
        ])
        .status
        .success()
    );
    assert!(
        std::fs::read_to_string(env.state_file_path(pane))
            .unwrap()
            .contains("working")
    );

    assert!(
        dispatch(vec!["RUN_FINISHED", "--id", "r1"])
            .status
            .success()
    );
    assert!(
        std::fs::read_to_string(env.state_file_path(pane))
            .unwrap()
            .contains("done")
    );
}

// ── agentctl agent clear ────────────────────────────────────────────────────────────

/// `agentctl agent clear --pane %42` should delete an existing state file and exit 0.
#[tokio::test]
async fn clear_state_removes_existing_state_file() {
    let env = common::TmuxEnv::new().await;

    // Write a state file first via agent event.
    let write_out = env
        .agentctl()
        .args([
            "agent",
            "event",
            "RUN_FINISHED",
            "--id",
            "r1",
            "--pane",
            "%42",
        ])
        .output()
        .expect("spawn agent event");
    assert!(write_out.status.success());
    assert!(env.state_file_path("%42").exists());

    // Now clear it.
    let output = env
        .agentctl()
        .args(["agent", "clear", "--pane", "%42"])
        .output()
        .expect("failed to spawn agentctl agent clear");

    assert!(
        output.status.success(),
        "`agentctl agent clear` should exit 0; stderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );
    assert!(
        !env.state_file_path("%42").exists(),
        "state file should have been deleted by agentctl agent clear"
    );
    assert!(
        !env.events_file_path("%42").exists(),
        "events.jsonl should have been deleted alongside state.json"
    );
}

/// `agentctl agent clear --pane %99` when no state file exists should exit 0 silently
/// (no-op).
#[tokio::test]
async fn clear_state_absent_file_exits_silently() {
    let env = common::TmuxEnv::new().await;

    let output = env
        .agentctl()
        .args(["agent", "clear", "--pane", "%99"])
        .output()
        .expect("failed to spawn agentctl agent clear");

    assert!(
        output.status.success(),
        "`agentctl agent clear` should exit 0 when state file absent"
    );
}

/// `agentctl agent clear` with `$TMUX_PANE` unset and no `--pane` flag exits 0
/// silently.
#[tokio::test]
async fn clear_state_no_pane_exits_silently() {
    let env = common::TmuxEnv::new().await;

    let output = env
        .agentctl()
        .args(["agent", "clear"])
        .env_remove("TMUX_PANE")
        .output()
        .expect("failed to spawn agentctl agent clear");

    assert!(
        output.status.success(),
        "`agentctl agent clear` should exit 0 silently when TMUX_PANE is unset"
    );
}
