//! `dv hooks` — hook registration placeholder.
//!
//! No harnesses are supported in this iteration. All inputs to
//! `dv hooks install <harness>` cause the command to exit 1 with a clear
//! error message. Concrete harness support is added by dependent changes.

use anyhow::{Result, bail};

/// Entry point for `dv hooks install <harness>`.
///
/// Exits 1 with an explanatory message for all inputs.
pub fn run_install(harness: &str) -> Result<()> {
    bail!(
        "unknown harness '{}'; no harnesses are currently supported",
        harness
    );
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_unknown_harness_returns_error() {
        let result = run_install("claude");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("unknown harness 'claude'"),
            "unexpected message: {msg}"
        );
        assert!(
            msg.contains("no harnesses are currently supported"),
            "unexpected message: {msg}"
        );
    }

    #[test]
    fn install_arbitrary_harness_returns_error() {
        let result = run_install("my-custom-harness");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("unknown harness 'my-custom-harness'"),
            "unexpected message: {msg}"
        );
    }
}
