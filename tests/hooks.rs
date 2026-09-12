//! Integration tests for `agentctl install`.

mod common;

use std::fs;

/// `agentctl install <unknown-harness>` exits 2 (Clap usage error).
/// Note: `claude` is now a valid harness; this test uses unrecognised names.
#[tokio::test]
async fn hooks_install_unknown_harness_exits_two() {
    let env = common::TmuxEnv::new().await;

    let output = env
        .agentctl()
        .args(["install", "not-a-real-harness"])
        .output()
        .expect("failed to spawn agentctl install");

    assert_eq!(
        output.status.code(),
        Some(2),
        "`agentctl install not-a-real-harness` should exit with Clap error code 2; got: {:?}",
        output.status,
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not-a-real-harness"),
        "stderr should mention the invalid value; got:\n{stderr}",
    );
}

/// `agentctl install <unknown-harness>` always exits 2 (Clap usage error).
#[tokio::test]
async fn hooks_install_unknown_harness_names_exit_two() {
    let env = common::TmuxEnv::new().await;

    for harness in &["aider", "my-custom-harness"] {
        let output = env
            .agentctl()
            .args(["install", harness])
            .output()
            .unwrap_or_else(|_| panic!("spawn agentctl install {harness}"));

        assert_eq!(
            output.status.code(),
            Some(2),
            "`agentctl install {harness}` should exit 2; got: {:?}",
            output.status,
        );
    }
}

// ── Pi integration tests (task 2.6 and 2.7) ───────────────────────────────────

/// `agentctl install pi` on a container with no `~/.pi/agent/extensions/`
/// directory: directory and `agentctl.ts` are created; file contains all four
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
        .join("agentctl.ts");

    // Confirm extension does not exist before install.
    assert!(
        !ext_path.exists(),
        "agentctl.ts should not exist before install"
    );

    let output = env
        .agentctl()
        .args(["install", "pi"])
        .env("HOME", &fake_home)
        .output()
        .expect("spawn agentctl install pi");

    assert!(
        output.status.success(),
        "`agentctl install pi` should exit 0; got: {:?}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr),
    );

    // Directory must have been created.
    assert!(
        ext_path.parent().unwrap().exists(),
        "~/.pi/agent/extensions/ directory should be created"
    );

    // File must exist.
    assert!(ext_path.exists(), "agentctl.ts should be created");

    // File must contain all four handlers.
    let content = fs::read_to_string(&ext_path).expect("read agentctl.ts");
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

/// `agentctl install pi` run twice: second run is a no-op; file is
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
        .join("agentctl.ts");

    // First install.
    let out1 = env
        .agentctl()
        .args(["install", "pi"])
        .env("HOME", &fake_home)
        .output()
        .expect("first agentctl install pi");
    assert!(out1.status.success(), "first install should succeed");

    let content_after_first = fs::read_to_string(&ext_path).expect("read after first install");
    let mtime_after_first = fs::metadata(&ext_path).unwrap().modified().unwrap();

    // Small sleep so that a write would produce a different mtime.
    std::thread::sleep(std::time::Duration::from_millis(20));

    // Second install.
    let out2 = env
        .agentctl()
        .args(["install", "pi"])
        .env("HOME", &fake_home)
        .output()
        .expect("second agentctl install pi");
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

fn claude_plugin_dir(fake_home: &std::path::Path) -> std::path::PathBuf {
    fake_home.join(".claude").join("skills").join("agentctl")
}

