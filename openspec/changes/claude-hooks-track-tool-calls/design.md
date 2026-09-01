## Context

`rd list` shows an active-tool-count column derived from `active_tools` in
`state.json`, incremented on `TOOL_CALL_START` and decremented on
`TOOL_CALL_RESULT` (see `agent-events` / `agent-state` specs, from the
`track-tool-calls` change). The pi extension implements this with two general
handlers that fire on every tool call:

```ts
pi.on("tool_call", async (event) => {
  rd(
    "agent",
    "event",
    "TOOL_CALL_START",
    "--id",
    event.toolCallId,
    "--harness",
    "pi",
  );
});
pi.on("tool_result", async (event) => {
  rd(
    "agent",
    "event",
    "TOOL_CALL_RESULT",
    "--id",
    event.toolCallId,
    "--harness",
    "pi",
  );
});
```

`RADR_CLAUDE_HOOKS` in `src/commands/hooks.rs` has no equivalent. Its only
`TOOL_CALL_START`/`TOOL_CALL_RESULT` entries are scoped to
`matcher: "AskUserQuestion|ExitPlanMode"` (used to drive the `blocked` status,
not tool-count tracking) plus a `"perm-$id"` pseudo-id pair used solely to close
out permission-prompt blocking windows. Consequently `active_tools` never goes
above zero for ordinary Claude tool calls (`Read`, `Edit`, `Bash`, etc.), even
though the feature was designed to track exactly that. This is the parity gap
the user flagged as "hooks still using old subcommands" — the Claude hook set
reflects the pre-`track-tool-calls` design and was never extended.

## Goals / Non-Goals

**Goals:**

- Give Claude sessions the same per-tool-call `active_tools` tracking pi
  sessions already have.
- Keep the existing `blocked`-status machinery (elicitation-typed
  `PreToolUse`/`PostToolUse`, `Notification`, unmatched `PostToolUse` "perm-$id"
  pair) completely unchanged.
- Preserve `rd install claude` idempotency and its migration path for
  settings.json files written by older `rd` binaries.

**Non-Goals:**

- No change to the status FSM, event names, or `rd agent event` CLI surface.
- No change to pi's hooks (already correct).
- Not attempting to unify the "perm-$id" permission-block bookkeeping with the
  new general tool-count hooks — they track different things (blocked status vs.
  active tool count) and use different id namespaces.

## Decisions

- **Add two new, unmatched hook entries** rather than widening the existing
  elicitation-typed entries: `PreToolUse` (no matcher) →
  `TOOL_CALL_START --id <tool_use_id> --harness claude` (async), and
  `PostToolUse` (no matcher) →
  `TOOL_CALL_RESULT --id <tool_use_id> --harness claude` (async), with no
  `--type` flag. This mirrors pi's `tool_call`/ `tool_result` handlers (also
  untyped) and keeps `active_tools` bookkeeping symmetric with the FSM's default
  (non-elicitation) tool-call path.
  - Alternative considered: reuse the existing unmatched `PostToolUse` entry
    (currently `TOOL_CALL_RESULT --id "perm-$id" --type elicitation`) by adding
    a second command to the same hook array entry. Rejected: Claude Code hook
    entries support multiple commands per event/matcher pair, but
    `get_single_radr_command`'s migration/cleanup logic in `hooks.rs`
    specifically only recognizes and manages _single_-command entries;
    conflating the two would require rewriting that recognition logic for
    marginal benefit. Two separate entries keep each hook's purpose and
    id-namespace independent and keep the existing single-command invariant
    intact.
- **Use `tool_use_id`, not `session_id`**, as the general entries' `--id`,
  matching the existing elicitation-typed `PreToolUse`/`PostToolUse` entries
  (which already extract `tool_use_id` from the same hook payload), so each
  concurrent tool call gets a distinct id and `TOOL_CALL_START`/
  `TOOL_CALL_RESULT` pairs correctly for parallel tool calls.
- **`RADR_CLAUDE_HOOKS` grows from nine to eleven entries.** No existing entry's
  command string, matcher, or async flag changes, so the existing cleanup pass
  (superseded-entry removal, stale-matcher update) needs no new logic beyond the
  add-or-skip pass already picking up newly-defined hooks that aren't yet
  present in a given settings.json — this is the same mechanism that already
  lets `rd install claude` add new hooks to files written by older binaries (no
  code change needed there beyond the two new `ClaudeHookDef` entries).

## Risks / Trade-offs

- [Every tool call now shells out to `jq` and `rd` twice more per turn
  (PreToolUse + PostToolUse), adding process-spawn overhead per tool call] →
  Mitigated by `async: true` (already used for all other per-tool hooks); this
  matches the overhead profile Claude sessions already accept for the
  elicitation-typed entries and pi already accepts for every tool call.
- [Existing installs need to re-run `rd install claude` to pick up the two new
  entries; until then, `active_tools` stays at 0 for Claude sessions] → No
  auto-upgrade mechanism exists for either harness today (pi extensions have the
  same limitation), so this matches existing precedent; call out in the summary
  output that new entries were added.
- [`active_tools` could double-count for `AskUserQuestion`/`ExitPlanMode` since
  those tools now fire both the general and the elicitation-typed
  `PreToolUse`/`PostToolUse` hooks] → Intentional and consistent with pi's
  behavior, where `tool_call`/`tool_result` fire for every tool regardless of
  type; `active_tools` already counts elicitation tool calls today (per
  `agent-events` spec, `TOOL_CALL_START`/`RESULT` increment/decrement regardless
  of `--type`), so this is not a new form of double-counting introduced by this
  change — every existing `TOOL_CALL_START` already increments the same counter
  used by every other call.

## Migration Plan

1. Add the two `ClaudeHookDef` entries to `RADR_CLAUDE_HOOKS`.
2. Update `format_claude_summary`'s expected-entry-count references (nine →
   eleven) and any hard-coded "(N entries)" strings.
3. Update `README.md`'s Claude install example output and hook table.
4. Update `openspec/specs/claude-state-hooks/spec.md` per the delta spec in this
   change.
5. Re-run `rd install claude` on developer machines (including CI fixtures in
   `tests/hooks.rs`) to confirm the add-or-skip pass appends the two new entries
   to an existing nine-entry settings.json without touching the other nine.

No rollback concerns beyond reverting the commit; `rd uninstall claude` already
removes any entry whose single command matches
`rd agent event TOOL_CALL_START `/`rd agent event TOOL_CALL_RESULT ` regardless
of `--type`, so the two new entries are cleaned up by the existing
`is_radr_command` matching with no changes needed there.

## Open Questions

- None outstanding.
