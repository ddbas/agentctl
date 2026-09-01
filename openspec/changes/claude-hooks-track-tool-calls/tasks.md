## 1. Add general tool-call-tracking hook entries

- [ ] 1.1 Add a `ClaudeHookDef` for `PreToolUse` (no matcher) →
      `rd agent     event TOOL_CALL_START --id <tool_use_id> --harness claude`
      (async, no `--type`) to `RADR_CLAUDE_HOOKS` in `src/commands/hooks.rs`.
- [ ] 1.2 Add a `ClaudeHookDef` for `PostToolUse` (no matcher) →
      `rd agent     event TOOL_CALL_RESULT --id <tool_use_id> --harness claude`
      (async, no `--type`) to `RADR_CLAUDE_HOOKS`.
- [ ] 1.3 Update the doc comment above `RADR_CLAUDE_HOOKS` ("The nine radr hook
      entries...") to say eleven, and list the two new entries.
- [ ] 1.4 Update the hard-coded
      `"Installed radr hooks for claude (9     entries):"` string in
      `format_claude_summary` to say `(11 entries)`.

## 2. Update tests

- [ ] 2.1 Update `install_claude_clean_file_writes_all_eight_entries` (and any
      other test asserting fixed counts) in `src/commands/hooks.rs` to expect
      three `PreToolUse` entries and three `PostToolUse` entries (one unmatched
      untyped, one matched elicitation-typed, plus the existing unmatched
      `"perm-$id"` `PostToolUse` entry), for eleven entries total.
- [ ] 2.2 Add a unit test asserting the two new entries have
      `--id     <tool_use_id>`, no `--type` flag, and `async: true`.
- [ ] 2.3 Add a unit test (mirroring
      `install_claude_updates_stale_session_start_matcher`) that pre-populates a
      nine-entry settings.json (the pre-change entry set) and asserts
      `install_claude_to` adds exactly the two new entries, leaving the nine
      existing entries byte-for-byte unchanged.
- [ ] 2.4 Update `tests/hooks.rs`'s doc comments and event-count assertions
      (`nine` → `eleven`, `PreToolUse`/`PostToolUse` entry counts) to match.
- [ ] 2.5 Add an integration test in `tests/hooks.rs` verifying
      `rd install     claude` followed by simulated `PreToolUse`/`PostToolUse`
      hook invocations (piping a JSON payload with a `tool_use_id` into the
      generated command) increments and decrements `active_tools` in the
      resulting state file, mirroring existing pi tool-call-tracking test
      coverage if present.

## 3. Update documentation

- [ ] 3.1 Update `README.md`'s Claude install example output and hook table to
      include the two new entries and the eleven-entry total.
- [ ] 3.2 Update `openspec/specs/claude-state-hooks/spec.md` per the delta spec
      in this change (sync after implementation, per the `openspec-sync-specs`
      workflow, or apply directly if not using a separate sync step).

## 4. Verify

- [ ] 4.1 Run `mise run build` and `mise run test` (or equivalent
      `cargo build`/`cargo test` via `mise exec`), confirming no regressions.
- [ ] 4.2 Manually run `rd install claude` against a fresh
      `~/.claude/settings.json` and confirm eleven entries are written with the
      expected commands/matchers/async flags.
- [ ] 4.3 Manually run `rd install claude` against a settings.json seeded with
      only the pre-change nine entries and confirm exactly the two new entries
      are appended, with the nine existing entries unchanged.
- [ ] 4.4 Manually simulate a `PreToolUse`/`PostToolUse` pair for a
      non-interactive tool (e.g. `Bash`) via the generated shell command and
      confirm `active_tools` increments then decrements in the pane's
      `state.json`.