/// `agentctl install claude` on a container with no `~/.claude/skills/`: the plugin
/// directory is created and contains all eleven agentctl hook entries.
#[tokio::test]
async fn hooks_install_claude_creates_plugin_directory() {
    let env = common::TmuxEnv::new().await;

    // Use an isolated HOME so the install lands in a known location and never
    // touches the developer's real ~/.claude/skills/agentctl/.
    let fake_home = env.workspace_path.join("home_claude");
    fs::create_dir_all(&fake_home).expect("create fake home");

    let plugin_dir = claude_plugin_dir(&fake_home);
    let hooks_json_path = plugin_dir.join("hooks").join("hooks.json");
    let manifest_path = plugin_dir.join(".claude-plugin").join("plugin.json");

    // Confirm the plugin directory does not exist before install.
    assert!(
        !plugin_dir.exists(),
        "plugin directory should not exist before install"
    );

    let output = env
        .agentctl()
        .args(["install", "claude"])
        .env("HOME", &fake_home)
        .output()
        .expect("spawn agentctl install claude");

    assert!(
        output.status.success(),
        "`agentctl install claude` should exit 0; got: {:?}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr),
    );

    // Both files must exist.
    assert!(manifest_path.exists(), "plugin.json should be created");
    assert!(hooks_json_path.exists(), "hooks.json should be created");

    let manifest_content = fs::read_to_string(&manifest_path).expect("read plugin.json");
    let manifest: serde_json::Value = serde_json::from_str(&manifest_content).expect("valid JSON");
    assert_eq!(manifest["name"], "agentctl");

    // Parse and verify all seven distinct hook events are registered (eleven
    // entries total — PreToolUse, PostToolUse, and PostToolUseFailure are each
    // registered twice).
    let content = fs::read_to_string(&hooks_json_path).expect("read hooks.json");
    let parsed: serde_json::Value = serde_json::from_str(&content).expect("valid JSON");
    let hooks = &parsed["hooks"];

    for event in &[
        "SessionStart",
        "UserPromptSubmit",
        "Stop",
        "StopFailure",
        "SessionEnd",
    ] {
        let arr = hooks[event]
            .as_array()
            .unwrap_or_else(|| panic!("hooks.{event} should be an array"));
        assert_eq!(arr.len(), 1, "hooks.{event} should have exactly one entry");
    }

    assert!(
        hooks.get("Notification").is_none(),
        "hooks.json should not register a Notification handler"
    );

    for event in &["PreToolUse", "PostToolUse", "PostToolUseFailure"] {
        let arr = hooks[event]
            .as_array()
            .unwrap_or_else(|| panic!("hooks.{event} should be an array"));
        assert_eq!(
            arr.len(),
            2,
            "hooks.{event} should have exactly two entries"
        );
    }

    // stdout should mention the two written files.
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("plugin.json") && stdout.contains("hooks.json"),
        "stdout should mention the written plugin files; got:\n{stdout}"
    );
}

/// `agentctl install claude` run twice: second run deletes and recreates the plugin
/// directory; resulting files are byte-for-byte identical to after the first
/// run.
#[tokio::test]
async fn hooks_install_claude_reinstall_produces_identical_content() {
    let env = common::TmuxEnv::new().await;

    let fake_home = env.workspace_path.join("home_claude2");
    fs::create_dir_all(&fake_home).expect("create fake home");

    let plugin_dir = claude_plugin_dir(&fake_home);
    let hooks_json_path = plugin_dir.join("hooks").join("hooks.json");

    // First install.
    let out1 = env
        .agentctl()
        .args(["install", "claude"])
        .env("HOME", &fake_home)
        .output()
        .expect("first agentctl install claude");
    assert!(out1.status.success(), "first install should succeed");

    let content_after_first = fs::read_to_string(&hooks_json_path).expect("read after first");

    // Second install (re-install: deletes then rewrites the directory).
    let out2 = env
        .agentctl()
        .args(["install", "claude"])
        .env("HOME", &fake_home)
        .output()
        .expect("second agentctl install claude");
    assert!(out2.status.success(), "second install should succeed");

    let content_after_second = fs::read_to_string(&hooks_json_path).expect("read after second");

    assert_eq!(
        content_after_first, content_after_second,
        "hooks.json content should be byte-for-byte identical after re-install"
    );
}

/// `agentctl uninstall claude` removes the plugin directory entirely; running it
/// again is a no-op that still exits 0.
#[tokio::test]
async fn hooks_uninstall_claude_removes_plugin_directory() {
    let env = common::TmuxEnv::new().await;

    let fake_home = env.workspace_path.join("home_claude_uninstall");
    fs::create_dir_all(&fake_home).expect("create fake home");

    let plugin_dir = claude_plugin_dir(&fake_home);

    env.agentctl()
        .args(["install", "claude"])
        .env("HOME", &fake_home)
        .output()
        .expect("spawn agentctl install claude");
    assert!(
        plugin_dir.exists(),
        "plugin directory should exist after install"
    );

    let out1 = env
        .agentctl()
        .args(["uninstall", "claude"])
        .env("HOME", &fake_home)
        .output()
        .expect("spawn agentctl uninstall claude");
    assert!(out1.status.success(), "uninstall should exit 0");
    assert!(
        !plugin_dir.exists(),
        "plugin directory should be removed after uninstall"
    );

    // Second uninstall is a no-op.
    let out2 = env
        .agentctl()
        .args(["uninstall", "claude"])
        .env("HOME", &fake_home)
        .output()
        .expect("spawn agentctl uninstall claude again");
    assert!(
        out2.status.success(),
        "second uninstall should still exit 0"
    );
}

