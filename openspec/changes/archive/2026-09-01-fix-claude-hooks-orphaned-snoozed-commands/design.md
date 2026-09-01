## Context

`~/.claude/settings.json` was found to contain these four hook entries (all
currently invisible to `main`'s `rd install claude`/`rd uninstall claude`):

| Event         | Matcher   | Command                                   |
| ------------- | --------- | ----------------------------------------- |
| `PreToolUse`  | `Monitor` | `rd agent watch arm --harness claude`     |
| `PostToolUse` | `Monitor` | `rd agent watch resolve --harness claude` |
| `Stop`        | —         | `rd agent stop --harness claude`          |
| `StopFailure` | —         | `rd agent stop --harness claude`          |

`git log --all` traces these to commit `1005afb` ("Add AgentState::Snoozed and
Claude Monitor-tool detection") on the `new-agent-status-snoozed` branch,
implementing the (never-merged) `add-snoozed-agent-state` OpenSpec change. That
branch added `rd agent watch arm`/`rd agent watch resolve` (arms/ resolves a
per-pane "Monitor" tool watch) and `rd agent stop` (writes `snoozed` instead of
`done` when a watch is armed), and wired its `rd install claude` to register the
four entries above. The branch was based on an older point in `main`'s history
(before the `agent status`→`agent event` rename and `track-tool-calls`) and
diverged without merging.

`main`'s `is_radr_command` recognizes every _other_ historical rename
(`rd set-state`/`rd clear-state` → `rd state set`/`rd state clear` →
`rd agent status`/`rd agent clear` → today's
`rd agent event ...`/`rd agent clear`) specifically so `rd install claude` can
detect and clean up stale entries from any prior `rd` vintage.
`rd agent watch ...`/`rd agent stop` were never added to that recognition list,
because they were never part of any `main` release — they leaked into a real
settings.json only because a feature-branch build of `rd` was used to run
`rd install claude` at some point. The effect is identical to the renames
`is_radr_command` already handles: a real `~/.claude/settings.json` contains
command strings that a current `rd` binary doesn't know how to run, and nothing
currently cleans them up.

## Goals / Non-Goals

**Goals:**

- Make `rd install claude`/`rd uninstall claude` (built from `main`) recognize
  and remove `rd agent watch ...`/`rd agent stop` hook entries as radr-managed,
  so they get cleaned up like every other historical form.
- Leave the door open for a future, from-scratch reintroduction of Monitor-tool
  watching (if `add-snoozed-agent-state` is ever revived) — this change only
  stops `main` from leaving dead entries behind; it does not resurrect or reject
  the feature itself.

**Non-Goals:**

- Not re-implementing `rd agent watch`/`rd agent stop`, `AgentState::Snoozed`,
  or Monitor-tool detection. That remains scoped to a separate decision about
  whether to revive `add-snoozed-agent-state`.
- Not adding replacement hook entries for the removed ones — there is currently
  no `main` feature for these entries to be migrated _to_; they are pure
  removals, same as the `PreToolUse` "old general working" entry removed in
  `fix-claude-hooks-state-accuracy` (#18).
- Not changing recognition/removal for the `AskUserQuestion`/`ExitPlanMode`/
  `permission_prompt`/`perm-$id` entries, which are unrelated and already
  correctly recognized.

## Decisions

- **Treat `rd agent watch ...`/`rd agent stop` as superseded-with-no-
  successor**, exactly like the existing "old general `PreToolUse` → working"
  case in `is_radr_command`'s cleanup pass: once recognized,
  `get_single_radr_command` returns the command,
  `radr_lookup.get(&(event, cmd))` misses (no `ClaudeHookDef` in
  `RADR_CLAUDE_HOOKS` matches these strings), and the existing "superseded —
  schedule for removal" branch already deletes the entry. **No new code path is
  needed for removal itself** — only the recognition predicate needs widening.
  - Alternative considered: special-case these two command families with their
    own migration message (distinct from the generic "removed" label). Rejected:
    the existing generic "[removed]" summary line already communicates removal
    clearly, and adding a second removal category would complicate
    `format_claude_summary` for a one-off, low-frequency cleanup case.
- **Match on `rd agent watch ` (prefix, trailing space) and `rd agent stop`
  (prefix, since it takes `--harness`/other flags)**, mirroring the existing
  prefix-matching style used for `rd agent status ` and `rd state set `.
  `rd agent watch arm ...`/`rd agent watch resolve ...` both start with
  `rd agent watch `, and `rd agent stop --harness claude` starts with
  `rd agent stop`; a single prefix each covers both variants without needing to
  enumerate `arm`/`resolve` separately.
- **No `RADR_CLAUDE_HOOKS` changes.** Since there's no `main` replacement
  behavior for "Monitor" tool arming/resolving or snoozed-on-stop, nothing is
  added to the table; the fix is purely subtractive.

## Risks / Trade-offs

- [Widening `is_radr_command` with a broad `rd agent stop` prefix could, in
  principle, match a future unrelated subcommand that happens to start the same
  way] → Acceptable: `rd agent stop` is not currently a valid `main` subcommand
  and clap's exhaustive `AgentSubcommand` enum means any future reuse of that
  name would need a deliberate, reviewed code change; revisit the predicate then
  if needed.
- [Only fixes settings.json files already on disk the next time `rd install
  claude`/`rd uninstall claude` runs — doesn't retroactively fix files without a
  user action] → Consistent with how every prior rename migration in this
  codebase already works (documented precedent in `claude-state-hooks` spec's
  existing migration scenarios); no new precedent being set.

## Migration Plan

1. Add `rd agent watch ` (prefix) and `rd agent stop` (prefix) to
   `is_radr_command`'s recognized-forms list in `src/commands/hooks.rs`,
   updating its doc comment to describe the new forms and why they exist (leaked
   from an unmerged branch).
2. Add unit tests seeding a settings.json with the four orphaned entries (see
   Context table) under existing non-radr and radr entries, and asserting:
   - `rd install claude` removes all four and reports them in the `[removed]`
     summary, leaving all currently-shipped entries untouched.
   - `rd uninstall claude` also recognizes and removes them.
3. Update `claude-state-hooks` spec with the new requirement/scenarios.
4. No rollback concerns: reverting the recognition-predicate change simply stops
   future cleanup of these specific strings; it does not affect any
   currently-shipped hook behavior.

## Open Questions

- None outstanding. (Whether to ever revive `add-snoozed-agent-state` on top of
  current `main` is out of scope for this fix and left for a separate proposal
  if desired.)
