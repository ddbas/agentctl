//! Integration tests for `dv hooks install`.

mod common;

use std::fs;

/// `dv hooks install claude` should exit 2 (Clap usage error) because
/// `claude` is not a recognised harness value.
#[tokio::test]
async fn hooks_install_unknown_harness_exits_two() {
    let env = common::TmuxEnv::new().await;

    let output = env
        .dv()
        .args(["hooks", "install", "claude"])
        .output()
        .expect("failed to spawn dv hooks install");

    assert_eq!(
        output.status.code(),
        Some(2),
        "`dv hooks install claude` should exit with Clap error code 2; got: {:?}",
        output.status,
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("claude"),
        "stderr should mention the invalid value 'claude'; got:\n{stderr}",
    );
}

/// `dv hooks install <unknown-harness>` always exits 2 (Clap usage error).
#[tokio::test]
async fn hooks_install_unknown_harness_names_exit_two() {
    let env = common::TmuxEnv::new().await;

    for harness in &["aider", "my-custom-harness"] {
        let output = env
            .dv()
            .args(["hooks", "install", harness])
            .output()
            .unwrap_or_else(|_| panic!("spawn dv hooks install {harness}"));

        assert_eq!(
            output.status.code(),
            Some(2),
            "`dv hooks install {harness}` should exit 2; got: {:?}",
            output.status,
        );
    }
}

// ── Pi integration tests (task 2.6 and 2.7) ───────────────────────────────────

/// `dv hooks install pi` on a container with no `~/.pi/agent/extensions/`
/// directory: directory and `drovr.ts` are created; file contains all four
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
        .join("drovr.ts");

    // Confirm extension does not exist before install.
    assert!(
        !ext_path.exists(),
        "drovr.ts should not exist before install"
    );

    let output = env
        .dv()
        .args(["hooks", "install", "pi"])
        .env("HOME", &fake_home)
        .output()
        .expect("spawn dv hooks install pi");

    assert!(
        output.status.success(),
        "`dv hooks install pi` should exit 0; got: {:?}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr),
    );

    // Directory must have been created.
    assert!(
        ext_path.parent().unwrap().exists(),
        "~/.pi/agent/extensions/ directory should be created"
    );

    // File must exist.
    assert!(ext_path.exists(), "drovr.ts should be created");

    // File must contain all four handlers.
    let content = fs::read_to_string(&ext_path).expect("read drovr.ts");
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

/// `dv hooks install pi` run twice: second run is a no-op; file is
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
        .join("drovr.ts");

    // First install.
    let out1 = env
        .dv()
        .args(["hooks", "install", "pi"])
        .env("HOME", &fake_home)
        .output()
        .expect("first dv hooks install pi");
    assert!(out1.status.success(), "first install should succeed");

    let content_after_first = fs::read_to_string(&ext_path).expect("read after first install");
    let mtime_after_first = fs::metadata(&ext_path).unwrap().modified().unwrap();

    // Small sleep so that a write would produce a different mtime.
    std::thread::sleep(std::time::Duration::from_millis(20));

    // Second install.
    let out2 = env
        .dv()
        .args(["hooks", "install", "pi"])
        .env("HOME", &fake_home)
        .output()
        .expect("second dv hooks install pi");
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
