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
//! The container uses the `tmux-test:latest` local image (Alpine + tmux
//! pre-installed; build with `mise run //crates/agentctl:build:test-image`). A tmux server is
//! started inside the container on a Unix socket at `/workspace/tmux.sock`,
//! which lives on a bind-mounted host `TempDir`. The host `agentctl` binary is
//! directed at the container's socket via the `AGENTCTL_TMUX_SOCKET` env var.
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
//!     // Run agentctl on the host; it is directed at the container's tmux server.
//!     let output = env.agentctl().arg("help").output().expect("spawn agentctl");
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

/// Pre-built image with tmux installed (see `tests/Dockerfile`).
/// Build with `mise run //crates/agentctl:build:test-image`.
///
/// Using a pre-built image avoids running `apk add` at test time, which
/// requires container internet access and is unreliable on macOS Docker
/// Desktop (DNS/network issues cause it to hang indefinitely).
const TMUX_IMAGE: &str = "tmux-test";
const TMUX_TAG: &str = "latest";

/// A fully isolated tmux test environment backed by a Docker container.
///
/// - An `tmux-test` container is started (Alpine + tmux pre-installed;
///   build with `mise run //crates/agentctl:build:test-image`). The container's CMD starts a
///   tmux server on the shared workspace socket and prints `"tmux-ready"`
///   to stdout once the server is accepting connections.  testcontainers
///   blocks until that message appears before returning from `.start()`.
/// - The tmux server runs on a Unix socket at `/workspace/tmux.sock`.
///   `/workspace` is bind-mounted from a host `TempDir`, making the socket
///   accessible on the host at `workspace_path/tmux.sock`.
/// - The host `agentctl` binary is directed at this socket via `AGENTCTL_TMUX_SOCKET`.
///   `TMUX` and `TMUX_PANE` are cleared so `agentctl` does not think it is
///   running inside an existing tmux session.
/// - A per-test XDG data directory at `workspace_path/xdg` is used for all
///   state files.  `XDG_DATA_HOME` is set on every `agentctl()` invocation so state
///   operations are fully isolated from the developer's host environment.
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
    /// Isolated XDG data home for this test (`workspace_path/xdg`).
    /// All `agentctl` state files are written here; never touches the real user dir.
    pub xdg_path: PathBuf,
}

impl TmuxEnv {
    /// Start the container and wait for the tmux server to be ready.
    ///
    /// The container CMD installs nothing at startup (tmux is pre-installed in
    /// the image) and prints `"tmux-ready"` once the server is up.
    /// testcontainers waits for that message before returning.
    ///
    /// # Panics
    ///
    /// Panics if Docker is unavailable, the image is missing (run
    /// `mise run //crates/agentctl:build:test-image`), or the tmux server fails to start.
    pub async fn new() -> Self {
        // Canonicalise so Docker Desktop's bind-mount path resolution works
        // on macOS (/var/folders symlink -> /private/var/folders).
        let workspace_tmp = TempDir::new().expect("temp workspace dir");
        let workspace_path = workspace_tmp
            .path()
            .canonicalize()
            .expect("canonicalize workspace");

        // The CMD starts tmux on the workspace socket and prints "tmux-ready"
        // once the server is up.  testcontainers waits for that message before
        // returning, so by the time `new()` returns the socket is ready.
        let container = GenericImage::new(TMUX_IMAGE, TMUX_TAG)
            .with_entrypoint("sh")
            .with_wait_for(WaitFor::message_on_stdout("tmux-ready"))
            .with_cmd([
                "-c",
                "tmux -S /workspace/tmux.sock new-session -d -s _init \
                 && echo 'tmux-ready' \
                 && tail -f /dev/null",
            ])
            .with_mount(Mount::bind_mount(
                workspace_path
                    .to_str()
                    .expect("workspace path is valid utf-8"),
                "/workspace",
            ))
            .start()
            .await
            .expect(
                "failed to start tmux container — is Docker running? \
                 Is `tmux-test:latest` built? Run `mise run //crates/agentctl:build:test-image`.",
            );

        let xdg_path = workspace_path.join("xdg");
        std::fs::create_dir_all(&xdg_path).expect("create xdg dir");

        TmuxEnv {
            _container: container,
            socket_path: workspace_path.join("tmux.sock"),
            xdg_path,
            _workspace: workspace_tmp,
            workspace_path,
        }
    }

