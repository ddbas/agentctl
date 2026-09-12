//! `agentctl-harness` — the `Harness` trait contract for harness
//! integrations (e.g. `agentctl-harness-pi`, `agentctl-harness-claude`).
//!
//! Contains no harness-specific code and no filesystem/process I/O of its
//! own; it only declares types and the trait itself.

use std::fmt;
use std::path::Path;

use anyhow::Result;

/// The contract a harness integration must implement to support
/// `agentctl install <harness>` / `agentctl uninstall <harness>`.
pub trait Harness {
    /// The harness's canonical name (e.g. `"pi"`, `"claude"`), as accepted on
    /// the CLI.
    fn name(&self) -> &'static str;

    /// Install agentctl's integration for this harness, returning a
    /// human-readable summary of what was written.
    fn install(&self) -> Result<InstallSummary>;

    /// Uninstall agentctl's integration for this harness, returning a
    /// human-readable summary of what was removed. Idempotent: uninstalling
    /// a harness that was never installed (or already uninstalled) succeeds
    /// as a no-op.
    fn uninstall(&self) -> Result<UninstallSummary>;
}

/// Human-readable summary text produced by [`Harness::install`].
#[derive(Debug, Clone)]
pub struct InstallSummary(pub String);

impl fmt::Display for InstallSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Human-readable summary text produced by [`Harness::uninstall`].
#[derive(Debug, Clone)]
pub struct UninstallSummary(pub String);

impl fmt::Display for UninstallSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Replace the home directory prefix with `~` for human-readable output.
///
/// Shared by every harness's summary generation, and by `agentctl-core`'s
/// `info` use case.
pub fn tilde_collapse(path: &Path) -> String {
    if let Some(home) = dirs::home_dir()
        && let Ok(rel) = path.strip_prefix(&home)
    {
        return format!("~/{}", rel.display());
    }
    path.display().to_string()
}
