## Context

Radr tracks Claude Code session state via Claude's hook system. The current
implementation registers six hooks in `~/.claude/settings.json` that call
`rd set-state` / `rd clear-state` at key lifecycle events. Three gaps were
identified through empirical testing:

1. **`SessionStart` matcher is `"startup"` only** — resumed sessions fire
   `SessionStart` with `source: "resume"`, which doesn't match, so state stays
   "unknown" instead of resetting to "idle".
2. **No `StopFailure` hook** — API errors fire `StopFailure` instead of `Stop`;
   the existing `Stop → done` hook never fires, leaving state stuck at
   "working".
3. **`AskUserQuestion` and `ExitPlanMode` don't signal "blocked"** — the general
   `PreToolUse → working` hook fires for these tools, but the user is actually
   blocked by an interactive dialog. A race condition prevents naively adding a
   specific `PreToolUse(AskUserQuestion) → blocked` hook alongside the general
   one (all matching hooks fire in parallel).

The fix spans `src/commands/hooks.rs` (the `RADR_CLAUDE_HOOKS` constant and
associated tests) and the `claude-state-hooks` spec.

## Goals / Non-Goals

**Goals:**

- Resumed and cleared sessions correctly report "idle"
- API-error stops correctly report "done"
- Interactive tool dialogs (`AskUserQuestion`, `ExitPlanMode`) report "blocked"
- No race conditions in hook execution
- `rd hooks install claude` remains idempotent

**Non-Goals:**

- Pi harness hooks (unaffected)
- Adding new `AgentState` variants (e.g., `error`)
- Tracking subagent state separately from the main session
- Supporting `SessionStart(compact)` → "idle" (compaction is mid-session; the
  next `UserPromptSubmit` or `PostToolUse` immediately re-signals "working")

## Decisions

### Decision: Widen SessionStart matcher to `startup|resume|clear`

**Rationale:** All three sources represent a session entering a quiescent state
where the user is about to give input. Setting "idle" on each is correct and
safe. `compact` is excluded because compaction occurs mid-turn and Claude
continues working immediately after; the existing `PostToolUse → working`
refresh handles re-signalling.

**Alternative considered:** Remove the matcher entirely (matches all four
sources including `compact`). Rejected because `compact` mid-turn briefly
flashes "idle" in the status bar while Claude is actively working.

### Decision: Replace `PreToolUse(*) → working` with `PostToolUse(*) → working`

**Rationale:** Moving the general "working" refresh to `PostToolUse` eliminates
the race condition between the general hook and a specific `PreToolUse` hook for
blocking tools. Claude Code fires all matching hooks for an event in parallel;
`PreToolUse(AskUserQuestion) → blocked` and `PreToolUse(*) → working` would race
because both are async. With `PostToolUse(*) → working`, there is no concurrent
hook for `PreToolUse(AskUserQuestion)` — the state transition is:

```
UserPromptSubmit → working
PreToolUse(AskUserQuestion) → blocked    ← no competing hook at this event
   user answers
PostToolUse(*) → working                 ← fires after AskUserQuestion completes
```

**Alternative considered:** Keep `PreToolUse(*) → working` and make the specific
`AskUserQuestion` hook synchronous, betting on execution order. Rejected as
non-deterministic and fragile across OS schedulers.

**Semantics note:** `PostToolUse` fires only on tool success.
`PostToolUseFailure` is separate. For radr's purposes "working" after a failed
tool is also correct (Claude processes the failure and continues), so no
`PostToolUseFailure` hook is added.

### Decision: `StopFailure → done` (not a new `error` state)

**Rationale:** Adding a new `AgentState::Error` variant is a cross-cutting
change (affects `state.rs`, display, list output, documentation). The practical
value is marginal — the user can see the error in Claude's UI. Mapping
`StopFailure` to `done` accurately signals "Claude is no longer working" without
expanding the state model.

### Decision: Hook count increases from 6 to 8

The final registered hooks are:

| #   | Event              | Matcher                         | Command                | Async |
| --- | ------------------ | ------------------------------- | ---------------------- | ----- |
| 1   | `SessionStart`     | `startup\|resume\|clear`        | `rd set-state idle`    | no    |
| 2   | `UserPromptSubmit` | —                               | `rd set-state working` | yes   |
| 3   | `PostToolUse`      | —                               | `rd set-state working` | yes   |
| 4   | `PreToolUse`       | `AskUserQuestion\|ExitPlanMode` | `rd set-state blocked` | yes   |
| 5   | `Stop`             | —                               | `rd set-state done`    | yes   |
| 6   | `StopFailure`      | —                               | `rd set-state done`    | yes   |
| 7   | `Notification`     | `permission_prompt`             | `rd set-state blocked` | yes   |
| 8   | `SessionEnd`       | —                               | `rd clear-state`       | no    |

Hook 3 (`PreToolUse` → working) is removed and replaced by hook 3 above.

## Risks / Trade-offs

- **`PostToolUse` fires only on success** → if every tool in a turn fails,
  "working" is never refreshed. `UserPromptSubmit` already set it at turn start,
  so state is correct; this is not a regression.
- **Widening `SessionStart` matcher** → existing installs have
  `matcher: "startup"`. `rd hooks install claude` is idempotent by command
  string — the old `startup`-matcher entry and the new
  `startup|resume|clear`-matcher entry have different structures, so both would
  coexist after re-install. The installer MUST detect and replace the old entry,
  not just skip it. Implementation note: de-duplication currently checks by
  command string only; this is sufficient because the command string is
  unchanged — the old entry won't be duplicated, but it also won't be updated. A
  separate migration step (remove old, add new) is needed, OR the idempotency
  check must include matcher comparison.
- **`ExitPlanMode` in the blocked matcher** → `ExitPlanMode` shows a plan
  approval dialog. Adding it alongside `AskUserQuestion` is low-risk and
  improves coverage of interactive dialogs.

## Migration Plan

Users who already ran `rd hooks install claude` will have the old hook set.
After updating `rd`:

1. Run `rd hooks install claude` again — new hooks (PostToolUse, StopFailure,
   AskUserQuestion-scoped PreToolUse) are added.
2. The old `PreToolUse(*)` → working entry remains (no removal logic). It is
   harmless alongside the new `PostToolUse(*)` → working entry; the duplicate
   "working" signal on `PreToolUse` is benign.
3. The old `SessionStart(startup)` entry remains alongside the new
   `SessionStart(startup|resume|clear)` entry. Both fire on `startup`, writing
   "idle" twice — benign. The new entry also fires on `resume`/`clear`.

This is acceptable for the initial fix. A future cleanup task can add removal of
superseded entries.

## Open Questions

_(none)_