    /// Returns a `Command` builder for the `agentctl` binary with:
    /// - `AGENTCTL_TMUX_SOCKET` = path to the container's tmux socket
    /// - `XDG_DATA_HOME` = isolated per-test state directory
    /// - `TMUX` and `TMUX_PANE` removed (prevent session contamination)
    pub fn agentctl(&self) -> Command {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_agentctl"));
        cmd.env(
            "AGENTCTL_TMUX_SOCKET",
            self.socket_path
                .to_str()
                .expect("socket path is valid utf-8"),
        )
        .env(
            "XDG_DATA_HOME",
            self.xdg_path.to_str().expect("xdg path is valid utf-8"),
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

    /// Create a fake agent pane in the container tmux server and return its
    /// pane ID (e.g. `%3`).
    ///
    /// Copies `/bin/sleep` to `/tmp/<agent>` inside the container and opens a
    /// new tmux window running it so `#{pane_current_command}` reports the
    /// given agent name. The pane ID is written to the bind-mounted workspace
    /// directory so the host can read it without needing a local tmux.
    ///
    /// # Panics
    ///
    /// Panics if any setup step fails.
    pub async fn create_fake_agent_pane(&self, agent: &str) -> String {
        // 1. Copy /bin/sleep as /tmp/<agent> so the executable name is <agent>.
        self.exec(&[
            "sh",
            "-c",
            &format!("cp /bin/sleep /tmp/{agent} && chmod +x /tmp/{agent}"),
        ])
        .await;

        // 2. Open a new tmux window named after the agent (no command — opens
        //    the default shell so the window stays alive).
        self.exec(&[
            "tmux",
            "-S",
            "/workspace/tmux.sock",
            "new-window",
            "-n",
            agent,
        ])
        .await;

        // 3. Query the pane ID by looking up the window we just named.
        //    Write to the bind-mount so the host can read it.
        let pane_id_file = format!("pane_id_{agent}.txt");
        self.exec(&[
            "sh",
            "-c",
            &format!(
                "tmux -S /workspace/tmux.sock \
                 list-panes -t '_init:{agent}' \
                 -F '#{{pane_id}}' \
                 > /workspace/{pane_id_file}"
            ),
        ])
        .await;

        let host_pane_id_path = self.workspace_path.join(&pane_id_file);
        let pane_id = std::fs::read_to_string(&host_pane_id_path)
            .unwrap_or_else(|e| panic!("read {}: {e}", host_pane_id_path.display()))
            .trim()
            .to_owned();
        assert!(!pane_id.is_empty(), "got empty pane_id");

        // 4. Replace the shell with /tmp/<agent> via `exec` builtin.
        self.exec(&[
            "tmux",
            "-S",
            "/workspace/tmux.sock",
            "send-keys",
            "-t",
            &pane_id,
            &format!("exec /tmp/{agent} 9999"),
            "Enter",
        ])
        .await;

        // 5. Wait for the exec to take effect in the pane.
        tokio::time::sleep(std::time::Duration::from_millis(700)).await;

        pane_id
    }

    /// Write a state file for the given pane under this env's XDG directory.
    ///
    /// The state is a minimal JSON file: `{"pane_id": ..., "harness": ...,
    /// "status": ..., "updated_at": ...}`.  The file is written on the HOST at
    /// `xdg_path/agentctl/agents/<pane_id>/state.json`.
    pub fn write_state_file(&self, pane_id: &str, harness: &str, status: &str) {
        let dir = self.xdg_path.join("agentctl").join("agents").join(pane_id);
        std::fs::create_dir_all(&dir).expect("create agent dir");
        let json = format!(
            r#"{{"pane_id":"{pane_id}","harness":"{harness}","status":"{status}","updated_at":"2026-01-01T00:00:00Z"}}"#
        );
        std::fs::write(dir.join("state.json"), json).expect("write state file");
    }

    /// Return the path of the state file for the given pane under this env's
    /// XDG directory.
    pub fn state_file_path(&self, pane_id: &str) -> PathBuf {
        self.xdg_path
            .join("agentctl")
            .join("agents")
            .join(pane_id)
            .join("state.json")
    }

    /// Return the path of the events log file for the given pane under this
    /// env's XDG directory.
    pub fn events_file_path(&self, pane_id: &str) -> PathBuf {
        self.xdg_path
            .join("agentctl")
            .join("agents")
            .join(pane_id)
            .join("events.jsonl")
    }
}
