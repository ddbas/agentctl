//! Integration tests for `rd install`.

mod common;

use std::fs;

/// `rd install <unknown-harness>` exits 2 (Clap usage error).
/// Note: `claude` is now a valid harness; this test uses unrecognised names.
#[tokio::test]
async fn hooks_install_unknown_harness_exits_two() {
    let env = common::TmuxEnv::new().await;

    let output = env
        .rd()
        .args(["install", "not-a-real-harness"])
        .output()
        .expect("failed to spawn rd install");

    assert_eq!(
        output.status.code(),
        Some(2),
        "`rd install not-a-real-harness` should exit with Clap error code 2; got: {:?}",
        output.status,
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not-a-real-harness"),
        "stderr should mention the invalid value; got:\n{stderr}",
    );
}

/// `rd install <unknown-harness>` always exits 2 (Clap usage error).
#[tokio::test]
async fn hooks_install_unknown_harness_names_exit_two() {
    let env = common::TmuxEnv::new().await;

    for harness in &["aider", "my-custom-harness"] {
        let output = env
            .rd()
            .args(["install", harness])
            .output()
            .unwrap_or_else(|_| panic!("spawn rd install {harness}"));

        assert_eq!(
            output.status.code(),
            Some(2),
            "`rd install {harness}` should exit 2; got: {:?}",
            output.status,
        );
    }
}

// ── Pi integration tests (task 2.6 and 2.7) ───────────────────────────────────

