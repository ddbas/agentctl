## Why

`~/.claude/settings.json` on at least one developer machine contains four Claude
hook entries invoking `rd agent watch arm --harness claude`,
`rd agent watch resolve --harness claude`, and `rd agent stop --harness claude`
(twice, under `Stop` and `StopFailure`). None of these subcommands exist
anywhere in `src/main.rs` on `main` — they were only ever implemented on the
unmerged `new-agent-status-snoozed` branch (`add-snoozed-agent-state` OpenSpec
change), whose `rd install claude` wrote these entries while that branch's
binary was in use. `main`'s `is_radr_command` in `src/commands/hooks.rs` never
learned to recognize these command strings, so every subsequent
`rd install claude`/`rd uninstall claude` run (using a `main`-built `rd`)
silently leaves them in place: they are permanently invisible to the
cleanup/migration pass that already exists for every other historical rename
(`rd set-state`, `rd state set`, `rd agent status`). Each time Claude fires the
`Monitor`-matched `PreToolUse`/`PostToolUse` hooks or the `Stop`/`StopFailure`
hooks, it now shells out to a nonexistent `rd` subcommand, which exits non-zero
and (for the synchronous ones) can surface as hook errors to the user.

## What Changes

- Widen `is_radr_command` in `src/commands/hooks.rs` to also recognize
  `rd agent watch ` (prefix) and `rd agent stop` (with or without trailing
  flags) as radr-managed command strings, alongside the existing recognized
  forms.
- Because these two command families have no successor in `RADR_CLAUDE_HOOKS` on
  `main` (the feature that introduced them was never merged), the existing
  superseded-entry-removal logic in `install_claude_to` will delete them
  outright on the next `rd install claude` — no replacement entries are added.
  `rd uninstall claude` will likewise now recognize and remove them.
- Add unit tests seeding a settings.json with the four orphaned entries and
  asserting `rd install claude` and `rd uninstall claude` both clean them up.
- Update `claude-state-hooks` spec with a requirement covering recognition and
  removal of these orphaned/never-shipped command forms.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `claude-state-hooks`: `rd install claude`/`rd uninstall claude`'s radr-command
  recognition additionally covers `rd agent watch ...`/`rd agent stop` command
  strings (from a build of `rd` that implemented an unmerged feature) as
  superseded, unconditionally-removable radr-managed entries.

## Impact

- `src/commands/hooks.rs`: `is_radr_command` gains two more recognized
  prefixes/forms; no changes to `RADR_CLAUDE_HOOKS` itself (no successor entries
  to add).
- `openspec/specs/claude-state-hooks/spec.md`: one new requirement/scenario set
  describing recognition and removal of these orphaned command forms.
- Tests in `src/commands/hooks.rs` (unit) verifying install/uninstall clean up
  the four orphaned entries.
- No changes to `rd agent event`, `rd agent clear`, the status FSM, or any
  currently-shipped subcommand.
- Anyone who ran an `rd` build from the `new-agent-status-snoozed` branch and
  installed Claude hooks from it needs to re-run `rd install claude` (or
  `rd uninstall claude`) with a `main`-built `rd` after this change lands to
  actually clean up their `~/.claude/settings.json`.
