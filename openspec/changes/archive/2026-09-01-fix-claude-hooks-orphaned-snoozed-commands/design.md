## Context

`~/.claude/settings.json` was found to contain these four hook entries (all
currently invisible to `main`'s
`agentctl install claude`/`agentctl uninstall claude`):

| Event         | Matcher   | Command                                         |
| ------------- | --------- | ----------------------------------------------- |
| `PreToolUse`  | `Monitor` | `agentctl agent watch arm --harness claude`     |
| `PostToolUse` | `Monitor` | `agentctl agent watch resolve --harness claude` |
| `Stop`        | —         | `agentctl agent stop --harness claude`          |
| `StopFailure` | —         | `agentctl agent stop --harness claude`          |

`git log --all` traces these to commit `1005afb` ("Add AgentState::Snoozed and
Claude Monitor-tool detection") on the `new-agent-status-snoozed` branch,
implementing the (never-merged) `add-snoozed-agent-state` OpenSpec change. That
branch added `agentctl agent watch arm`/`agentctl agent watch resolve` (arms/
resolves a per-pane "Monitor" tool watch) and `agentctl agent stop` (writes
`snoozed` instead of `done` when a watch is armed), and wired its
`agentctl install claude` to register the four entries above. The branch was
based on an older point in `main`'s history (before the
`agent status`→`agent event` rename and `track-tool-calls`) and diverged without
merging.

`main`'s `is_agentctl_command` recognizes every _other_ historical rename
(`agentctl set-state`/`agentctl clear-state` →
`agentctl state set`/`agentctl state clear` →
`agentctl agent status`/`agentctl agent clear` → today's
`agentctl agent event ...`/`agentctl agent clear`) specifically so
`agentctl install claude` can detect and clean up stale entries from any prior
`agentctl` vintage. `agentctl agent watch ...`/`agentctl agent stop` were never
added to that recognition list, because they were never part of any `main`
release — they leaked into a real settings.json only because a feature-branch
build of `agentctl` was used to run `agentctl install claude` at some point. The
effect is identical to the renames `is_agentctl_command` already handles: a real
`~/.claude/settings.json` contains command strings that a current `agentctl`
binary doesn't know how to run, and nothing currently cleans them up.

## Goals / Non-Goals

**Goals:**

- Make `agentctl install claude`/`agentctl uninstall claude` (built from `main`)
  recognize and remove `agentctl agent watch ...`/`agentctl agent stop` hook
  entries as agentctl-managed, so they get cleaned up like every other
  historical form.
- Leave the door open for a future, from-scratch reintroduction of Monitor-tool
  watching (if `add-snoozed-agent-state` is ever revived) — this change only
  stops `main` from leaving dead entries behind; it does not resurrect or reject
  the feature itself.

**Non-Goals:**

- Not re-implementing `agentctl agent watch`/`agentctl agent stop`,
  `AgentState::Snoozed`, or Monitor-tool detection. That remains scoped to a
  separate decision about whether to revive `add-snoozed-agent-state`.
- Not adding replacement hook entries for the removed ones — there is currently
  no `main` feature for these entries to be migrated _to_; they are pure
  removals, same as the `PreToolUse` "old general working" entry removed in
  `fix-claude-hooks-state-accuracy` (#18).
- Not changing recognition/removal for the `AskUserQuestion`/`ExitPlanMode`/
  `permission_prompt`/`perm-$id` entries, which are unrelated and already
  correctly recognized.

## Decisions

- **Treat `agentctl agent watch ...`/`agentctl agent stop` as
  superseded-with-no- successor**, exactly like the existing "old general
  `PreToolUse` → working" case in `is_agentctl_command`'s cleanup pass: once
  recognized, `get_single_agentctl_command` returns the command,
  `agentctl_lookup.get(&(event, cmd))` misses (no `ClaudeHookDef` in
  `AGENTCTL_CLAUDE_HOOKS` matches these strings), and the existing "superseded —
  schedule for removal" branch already deletes the entry. **No new code path is
  needed for removal itself** — only the recognition predicate needs widening.
  - Alternative considered: special-case these two command families with their
    own migration message (distinct from the generic "removed" label). Rejected:
    the existing generic "[removed]" summary line already communicates removal
    clearly, and adding a second removal category would complicate
    `format_claude_summary` for a one-off, low-frequency cleanup case.
- **Match on `agentctl agent watch ` (prefix, trailing space) and
  `agentctl agent stop` (prefix, since it takes `--harness`/other flags)**,
  mirroring the existing prefix-matching style used for `agentctl agent status `
  and `agentctl state set `.
  `agentctl agent watch arm ...`/`agentctl agent watch resolve ...` both start
  with `agentctl agent watch `, and `agentctl agent stop --harness claude`
  starts with `agentctl agent stop`; a single prefix each covers both variants
  without needing to enumerate `arm`/`resolve` separately.
- **No `AGENTCTL_CLAUDE_HOOKS` changes.** Since there's no `main` replacement
  behavior for "Monitor" tool arming/resolving or snoozed-on-stop, nothing is
  added to the table; the fix is purely subtractive.

## Risks / Trade-offs

- [Widening `is_agentctl_command` with a broad `agentctl agent stop` prefix
  could, in principle, match a future unrelated subcommand that happens to start
  the same way] → Acceptable: `agentctl agent stop` is not currently a valid
  `main` subcommand and clap's exhaustive `AgentSubcommand` enum means any
  future reuse of that name would need a deliberate, reviewed code change;
  revisit the predicate then if needed.
- [Only fixes settings.json files already on disk the next time
  `agentctl install claude`/`agentctl uninstall claude` runs — doesn't
  retroactively fix files without a user action] → Consistent with how every
  prior rename migration in this codebase already works (documented precedent in
  `claude-state-hooks` spec's existing migration scenarios); no new precedent
  being set.

## Migration Plan

1. Add `agentctl agent watch ` (prefix) and `agentctl agent stop` (prefix) to
   `is_agentctl_command`'s recognized-forms list in `src/commands/hooks.rs`,
   updating its doc comment to describe the new forms and why they exist (leaked
   from an unmerged branch).
2. Add unit tests seeding a settings.json with the four orphaned entries (see
   Context table) under existing non-agentctl and agentctl entries, and
   asserting:
   - `agentctl install claude` removes all four and reports them in the
     `[removed]` summary, leaving all currently-shipped entries untouched.
   - `agentctl uninstall claude` also recognizes and removes them.
3. Update `claude-state-hooks` spec with the new requirement/scenarios.
4. No rollback concerns: reverting the recognition-predicate change simply stops
   future cleanup of these specific strings; it does not affect any
   currently-shipped hook behavior.

## Open Questions

- None outstanding. (Whether to ever revive `add-snoozed-agent-state` on top of
  current `main` is out of scope for this fix and left for a separate proposal
  if desired.)