/// Simulating the generated `PreToolUse`/`PostToolUse` (unmatched, untyped)
/// hook commands for a non-interactive tool increments then decrements
/// `active_tools` in the pane's state file.
#[tokio::test]
async fn hooks_install_claude_tool_call_tracking_updates_active_tools() {
    let env = common::TmuxEnv::new().await;

    let fake_home = env.workspace_path.join("home_claude3");
    fs::create_dir_all(&fake_home).expect("create fake home");

    let settings_path = claude_plugin_dir(&fake_home)
        .join("hooks")
        .join("hooks.json");

    let install_out = env
        .agentctl()
        .args(["install", "claude"])
        .env("HOME", &fake_home)
        .output()
        .expect("spawn agentctl install claude");
    assert!(
        install_out.status.success(),
        "install claude should succeed"
    );

    let content = fs::read_to_string(&settings_path).expect("read hooks.json");
    let parsed: serde_json::Value = serde_json::from_str(&content).expect("valid JSON");

    // Locate the unmatched, untyped PreToolUse entry (no matcher, no --type).
    let pre_cmd = parsed["hooks"]["PreToolUse"]
        .as_array()
        .expect("PreToolUse should be an array")
        .iter()
        .find_map(|entry| {
            if entry.get("matcher").is_some() {
                return None;
            }
            let cmd = entry["hooks"][0].get("command")?.as_str()?;
            if cmd.contains("TOOL_CALL_START") && !cmd.contains("--type") {
                Some(cmd.to_owned())
            } else {
                None
            }
        })
        .expect("unmatched untyped PreToolUse command should be present");

    // Locate the unmatched, untyped PostToolUse entry (no matcher, no --type,
    // no "perm-" id).
    let post_cmd = parsed["hooks"]["PostToolUse"]
        .as_array()
        .expect("PostToolUse should be an array")
        .iter()
        .find_map(|entry| {
            if entry.get("matcher").is_some() {
                return None;
            }
            let cmd = entry["hooks"][0].get("command")?.as_str()?;
            if cmd.contains("TOOL_CALL_RESULT") && !cmd.contains("--type") && !cmd.contains("perm-")
            {
                Some(cmd.to_owned())
            } else {
                None
            }
        })
        .expect("unmatched untyped PostToolUse command should be present");

    let pane = "%88";
    let rd_bin_dir = std::path::Path::new(env!("CARGO_BIN_EXE_agentctl"))
        .parent()
        .expect("agentctl binary has a parent dir")
        .to_path_buf();
    let path_with_rd = format!(
        "{}:{}",
        rd_bin_dir.display(),
        std::env::var("PATH").unwrap_or_default()
    );

    // Simulate the PreToolUse hook firing for a non-interactive tool call.
    let run_hook = |cmd: &str| {
        std::process::Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .env("TMUX_PANE", pane)
            .env("XDG_DATA_HOME", &env.xdg_path)
            .env("PATH", &path_with_rd)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write as _;
                child
                    .stdin
                    .take()
                    .unwrap()
                    .write_all(br#"{"tool_use_id":"tu1","session_id":"s1"}"#)?;
                child.wait_with_output()
            })
            .expect("failed to run simulated hook command")
    };

    let pre_out = run_hook(&pre_cmd);
    assert!(
        pre_out.status.success(),
        "simulated PreToolUse hook should exit 0; stderr: {}",
        String::from_utf8_lossy(&pre_out.stderr)
    );

    let state_after_pre = fs::read_to_string(env.state_file_path(pane))
        .expect("state file should exist after PreToolUse hook");
    let state_after_pre: serde_json::Value =
        serde_json::from_str(&state_after_pre).expect("valid state JSON");
    assert_eq!(
        state_after_pre["active_tools"].as_u64(),
        Some(1),
        "active_tools should be incremented to 1 after PreToolUse"
    );

    let post_out = run_hook(&post_cmd);
    assert!(
        post_out.status.success(),
        "simulated PostToolUse hook should exit 0; stderr: {}",
        String::from_utf8_lossy(&post_out.stderr)
    );

    let state_after_post = fs::read_to_string(env.state_file_path(pane))
        .expect("state file should exist after PostToolUse hook");
    let state_after_post: serde_json::Value =
        serde_json::from_str(&state_after_post).expect("valid state JSON");
    assert_eq!(
        state_after_post["active_tools"].as_u64(),
        Some(0),
        "active_tools should be decremented back to 0 after PostToolUse"
    );
}
