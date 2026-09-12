//! `agentctl-harness-claude` — the Claude Code harness integration.
//!
//! Writes (or removes) a self-contained Claude Code plugin at
//! `~/.claude/skills/agentctl/`, containing `.claude-plugin/plugin.json` and
//! `hooks/hooks.json`. Install is remove-then-write (no merge/diff against
//! an existing installation); this crate never reads or writes
//! `~/.claude/settings.json`.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde_json::json;

use agentctl_harness::{Harness, InstallSummary, UninstallSummary, tilde_collapse};

/// `Harness` implementation for Claude Code.
pub struct ClaudeHarness;

impl Harness for ClaudeHarness {
    fn name(&self) -> &'static str {
        "claude"
    }

    fn install(&self) -> Result<InstallSummary> {
        let dir = default_claude_plugin_dir()?;
        install_claude_to(&dir)
    }

    fn uninstall(&self) -> Result<UninstallSummary> {
        let dir = default_claude_plugin_dir()?;
        uninstall_claude_to(&dir)
    }
}

// ── Hook definitions ──────────────────────────────────────────────────────────

/// A single agentctl hook entry, registered under `hooks/hooks.json` in the
/// installed plugin.
struct ClaudeHookDef {
    event: &'static str,
    matcher: Option<&'static str>,
    command: &'static str,
    async_: bool,
}

/// The eleven agentctl hook entries to register in the plugin's `hooks.json`.
///
/// Every command reads the hook's JSON payload from stdin via `jq` and
/// forwards the extracted field as `--id` to `agentctl agent event`/`agentctl agent
/// clear`, per `specs/claude-state-hooks/spec.md`. `PreToolUse`, `PostToolUse`,
/// and `PostToolUseFailure` each appear twice: once unmatched (every tool,
/// untyped, drives `active_tools` via plain
/// `TOOL_CALL_START`/`TOOL_CALL_RESULT`) and once matched (interactive tools
/// only, `--type elicitation`). `PostToolUse` and `PostToolUseFailure` share
/// the same `tool_use_id`-keyed `TOOL_CALL_RESULT` command for each matcher,
/// since Claude Code fires exactly one of the two per tool call (success vs.
/// failure) and either closes out the `TOOL_CALL_START` opened by
/// `PreToolUse`.
const AGENTCTL_CLAUDE_HOOKS: &[ClaudeHookDef] = &[
    ClaudeHookDef {
        event: "SessionStart",
        matcher: Some("startup|resume|clear"),
        command: r#"id=$(cat | jq -r '.session_id'); agentctl agent event SESSION_STARTED --id "$id" --harness claude"#,
        async_: false,
    },
    ClaudeHookDef {
        event: "UserPromptSubmit",
        matcher: None,
        command: r#"id=$(cat | jq -r '.session_id'); agentctl agent event RUN_STARTED --id "$id" --harness claude"#,
        async_: true,
    },
    ClaudeHookDef {
        event: "PreToolUse",
        matcher: None,
        command: r#"id=$(cat | jq -r '.tool_use_id'); agentctl agent event TOOL_CALL_START --id "$id" --harness claude"#,
        async_: true,
    },
    ClaudeHookDef {
        event: "PreToolUse",
        matcher: Some("AskUserQuestion|ExitPlanMode"),
        command: r#"id=$(cat | jq -r '.tool_use_id'); agentctl agent event TOOL_CALL_START --id "$id" --type elicitation --harness claude"#,
        async_: true,
    },
    ClaudeHookDef {
        event: "PostToolUse",
        matcher: None,
        command: r#"id=$(cat | jq -r '.tool_use_id'); agentctl agent event TOOL_CALL_RESULT --id "$id" --harness claude"#,
        async_: true,
    },
    ClaudeHookDef {
        event: "PostToolUse",
        matcher: Some("AskUserQuestion|ExitPlanMode"),
        command: r#"id=$(cat | jq -r '.tool_use_id'); agentctl agent event TOOL_CALL_RESULT --id "$id" --type elicitation --harness claude"#,
        async_: true,
    },
    ClaudeHookDef {
        event: "PostToolUseFailure",
        matcher: None,
        command: r#"id=$(cat | jq -r '.tool_use_id'); agentctl agent event TOOL_CALL_RESULT --id "$id" --harness claude"#,
        async_: true,
    },
    ClaudeHookDef {
        event: "PostToolUseFailure",
        matcher: Some("AskUserQuestion|ExitPlanMode"),
        command: r#"id=$(cat | jq -r '.tool_use_id'); agentctl agent event TOOL_CALL_RESULT --id "$id" --type elicitation --harness claude"#,
        async_: true,
    },
    ClaudeHookDef {
        event: "Stop",
        matcher: None,
        command: r#"payload=$(cat); id=$(echo "$payload" | jq -r '.session_id'); n=$(echo "$payload" | jq -r '(.background_tasks // []) | length'); agentctl agent event RUN_FINISHED --id "$id" --active-tools "$n" --harness claude"#,
        async_: true,
    },
    ClaudeHookDef {
        event: "StopFailure",
        matcher: None,
        command: r#"payload=$(cat); id=$(echo "$payload" | jq -r '.session_id'); n=$(echo "$payload" | jq -r '(.background_tasks // []) | length'); agentctl agent event RUN_FINISHED --id "$id" --active-tools "$n" --harness claude"#,
        async_: true,
    },
    ClaudeHookDef {
        event: "SessionEnd",
        matcher: None,
        command: "agentctl agent clear",
        async_: false,
    },
];