/// `rd install pi` on a container with no `~/.pi/agent/extensions/`
/// directory: directory and `radr.ts` are created; file contains all four
/// event handlers.
#[tokio::test]
async fn hooks_install_pi_creates_extension_file() {
    let env = common::TmuxEnv::new().await;

    // Point HOME to the isolated workspace so the extension lands in a known
    // location on the host and does not touch the developer's real home dir.
    let fake_home = env.workspace_path.join("home");
    fs::create_dir_all(&fake_home).expect("create fake home");

    let ext_path = fake_home
        .join(".pi")
        .join("agent")
        .join("extensions")
        .join("radr.ts");

    // Confirm extension does not exist before install.
    assert!(
        !ext_path.exists(),
        "radr.ts should not exist before install"
    );

    let output = env
        .rd()
        .args(["install", "pi"])
        .env("HOME", &fake_home)
        .output()
        .expect("spawn rd install pi");

    assert!(
        output.status.success(),
        "`rd install pi` should exit 0; got: {:?}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr),
    );

    // Directory must have been created.
    assert!(
        ext_path.parent().unwrap().exists(),
        "~/.pi/agent/extensions/ directory should be created"
    );

    // File must exist.
    assert!(ext_path.exists(), "radr.ts should be created");

    // File must contain all four handlers.
    let content = fs::read_to_string(&ext_path).expect("read radr.ts");
    assert!(
        content.contains(r#"pi.on("session_start""#),
        "missing session_start handler"
    );
    assert!(
        content.contains(r#"pi.on("before_agent_start""#),
        "missing before_agent_start handler"
    );
    assert!(
        content.contains(r#"pi.on("agent_settled""#),
        "missing agent_settled handler"
    );
    assert!(
        content.contains(r#"pi.on("session_shutdown""#),
        "missing session_shutdown handler"
    );

    // stdout must contain the `+` prefix.
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains('+'),
        "stdout should contain '+' for new file; got:\n{stdout}"
    );
}

/// `rd install pi` run twice: second run is a no-op; file is
/// byte-for-byte identical to after the first run.
#[tokio::test]
async fn hooks_install_pi_idempotent() {
    let env = common::TmuxEnv::new().await;

    let fake_home = env.workspace_path.join("home2");
    fs::create_dir_all(&fake_home).expect("create fake home");

    let ext_path = fake_home
        .join(".pi")
        .join("agent")
        .join("extensions")
        .join("radr.ts");

    // First install.
    let out1 = env
        .rd()
        .args(["install", "pi"])
        .env("HOME", &fake_home)
        .output()
        .expect("first rd install pi");
    assert!(out1.status.success(), "first install should succeed");

    let content_after_first = fs::read_to_string(&ext_path).expect("read after first install");
    let mtime_after_first = fs::metadata(&ext_path).unwrap().modified().unwrap();

    // Small sleep so that a write would produce a different mtime.
    std::thread::sleep(std::time::Duration::from_millis(20));

    // Second install.
    let out2 = env
        .rd()
        .args(["install", "pi"])
        .env("HOME", &fake_home)
        .output()
        .expect("second rd install pi");
    assert!(out2.status.success(), "second install should succeed");

    let content_after_second = fs::read_to_string(&ext_path).expect("read after second install");
    let mtime_after_second = fs::metadata(&ext_path).unwrap().modified().unwrap();

    assert_eq!(
        content_after_first, content_after_second,
        "file content should be byte-for-byte identical after idempotent re-run"
    );
    assert_eq!(
        mtime_after_first, mtime_after_second,
        "file should not be rewritten on idempotent re-run"
    );

    // stdout must contain `=` prefix on second run.
    let stdout2 = String::from_utf8_lossy(&out2.stdout);
    assert!(
        stdout2.contains('='),
        "stdout should contain '=' for idempotent re-run; got:\n{stdout2}"
    );
}

// ── Claude integration tests (task 2.6 and 2.7) ───────────────────────────────

/// `rd install claude` on a container with no `~/.claude/settings.json`:
/// the file is created and contains all nine radr hook entries.
#[tokio::test]
async fn hooks_install_claude_creates_settings_file() {
    let env = common::TmuxEnv::new().await;

    // Use an isolated HOME so the install lands in a known location and never
    // touches the developer's real ~/.claude/settings.json.
    let fake_home = env.workspace_path.join("home_claude");
    fs::create_dir_all(&fake_home).expect("create fake home");

    let settings_path = fake_home.join(".claude").join("settings.json");

    // Confirm the file does not exist before install.
    assert!(
        !settings_path.exists(),
        "settings.json should not exist before install"
    );

    let output = env
        .rd()
        .args(["install", "claude"])
        .env("HOME", &fake_home)
        .output()
        .expect("spawn rd install claude");

    assert!(
        output.status.success(),
        "`rd install claude` should exit 0; got: {:?}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr),
    );

    // File must exist.
    assert!(settings_path.exists(), "settings.json should be created");

    // Parse and verify all seven distinct hook events are registered (nine
    // entries total — PostToolUse is registered twice).
    let content = fs::read_to_string(&settings_path).expect("read settings.json");
    let parsed: serde_json::Value = serde_json::from_str(&content).expect("valid JSON");
    let hooks = &parsed["hooks"];

    for event in &[
        "SessionStart",
        "UserPromptSubmit",
        "PreToolUse",
        "Stop",
        "StopFailure",
        "Notification",
        "SessionEnd",
    ] {
        let arr = hooks[event]
            .as_array()
            .unwrap_or_else(|| panic!("hooks.{event} should be an array"));
        assert_eq!(arr.len(), 1, "hooks.{event} should have exactly one entry");
    }

    let post_tool_use = hooks["PostToolUse"]
        .as_array()
        .expect("hooks.PostToolUse should be an array");
    assert_eq!(
        post_tool_use.len(),
        2,
        "hooks.PostToolUse should have exactly two entries"
    );

    // stdout should contain '+' for new entries.
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains('+'),
        "stdout should contain '+' for new entries; got:\n{stdout}"
    );
}

/// `rd install claude` run twice: second run is a no-op; file content
/// is byte-for-byte identical to after the first run.
#[tokio::test]
async fn hooks_install_claude_idempotent() {
    let env = common::TmuxEnv::new().await;

    let fake_home = env.workspace_path.join("home_claude2");
    fs::create_dir_all(&fake_home).expect("create fake home");

    let settings_path = fake_home.join(".claude").join("settings.json");

    // First install.
    let out1 = env
        .rd()
        .args(["install", "claude"])
        .env("HOME", &fake_home)
        .output()
        .expect("first rd install claude");
    assert!(out1.status.success(), "first install should succeed");

    let content_after_first = fs::read_to_string(&settings_path).expect("read after first");
    let mtime_after_first = fs::metadata(&settings_path).unwrap().modified().unwrap();

    // Small sleep so that a write would produce a different mtime.
    std::thread::sleep(std::time::Duration::from_millis(20));

    // Second install.
    let out2 = env
        .rd()
        .args(["install", "claude"])
        .env("HOME", &fake_home)
        .output()
        .expect("second rd install claude");
    assert!(out2.status.success(), "second install should succeed");

    let content_after_second = fs::read_to_string(&settings_path).expect("read after second");
    let mtime_after_second = fs::metadata(&settings_path).unwrap().modified().unwrap();

    assert_eq!(
        content_after_first, content_after_second,
        "file content should be byte-for-byte identical after idempotent re-run"
    );
    assert_eq!(
        mtime_after_first, mtime_after_second,
        "file should not be rewritten on idempotent re-run"
    );

    // stdout should contain '=' on second run.
    let stdout2 = String::from_utf8_lossy(&out2.stdout);
    assert!(
        stdout2.contains('='),
        "stdout should contain '=' for idempotent re-run; got:\n{stdout2}"
    );
}
