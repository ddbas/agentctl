## Why

`~/.claude/settings.json` on at least one developer machine contains four Claude
hook entries invoking `agentctl agent watch arm --harness claude`,
`agentctl agent watch resolve --harness claude`, and
`agentctl agent stop --harness claude` (twice, under `Stop` and `StopFailure`).
None of these subcommands exist anywhere in `src/main.rs` on `main` — they were
only ever implemented on the unmerged `new-agent-status-snoozed` branch
(`add-snoozed-agent-state` OpenSpec change), whose `agentctl install claude`
wrote these entries while that branch's binary was in use. `main`'s
`is_agentctl_command` in `src/commands/hooks.rs` never learned to recognize
these command strings, so every subsequent
`agentctl install claude`/`agentctl uninstall claude` run (using a `main`-built
`agentctl`) silently leaves them in place: they are permanently invisible to the
cleanup/migration pass that already exists for every other historical rename
(`agentctl set-state`, `agentctl state set`, `agentctl agent status`). Each time
Claude fires the `Monitor`-matched `PreToolUse`/`PostToolUse` hooks or the
`Stop`/`StopFailure` hooks, it now shells out to a nonexistent `agentctl`
subcommand, which exits non-zero and (for the synchronous ones) can surface as
hook errors to the user.

## What Changes

- Widen `is_agentctl_command` in `src/commands/hooks.rs` to also recognize
  `agentctl agent watch ` (prefix) and `agentctl agent stop` (with or without
  trailing flags) as agentctl-managed command strings, alongside the existing
  recognized forms.
- Because these two command families have no successor in
  `AGENTCTL_CLAUDE_HOOKS` on `main` (the feature that introduced them was never
  merged), the existing superseded-entry-removal logic in `install_claude_to`
  will delete them outright on the next `agentctl install claude` — no
  replacement entries are added. `agentctl uninstall claude` will likewise now
  recognize and remove them.
- Add unit tests seeding a settings.json with the four orphaned entries and
  asserting `agentctl install claude` and `agentctl uninstall claude` both clean
  them up.
- Update `claude-state-hooks` spec with a requirement covering recognition and
  removal of these orphaned/never-shipped command forms.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `claude-state-hooks`: `agentctl install claude`/`agentctl uninstall claude`'s
  agentctl-command recognition additionally covers
  `agentctl agent watch ...`/`agentctl agent stop` command strings (from a build
  of `agentctl` that implemented an unmerged feature) as superseded,
  unconditionally-removable agentctl-managed entries.

## Impact

- `src/commands/hooks.rs`: `is_agentctl_command` gains two more recognized
  prefixes/forms; no changes to `AGENTCTL_CLAUDE_HOOKS` itself (no successor
  entries to add).
- `openspec/specs/claude-state-hooks/spec.md`: one new requirement/scenario set
  describing recognition and removal of these orphaned command forms.
- Tests in `src/commands/hooks.rs` (unit) verifying install/uninstall clean up
  the four orphaned entries.
- No changes to `agentctl agent event`, `agentctl agent clear`, the status FSM,
  or any currently-shipped subcommand.
- Anyone who ran an `agentctl` build from the `new-agent-status-snoozed` branch
  and installed Claude hooks from it needs to re-run `agentctl install claude`
  (or `agentctl uninstall claude`) with a `main`-built `agentctl` after this
  change lands to actually clean up their `~/.claude/settings.json`.