/// Name of the agentctl plugin directory installed under `~/.claude/skills/`.
const AGENTCTL_CLAUDE_PLUGIN_NAME: &str = "agentctl";

/// Contents of the plugin manifest written to
/// `~/.claude/skills/agentctl/.claude-plugin/plugin.json`.
///
/// Minimal manifest: no `version`, `author`, or other optional fields, none
/// of which anything in `agentctl` currently requires. `hooks/hooks.json` is the
/// default hooks path Claude Code looks for, so no explicit `hooks` field is
/// needed here.
fn claude_plugin_manifest() -> serde_json::Value {
    json!({
        "name": AGENTCTL_CLAUDE_PLUGIN_NAME,
        "description": "agentctl agent-state lifecycle hooks for Claude Code",
    })
}

/// Resolve the default agentctl plugin directory:
/// `~/.claude/skills/agentctl`.
///
/// Exposed as a separate function so unit tests can call `install_claude_to`/
/// `uninstall_claude_to` with a custom path, while production code uses this
/// default — mirroring `agentctl-harness-pi`'s pattern.
fn default_claude_plugin_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("could not determine home directory")?;
    Ok(home
        .join(".claude")
        .join("skills")
        .join(AGENTCTL_CLAUDE_PLUGIN_NAME))
}

/// Path to the plugin manifest inside a plugin directory.
fn claude_plugin_manifest_path(plugin_dir: &Path) -> PathBuf {
    plugin_dir.join(".claude-plugin").join("plugin.json")
}

/// Path to the hooks configuration inside a plugin directory.
fn claude_hooks_json_path(plugin_dir: &Path) -> PathBuf {
    plugin_dir.join("hooks").join("hooks.json")
}

/// Build a single JSON hook entry object for the given hook definition.
fn build_claude_hook_entry(hook: &ClaudeHookDef) -> serde_json::Value {
    let mut cmd_obj = serde_json::Map::new();
    cmd_obj.insert("type".to_string(), json!("command"));
    cmd_obj.insert("command".to_string(), json!(hook.command));
    if hook.async_ {
        cmd_obj.insert("async".to_string(), json!(true));
    }

    let mut entry = serde_json::Map::new();
    if let Some(matcher) = hook.matcher {
        entry.insert("matcher".to_string(), json!(matcher));
    }
    entry.insert(
        "hooks".to_string(),
        json!([serde_json::Value::Object(cmd_obj)]),
    );

    serde_json::Value::Object(entry)
}

