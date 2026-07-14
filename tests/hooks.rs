//! Integration tests for `dv hooks install`.

mod common;

/// `dv hooks install claude` should exit 1 with a message containing
/// "unknown harness 'claude'" and "no harnesses are currently supported".
#[tokio::test]
async fn hooks_install_unknown_harness_exits_one() {
    let env = common::TmuxEnv::new().await;

    let output = env
        .dv()
        .args(["hooks", "install", "claude"])
        .output()
        .expect("failed to spawn dv hooks install");

    assert_eq!(
        output.status.code(),
        Some(1),
        "`dv hooks install` should exit with code 1; got: {:?}",
        output.status,
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unknown harness 'claude'"),
        "stderr should contain \"unknown harness 'claude'\"; got:\n{stderr}",
    );
    assert!(
        stderr.contains("no harnesses are currently supported"),
        "stderr should contain \"no harnesses are currently supported\"; got:\n{stderr}",
    );
}

/// `dv hooks install <any-harness>` always exits 1.
#[tokio::test]
async fn hooks_install_any_harness_exits_one() {
    let env = common::TmuxEnv::new().await;

    for harness in &["pi", "aider", "my-custom-harness"] {
        let output = env
            .dv()
            .args(["hooks", "install", harness])
            .output()
            .unwrap_or_else(|_| panic!("spawn dv hooks install {harness}"));

        assert_eq!(
            output.status.code(),
            Some(1),
            "`dv hooks install {harness}` should exit 1; got: {:?}",
            output.status,
        );
    }
}
