//! Shared test helpers for integration and E2E tests.
//!
//! # Isolation requirement
//!
//! Every integration test that interacts with tmux MUST use [`TmuxEnv`] rather
//! than spawning `tmux` directly on the host. `TmuxEnv` backs each test with a
//! Docker container (via testcontainers) that provides a fully isolated tmux
//! server: no shared sessions, no interference with the developer's running
//! tmux, and automatic cleanup on drop.
//!
//! The container runs `alpine:latest` with `tmux` installed. A tmux server is
//! started inside the container on a Unix socket at `/workspace/tmux.sock`,
//! which lives on a bind-mounted host `TempDir`. The host `dv` binary is
//! directed at the container's socket via the `DV_TMUX_SOCKET` env var.
//!
//! # Usage
//!
//! ```rust,ignore
//! mod common;
//!
//! #[tokio::test]
//! async fn my_test() {
//!     let env = common::TmuxEnv::new().await;
//!
//!     // Run dv on the host; it is directed at the container's tmux server.
//!     let output = env.dv().arg("help").output().expect("spawn dv");
//!     assert!(output.status.success());
//! } // container + TempDir cleaned up on drop
//! ```
//!
//! # Extending
//!
//! Add new helpers in this file for each service type your tests need.
//! Keep helpers focused: one function/struct per concern.

use std::path::PathBuf;
use std::process::Command;

use tempfile::TempDir;
use testcontainers::{
    ContainerAsync, GenericImage, ImageExt,
    core::{ExecCommand, Mount, WaitFor},
    runners::AsyncRunner,
};

// ── hello-world canary ────────────────────────────────────────────────────────

/// Start a lightweight `hello-world` container and return the handle.
///
/// This is the canary test for the Docker/testcontainers infrastructure.
/// If it passes, Docker is reachable and the async runner works correctly.
///
/// # Panics
///
/// Panics if Docker is unavailable or the image cannot be pulled.
pub async fn start_generic_container() -> ContainerAsync<GenericImage> {
    GenericImage::new("hello-world", "latest")
        .with_wait_for(WaitFor::message_on_stdout("Hello from Docker!"))
        .start()
        .await
        .expect("failed to start hello-world container — is Docker running?")
}

// ── TmuxEnv ───────────────────────────────────────────────────────────────────

const ALPINE_IMAGE: &str = "alpine";
const ALPINE_TAG: &str = "latest";

/// A fully isolated tmux test environment backed by a Docker container.
///
/// - An `alpine` container is started with `tmux` installed inside it.
///   The container's entrypoint is overridden to `sh -c "tail -f /dev/null"`
///   so that `exec` commands can be issued at any point.
/// - A tmux server is started inside the container on a Unix socket at
///   `/workspace/tmux.sock`.  The `/workspace` directory is bind-mounted
///   from a host `TempDir`, making the socket accessible on the host at
///   `workspace_path/tmux.sock`.
/// - The host `dv` binary is directed at this socket via `DV_TMUX_SOCKET`.
///   `TMUX` and `TMUX_PANE` are cleared so `dv` does not think it is
///   running inside an existing tmux session.
pub struct TmuxEnv {
    /// Keeps the container alive until the test ends.
    _container: ContainerAsync<GenericImage>,
    /// Keeps the workspace temp dir alive until the test ends.
    _workspace: TempDir,
    /// Canonicalised host path of the bind-mounted workspace.
    /// Matches the container's `/workspace`.
    pub workspace_path: PathBuf,
    /// Host path of the tmux server socket (`workspace_path/tmux.sock`).
    pub socket_path: PathBuf,
}

impl TmuxEnv {
    /// Start the container, install tmux, and start an isolated tmux server.
    ///
    /// # Panics
    ///
    /// Panics if Docker is unavailable, the image cannot be pulled, `apk`
    /// fails, or the tmux server cannot be started.
    pub async fn new() -> Self {
        // Canonicalise so Docker Desktop's bind-mount path resolution works
        // on macOS (/var/folders symlink -> /private/var/folders).
        let workspace_tmp = TempDir::new().expect("temp workspace dir");
        let workspace_path = workspace_tmp
            .path()
            .canonicalize()
            .expect("canonicalize workspace");

        let container = GenericImage::new(ALPINE_IMAGE, ALPINE_TAG)
            .with_entrypoint("sh")
            .with_wait_for(WaitFor::Nothing)
            .with_cmd(["-c", "tail -f /dev/null"])
            .with_mount(Mount::bind_mount(
                workspace_path
                    .to_str()
                    .expect("workspace path is valid utf-8"),
                "/workspace",
            ))
            .start()
            .await
            .expect(
                "failed to start alpine container — is Docker running? \
                 Does Docker Desktop have file sharing enabled for this path?",
            );

        let env = TmuxEnv {
            _container: container,
            socket_path: workspace_path.join("tmux.sock"),
            _workspace: workspace_tmp,
            workspace_path,
        };

        // Install tmux inside the container.
        env.exec(&["apk", "add", "--no-cache", "tmux"]).await;

        // Start the tmux server on the shared socket path.
        // `-d` starts the server in the background without attaching.
        env.exec(&["tmux", "-S", "/workspace/tmux.sock", "start-server"])
            .await;

        env
    }

    /// Returns a `Command` builder for the `dv` binary with:
    /// - `DV_TMUX_SOCKET` = path to the container's tmux socket
    /// - `TMUX` and `TMUX_PANE` removed (prevent session contamination)
    pub fn dv(&self) -> Command {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_dv"));
        cmd.env(
            "DV_TMUX_SOCKET",
            self.socket_path
                .to_str()
                .expect("socket path is valid utf-8"),
        )
        .env_remove("TMUX")
        .env_remove("TMUX_PANE");
        cmd
    }

    /// Run an arbitrary command inside the container.
    ///
    /// # Panics
    ///
    /// Panics if the command exits with a non-zero status.
    pub async fn exec(&self, args: &[&str]) {
        use tokio::io::AsyncReadExt as _;

        let cmd: Vec<String> = args.iter().map(|s| s.to_string()).collect();

        let exec_cmd = ExecCommand::new(cmd.clone())
            .with_cmd_ready_condition(testcontainers::core::CmdWaitFor::exit_code(0));

        let mut result = self
            ._container
            .exec(exec_cmd)
            .await
            .unwrap_or_else(|e| panic!("container exec failed for {cmd:?}: {e}"));

        let mut stderr_bytes = Vec::new();
        result.stderr().read_to_end(&mut stderr_bytes).await.ok();

        let exit = result
            .exit_code()
            .await
            .unwrap_or_else(|e| panic!("get exit code for {cmd:?}: {e}"))
            .expect("exit code should be set after CmdWaitFor::exit_code");

        assert_eq!(
            exit,
            0,
            "command {args:?} exited with status {exit}:\n{}",
            String::from_utf8_lossy(&stderr_bytes)
        );
    }
}