/// Build the full `hooks/hooks.json` document: the wrapper shape
/// `{ "hooks": { <event>: [ <entry>, ... ] } }`, with every `ClaudeHookDef` in
/// `AGENTCTL_CLAUDE_HOOKS` contributing one entry to its event's array (so events
/// registered by multiple hook defs, like `PreToolUse`/`PostToolUse`, collect
/// multiple array entries).
fn build_claude_hooks_json() -> serde_json::Value {
    let mut hooks_map = serde_json::Map::new();

    for hook in AGENTCTL_CLAUDE_HOOKS {
        let entry = build_claude_hook_entry(hook);
        hooks_map
            .entry(hook.event.to_string())
            .or_insert_with(|| json!([]))
            .as_array_mut()
            .expect("hooks_map entries are always initialized as arrays")
            .push(entry);
    }

    json!({ "hooks": serde_json::Value::Object(hooks_map) })
}

/// Install the agentctl plugin into the given plugin directory.
///
/// Remove-then-write, not merge: (1) create the parent `skills/` directory if
/// absent, (2) delete `plugin_dir` and all its contents if it already exists,
/// (3) write `.claude-plugin/plugin.json` and `hooks/hooks.json` fresh. Never
/// reads from or writes to `~/.claude/settings.json`.
///
/// This is the testable core; [`ClaudeHarness::install`] supplies the
/// default path.
pub fn install_claude_to(plugin_dir: &Path) -> Result<InstallSummary> {
    // 1. Create the parent `skills/` directory if it does not already exist.
    if let Some(parent) = plugin_dir.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory {}", parent.display()))?;
    }

    // 2. Delete any existing plugin directory outright — no diffing.
    if plugin_dir.exists() {
        fs::remove_dir_all(plugin_dir)
            .with_context(|| format!("failed to remove {}", plugin_dir.display()))?;
    }

    // 3. Write the manifest and hooks files fresh.
    let manifest_path = claude_plugin_manifest_path(plugin_dir);
    if let Some(parent) = manifest_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory {}", parent.display()))?;
    }
    let manifest_json = serde_json::to_string_pretty(&claude_plugin_manifest())
        .context("failed to serialize plugin.json")?;
    fs::write(&manifest_path, manifest_json)
        .with_context(|| format!("failed to write {}", manifest_path.display()))?;

    let hooks_path = claude_hooks_json_path(plugin_dir);
    if let Some(parent) = hooks_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory {}", parent.display()))?;
    }
    let hooks_json = serde_json::to_string_pretty(&build_claude_hooks_json())
        .context("failed to serialize hooks.json")?;
    fs::write(&hooks_path, hooks_json)
        .with_context(|| format!("failed to write {}", hooks_path.display()))?;

    // 4. Build human-readable summary.
    Ok(InstallSummary(format_claude_install_summary(plugin_dir)))
}

/// Build the human-readable install summary string.
///
/// Exposed as a separate function so unit tests can assert on its output
/// without capturing stdout. Every hook is reported as freshly written (`+`)
/// since install always deletes-then-recreates the plugin directory — there
/// is no partial-update state to distinguish.
pub fn format_claude_install_summary(plugin_dir: &Path) -> String {
    let mut lines = vec![format!(
        "Installed agentctl plugin for claude ({} hook entries):",
        AGENTCTL_CLAUDE_HOOKS.len()
    )];

    lines.push(format!(
        "  + {}",
        tilde_collapse(&claude_plugin_manifest_path(plugin_dir))
    ));
    lines.push(format!(
        "  + {}",
        tilde_collapse(&claude_hooks_json_path(plugin_dir))
    ));

    for hook in AGENTCTL_CLAUDE_HOOKS {
        let event_display = match hook.matcher {
            Some(m) => format!("{} ({})", hook.event, m),
            None => hook.event.to_string(),
        };
        let async_label = if hook.async_ { " [async]" } else { "" };
        lines.push(format!(
            "    {event_display:<35} → {}{}",
            hook.command, async_label
        ));
    }
    lines.push("(requires `jq` on PATH to parse hook payloads)".to_string());
    lines.join("\n")
}

/// Uninstall the agentctl plugin at the given plugin directory.
///
/// Deletes `plugin_dir` and all its contents if it exists; no-op (and exits
/// 0) if it does not. Never touches `~/.claude/settings.json`, and never
/// removes the parent `skills/` directory or any sibling entries within it.
///
/// This is the testable core; [`ClaudeHarness::uninstall`] supplies the
/// default path.
pub fn uninstall_claude_to(plugin_dir: &Path) -> Result<UninstallSummary> {
    let removed = plugin_dir.exists();
    if removed {
        fs::remove_dir_all(plugin_dir)
            .with_context(|| format!("failed to remove {}", plugin_dir.display()))?;
    }

    Ok(UninstallSummary(format_claude_uninstall_summary(
        plugin_dir, removed,
    )))
}

/// Build the human-readable uninstall summary string.
///
/// - `-` the plugin directory was removed
/// - `=` there was nothing to remove
pub fn format_claude_uninstall_summary(plugin_dir: &Path, removed: bool) -> String {
    let path_display = tilde_collapse(plugin_dir);
    if removed {
        format!("Uninstalled agentctl plugin for claude:\n  - {path_display}")
    } else {
        format!("Uninstalled agentctl plugin for claude:\n  = {path_display} (nothing to remove)")
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn claude_plugin_dir(dir: &TempDir) -> PathBuf {
        dir.path()
            .join(".claude")
            .join("skills")
            .join(AGENTCTL_CLAUDE_PLUGIN_NAME)
    }

    fn read_hooks_json(plugin_dir: &Path) -> serde_json::Value {
        let content = fs::read_to_string(claude_hooks_json_path(plugin_dir))
            .expect("hooks.json should exist");
        serde_json::from_str(&content).expect("hooks.json should be valid JSON")
    }

    fn read_plugin_manifest(plugin_dir: &Path) -> serde_json::Value {
        let content = fs::read_to_string(claude_plugin_manifest_path(plugin_dir))
            .expect("plugin.json should exist");
        serde_json::from_str(&content).expect("plugin.json should be valid JSON")
    }

    #[test]
    fn install_claude_creates_plugin_files_when_absent() {
        let tmp = TempDir::new().unwrap();
        let plugin_dir = claude_plugin_dir(&tmp);

        assert!(
            !plugin_dir.exists(),
            "plugin directory should not exist before install"
        );

        install_claude_to(&plugin_dir).expect("install_claude_to should succeed");

        assert!(
            claude_plugin_manifest_path(&plugin_dir).exists(),
            "plugin.json should be created"
        );
        assert!(
            claude_hooks_json_path(&plugin_dir).exists(),
            "hooks.json should be created"
        );

        let manifest = read_plugin_manifest(&plugin_dir);
        assert_eq!(manifest["name"], "agentctl");
        assert_eq!(
            manifest["description"],
            "agentctl agent-state lifecycle hooks for Claude Code"
        );
    }

    #[test]
    fn install_claude_creates_skills_directory_when_absent() {
        let tmp = TempDir::new().unwrap();
        let plugin_dir = claude_plugin_dir(&tmp);
        let skills_dir = plugin_dir.parent().unwrap();

        assert!(!skills_dir.exists());

        install_claude_to(&plugin_dir).expect("install should succeed");

        assert!(skills_dir.exists(), "skills directory should be created");
        assert!(plugin_dir.exists(), "plugin directory should be created");
    }

    #[test]
    fn install_claude_reinstall_replaces_existing_directory() {
        let tmp = TempDir::new().unwrap();
        let plugin_dir = claude_plugin_dir(&tmp);

        fs::create_dir_all(&plugin_dir).unwrap();
        fs::write(plugin_dir.join("stray-file.txt"), "not written by agentctl").unwrap();

        install_claude_to(&plugin_dir).expect("install should succeed");

        assert!(
            !plugin_dir.join("stray-file.txt").exists(),
            "stray pre-existing file should be removed on re-install"
        );
        assert!(claude_plugin_manifest_path(&plugin_dir).exists());
        assert!(claude_hooks_json_path(&plugin_dir).exists());
    }

    #[test]
    fn install_claude_hooks_json_contains_all_eleven_entries() {
        let tmp = TempDir::new().unwrap();
        let plugin_dir = claude_plugin_dir(&tmp);

        install_claude_to(&plugin_dir).expect("install should succeed");

        let parsed = read_hooks_json(&plugin_dir);
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
                "hooks.{event} should have exactly two entries (unmatched + matched)"
            );
        }

        let total: usize = hooks
            .as_object()
            .unwrap()
            .values()
            .map(|v| v.as_array().unwrap().len())
            .sum();
        assert_eq!(
            total,
            AGENTCTL_CLAUDE_HOOKS.len(),
            "total entry count should be eleven"
        );
    }

    #[test]
    fn install_claude_async_flags_are_correct() {
        let tmp = TempDir::new().unwrap();
        let plugin_dir = claude_plugin_dir(&tmp);

        install_claude_to(&plugin_dir).expect("install");

        let parsed = read_hooks_json(&plugin_dir);
        let hooks = &parsed["hooks"];

        for event in &[
            "UserPromptSubmit",
            "PostToolUse",
            "PostToolUseFailure",
            "PreToolUse",
            "Stop",
            "StopFailure",
        ] {
            let hook_entry = &hooks[event][0]["hooks"][0];
            assert_eq!(
                hook_entry.get("async"),
                Some(&serde_json::Value::Bool(true)),
                "{event} should have async: true"
            );
        }

        for event in &["SessionStart", "SessionEnd"] {
            let hook_entry = &hooks[event][0]["hooks"][0];
            assert!(
                hook_entry.get("async").is_none(),
                "{event} should not have async field"
            );
        }
    }

    #[test]
    fn install_claude_session_start_has_correct_matcher() {
        let tmp = TempDir::new().unwrap();
        let plugin_dir = claude_plugin_dir(&tmp);
        install_claude_to(&plugin_dir).expect("install");

        let parsed = read_hooks_json(&plugin_dir);
        let session_start = &parsed["hooks"]["SessionStart"][0];
        assert_eq!(
            session_start.get("matcher").and_then(|v| v.as_str()),
            Some("startup|resume|clear"),
            "SessionStart should have matcher startup|resume|clear"
        );
    }

    #[test]
    fn install_claude_pretooluse_blocked_has_correct_matcher() {
        let tmp = TempDir::new().unwrap();
        let plugin_dir = claude_plugin_dir(&tmp);
        install_claude_to(&plugin_dir).expect("install");

        let parsed = read_hooks_json(&plugin_dir);
        let hooks = parsed["hooks"]["PreToolUse"]
            .as_array()
            .expect("PreToolUse should be an array");

        let blocked_entry = hooks.iter().find(|entry| {
            entry
                .get("hooks")
                .and_then(|h| h.as_array())
                .map(|hs| {
                    hs.iter().any(|h| {
                        h.get("command")
                            .and_then(|c| c.as_str())
                            .map(|c| {
                                c.contains("agentctl agent event TOOL_CALL_START")
                                    && c.contains("--type elicitation")
                            })
                            .unwrap_or(false)
                    })
                })
                .unwrap_or(false)
        });
        let entry = blocked_entry.expect("should have a blocked PreToolUse entry");
        assert_eq!(
            entry.get("matcher").and_then(|v| v.as_str()),
            Some("AskUserQuestion|ExitPlanMode"),
            "blocked PreToolUse should have matcher AskUserQuestion|ExitPlanMode"
        );
    }

    #[test]
    fn install_claude_posttooluse_has_two_entries() {
        let tmp = TempDir::new().unwrap();
        let plugin_dir = claude_plugin_dir(&tmp);
        install_claude_to(&plugin_dir).expect("install");

        let parsed = read_hooks_json(&plugin_dir);
        let entries = parsed["hooks"]["PostToolUse"]
            .as_array()
            .expect("PostToolUse should be an array");
        assert_eq!(entries.len(), 2, "PostToolUse should have two entries");

        let matched = entries
            .iter()
            .find(|e| {
                e.get("matcher").and_then(|m| m.as_str()) == Some("AskUserQuestion|ExitPlanMode")
            })
            .expect("should have a matched PostToolUse entry");
        let matched_cmd = matched["hooks"][0]
            .get("command")
            .and_then(|v| v.as_str())
            .expect("matched PostToolUse hook should have a command");
        assert!(matched_cmd.contains("agentctl agent event TOOL_CALL_RESULT"));
        assert!(matched_cmd.contains("--type elicitation"));

        let unmatched = entries
            .iter()
            .find(|e| e.get("matcher").is_none())
            .expect("should have an unmatched PostToolUse entry");
        let unmatched_cmd = unmatched["hooks"][0]
            .get("command")
            .and_then(|v| v.as_str())
            .unwrap();
        assert!(unmatched_cmd.contains("tool_use_id"));
        assert!(!unmatched_cmd.contains("--type"));
        assert!(!unmatched_cmd.contains("perm-"));
    }

    #[test]
    fn install_claude_posttoolusefailure_mirrors_posttooluse() {
        let tmp = TempDir::new().unwrap();
        let plugin_dir = claude_plugin_dir(&tmp);
        install_claude_to(&plugin_dir).expect("install");

        let parsed = read_hooks_json(&plugin_dir);
        let post_tool_use = parsed["hooks"]["PostToolUse"]
            .as_array()
            .expect("PostToolUse should be an array");
        let post_tool_use_failure = parsed["hooks"]["PostToolUseFailure"]
            .as_array()
            .expect("PostToolUseFailure should be an array");

        assert_eq!(
            post_tool_use_failure.len(),
            2,
            "PostToolUseFailure should have two entries"
        );

        for entry in post_tool_use {
            let matcher = entry.get("matcher").cloned();
            let command = entry["hooks"][0].get("command").cloned();
            let found = post_tool_use_failure.iter().any(|f| {
                f.get("matcher").cloned() == matcher
                    && f["hooks"][0].get("command").cloned() == command
            });
            assert!(
                found,
                "PostToolUseFailure should mirror PostToolUse entry with matcher {matcher:?}"
            );
        }

        assert!(
            post_tool_use_failure
                .iter()
                .all(|e| e["hooks"][0].get("async") == Some(&serde_json::Value::Bool(true))),
            "every PostToolUseFailure entry should be async"
        );
    }

    #[test]
    fn install_claude_stopfailure_has_run_finished_command_async_and_no_matcher() {
        let tmp = TempDir::new().unwrap();
        let plugin_dir = claude_plugin_dir(&tmp);
        install_claude_to(&plugin_dir).expect("install");

        let parsed = read_hooks_json(&plugin_dir);
        let entry = &parsed["hooks"]["StopFailure"][0];

        assert!(
            entry.get("matcher").is_none(),
            "StopFailure entry should have no matcher"
        );
        let hook = &entry["hooks"][0];
        let cmd = hook
            .get("command")
            .and_then(|v| v.as_str())
            .expect("StopFailure hook should have a command");
        assert!(cmd.contains("agentctl agent event RUN_FINISHED"));
        assert_eq!(
            hook.get("async"),
            Some(&serde_json::Value::Bool(true)),
            "StopFailure should have async: true"
        );
    }

    #[test]
    fn general_tool_call_tracking_entries_have_no_type_flag_and_are_async() {
        let pre = AGENTCTL_CLAUDE_HOOKS
            .iter()
            .find(|h| h.event == "PreToolUse" && h.matcher.is_none())
            .expect("should have an unmatched PreToolUse entry");
        assert!(pre.command.contains("tool_use_id"));
        assert!(pre.command.contains("agentctl agent event TOOL_CALL_START"));
        assert!(
            !pre.command.contains("--type"),
            "unmatched PreToolUse entry should not have a --type flag"
        );
        assert!(pre.async_, "unmatched PreToolUse entry should be async");

        let post = AGENTCTL_CLAUDE_HOOKS
            .iter()
            .find(|h| {
                h.event == "PostToolUse" && h.matcher.is_none() && !h.command.contains("perm-")
            })
            .expect("should have an untyped unmatched PostToolUse entry");
        assert!(post.command.contains("tool_use_id"));
        assert!(
            post.command
                .contains("agentctl agent event TOOL_CALL_RESULT")
        );
        assert!(
            !post.command.contains("--type"),
            "untyped unmatched PostToolUse entry should not have a --type flag"
        );
        assert!(
            post.async_,
            "untyped unmatched PostToolUse entry should be async"
        );
    }

    #[test]
    fn format_claude_install_summary_reports_all_entries_as_new() {
        let tmp = TempDir::new().unwrap();
        let plugin_dir = claude_plugin_dir(&tmp);
        let summary = format_claude_install_summary(&plugin_dir);

        assert!(summary.contains("plugin.json"));
        assert!(summary.contains("hooks.json"));
        for hook in AGENTCTL_CLAUDE_HOOKS {
            assert!(
                summary.contains(hook.command),
                "summary should mention command: {}",
                hook.command
            );
        }
        assert!(summary.contains("requires `jq` on PATH"));
    }

    #[test]
    fn uninstall_claude_removes_existing_plugin_directory() {
        let tmp = TempDir::new().unwrap();
        let plugin_dir = claude_plugin_dir(&tmp);

        install_claude_to(&plugin_dir).expect("install first");
        assert!(plugin_dir.exists());

        uninstall_claude_to(&plugin_dir).expect("uninstall should succeed");

        assert!(
            !plugin_dir.exists(),
            "plugin directory should be removed after uninstall"
        );
    }

    #[test]
    fn uninstall_claude_noop_when_plugin_absent() {
        let tmp = TempDir::new().unwrap();
        let plugin_dir = claude_plugin_dir(&tmp);

        assert!(!plugin_dir.exists());

        uninstall_claude_to(&plugin_dir).expect("uninstall should succeed even when absent");

        assert!(!plugin_dir.exists());
    }

    #[test]
    fn uninstall_claude_leaves_sibling_skills_entries_untouched() {
        let tmp = TempDir::new().unwrap();
        let plugin_dir = claude_plugin_dir(&tmp);
        let skills_dir = plugin_dir.parent().unwrap().to_path_buf();

        install_claude_to(&plugin_dir).expect("install first");

        let sibling_dir = skills_dir.join("other-plugin");
        fs::create_dir_all(&sibling_dir).unwrap();
        fs::write(sibling_dir.join("marker.txt"), "not agentctl").unwrap();

        uninstall_claude_to(&plugin_dir).expect("uninstall should succeed");

        assert!(
            !plugin_dir.exists(),
            "agentctl plugin directory should be removed"
        );
        assert!(
            skills_dir.exists(),
            "~/.claude/skills/ itself should remain in place"
        );
        assert!(
            sibling_dir.join("marker.txt").exists(),
            "sibling plugin directory should be untouched"
        );
    }

    #[test]
    fn uninstall_claude_idempotent_second_run_noop() {
        let tmp = TempDir::new().unwrap();
        let plugin_dir = claude_plugin_dir(&tmp);

        install_claude_to(&plugin_dir).expect("install first");
        uninstall_claude_to(&plugin_dir).expect("first uninstall");

        assert!(!plugin_dir.exists());

        uninstall_claude_to(&plugin_dir).expect("second uninstall should also succeed");

        assert!(!plugin_dir.exists());
    }

    #[test]
    fn format_claude_uninstall_summary_nothing_to_remove() {
        let tmp = TempDir::new().unwrap();
        let plugin_dir = claude_plugin_dir(&tmp);
        let summary = format_claude_uninstall_summary(&plugin_dir, false);
        assert!(summary.contains("= "));
        assert!(summary.contains("nothing to remove"));
    }

    #[test]
    fn format_claude_uninstall_summary_reports_removed_directory() {
        let tmp = TempDir::new().unwrap();
        let plugin_dir = claude_plugin_dir(&tmp);
        let summary = format_claude_uninstall_summary(&plugin_dir, true);
        let lines: Vec<&str> = summary.lines().collect();
        assert!(lines[1].trim_start().starts_with('-'));
        assert!(lines[1].contains("agentctl"));
    }

    #[test]
    fn harness_name_is_claude() {
        assert_eq!(ClaudeHarness.name(), "claude");
    }
}
