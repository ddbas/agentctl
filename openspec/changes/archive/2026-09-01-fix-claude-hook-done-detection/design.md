## Context

`agentctl`'s status FSM (`src/status_fsm.rs`) derives a pane's `status` from a
small set of lifecycle events (`SESSION_STARTED`, `RUN_STARTED`, `RUN_FINISHED`,
`TOOL_CALL_START`/`TOOL_CALL_RESULT`). It is intentionally lenient: an event
that doesn't match a defined `(state, input)` transition is a no-op for `status`
(still logged to `events.jsonl`). Today `RUN_FINISHED` only matches from
`Working`, and never consults `active_tools`. Separately, the Claude hook set
(`src/commands/hooks.rs`) tracks two independent things that currently share
bookkeeping in ways that conflict:

- **`active_tools`**: a plain in-flight counter, incremented by
  `TOOL_CALL_START` and decremented by `TOOL_CALL_RESULT`, _regardless of
  `--type`_. Real per-tool hooks (untyped `PreToolUse`/`PostToolUse`, and the
  elicitation-typed `AskUserQuestion`/`ExitPlanMode` pair) all mutate it, by
  design (see `agent-events` spec).
  - `Stop`/`StopFailure` already pass `--active-tools <n>` (derived from
    Claude's `background_tasks` array) to reconcile this counter against tools
    that fire `PostToolUse` before their background work actually finishes.
- **Permission-prompt blocking**: `Notification` (`permission_prompt`) opens a
  `Blocked` status via a synthetic
  `TOOL_CALL_START --id "perm-<session_id>" --type elicitation`; an unmatched
  `PostToolUse` (fires on _every_ tool completion) closes it via
  `TOOL_CALL_RESULT --id "perm-<session_id>" --type elicitation`. Because this
  reuses the `elicitation` type, and because `active_tools` mutation doesn't
  distinguish `--type`, **every tool completion double-decrements
  `active_tools`**: once via the real untyped close, once via this synthetic
  `perm-` close. This is currently harmless only because nothing reads
  `active_tools` as a gate.

Two independent bug reports motivate this change: panes stuck at `working`
forever (status desynced from `Blocked`/`Idle` when `Stop` fires, so
`RUN_FINISHED` is a silent no-op), and — once `active_tools` becomes a gate —
the risk of the opposite failure (`Done` firing while a second concurrent tool
call is genuinely still in flight, because of the double-decrement above).

## Goals / Non-Goals

**Goals:**

- `RUN_FINISHED` reliably drives a pane to `Done` regardless of what status it
  drifted into beforehand (`Stop`/`StopFailure` are authoritative).
- `RUN_FINISHED` does not fire `Done` while there is genuinely still in-flight
  work (`active_tools > 0`).
- Two concurrent permission-prompt blocks in the same Claude session cannot have
  the first one to resolve prematurely clear the other.
- Close the `active_tools` double-decrement hole so the two goals above don't
  fight each other.

**Non-Goals:**

- No change to the elicitation-typed `AskUserQuestion`/`ExitPlanMode`
  `PreToolUse`/`PostToolUse` behavior — that pair already correctly toggles
  `Blocked`/`Working` for a single interactive tool call and already
  intentionally mutates `active_tools` (existing spec behavior); this change
  does not touch it.
- No attempt to give `agentctl` real id-based correlation between a specific
  `Notification` permission prompt and the tool call that resolves it — Claude
  Code's `Notification` hook payload for `permission_prompt` carries no
  `tool_use_id` or other field that would let `agentctl` (or Claude itself) tie
  a specific prompt to a specific later `PostToolUse` invocation (tracked
  upstream as anthropics/claude-code#11128). Any fix must work without that
  correlation.
- No change to `agentctl install`/`agentctl uninstall` cleanup/migration
  mechanics beyond what's needed to update the two affected hook command strings
  (already handled generically by the existing superseded/stale-matcher passes).

## Decisions

### 1. `RUN_FINISHED` becomes a wildcard transition, gated on a caller-supplied `active_tools` value

`Event::RunFinished` gains a required field carrying the _reconciled_
`active_tools` count (computed by `agent_event.rs` before invoking the FSM,
exactly the same override value already threaded through from
`--active-tools`/Claude's `background_tasks`, defaulting to the pane's current
counter when no override is given):

```rust
RunFinished { active_tools: u32 }
```

Transition rule, replacing the single `(Working, RunFinished) => Done` arm:

```rust
(_, Event::RunFinished { active_tools: 0 }) => Some(Done),
(_, Event::RunFinished { .. }) => None, // still busy — no-op
```

This makes `RUN_FINISHED` authoritative from _any_ current status (including
`Idle`, `Blocked`, and `Done` itself, which is already idempotent), while still
refusing to mark `Done` when `active_tools > 0`. A `None` result is the existing
"no-op for status, still logged" behavior already used throughout the FSM, so no
new fallback semantics are introduced.

Alternative considered: keep `RunFinished` as a plain unit variant and apply the
gating as a post-hoc override in `agent_event.rs` (compute the FSM's answer,
then discard it if `active_tools > 0`). Rejected: the transition table in
`status_fsm.rs` is the single documented source of truth for status derivation
(see `agent-events` spec); splitting the actual decision logic across two files
would make the FSM's own unit tests unable to exercise the gating at all, and
would silently violate the "an event that doesn't match a transition doesn't
change status but is still logged" invariant since the override would need its
own bespoke special-casing.

### 2. `active_tools` and the new `blocked_prompts` counter are decoupled by tool-call type

Add a third `ToolCallType::PermissionPrompt` variant (CLI value
`permission-prompt`), used only by the `Notification`/`perm-` pair. Move the
`active_tools`-vs-not-active_tools branch in `agent_event.rs` from "always
mutate `active_tools` regardless of type" to:

- `Custom`, `Elicitation`, or no type → mutate `active_tools` (unchanged
  behavior).
- `PermissionPrompt` → mutate a new `blocked_prompts` counter instead, never
  `active_tools`.

This directly closes the double-decrement hole: the real untyped `PostToolUse`
close still decrements `active_tools` exactly once per tool completion; the
`perm-` close now decrements `blocked_prompts` instead.

Alternative considered: key the branch on the `--id` prefix (`"perm-"`) instead
of adding a type. Rejected: string-prefix sniffing on an otherwise-opaque
correlation id is fragile (a legitimate `tool_use_id` could theoretically
collide) and undocumented as a contract; an explicit type is self-describing and
matches how `Elicitation` already disambiguates the
`AskUserQuestion`/`ExitPlanMode` pair.

### 3. `Blocked` → `Working` for `PermissionPrompt` is gated on `blocked_prompts == 0`, mirroring decision 1

Mirroring the `RunFinished`/`active_tools` gate, `Event::ToolCallResult` for
`PermissionPrompt` carries the reconciled `blocked_prompts` count:

```rust
(Blocked, Event::ToolCallResult { r#type: Some(PermissionPrompt), blocked_prompts: 0 }) => Some(Working),
(Blocked, Event::ToolCallResult { r#type: Some(PermissionPrompt), .. }) => None,
```

with `Event::ToolCallStart { r#type: Some(PermissionPrompt), .. }` continuing to
open `Blocked` from `Working` exactly as `Elicitation` does today (a second,
concurrent open while already `Blocked` is a no-op for status, same as the
existing no-op-for-unmatched-state behavior, but still increments
`blocked_prompts`). This is what actually fixes "one resolved prompt closes out
a different still-pending one": with two prompts open, `blocked_prompts == 2`;
the first close brings it to `1` (no status change, stays `Blocked`); only the
second close brings it to `0` and clears `Blocked`.

This is why the literal recommendation to "key the `perm-` id on something
unique per prompt" doesn't fix anything by itself: the FSM's `transition`
function has never consulted `--id` (it only reads `(state, input-type)`), and
Claude's `Notification` payload has no data that would let two separate hook
invocations (the `Notification` open and the later `PostToolUse` close) agree on
a shared id even if we wanted them to. The counter above fixes the behavior
without needing any such correlation. The `perm-` id itself still gets a
unique-per-invocation suffix (e.g. an appended random value) purely so
`events.jsonl` shows each prompt/close as a distinct line for debugging — a
log-quality improvement, not a functional fix.

### 4. `blocked_prompts` follows the same file-schema conventions as `active_tools`

New `AgentInfo.blocked_prompts: u32` field (`#[serde(default)]` for backward
compatibility with pre-existing state files, exactly like `active_tools`'s own
migration), reset to `0` on `SESSION_STARTED` alongside `active_tools`, never
decremented below `0` (`saturating_sub`).

## Risks / Trade-offs

- [Existing installs need to re-run `agentctl install claude` to pick up the
  `--type permission-prompt` command-string change] → The existing stale-command
  cleanup pass in `install_claude_to` already removes superseded
  `(event, command)` pairs and adds the new ones; no new migration logic needed,
  same precedent as prior hook-set changes.
- [A pane whose `active_tools`/`blocked_prompts` counters are themselves wrong
  (e.g. an untracked background task never reconciled via `--active-tools`) will
  now get stuck at its current status instead of forcibly reaching `Done`] →
  This is the intended trade-off requested by point 3: refusing to mark `Done`
  while work might still be in flight is safer than the current false-`done`
  failure mode; a subsequent `RUN_FINISHED` (next turn) or a corrected
  `--active-tools` override still resolves it. Counter correctness itself is out
  of scope for this change.
- [Adding a third `ToolCallType` and a second counter increases surface area for
  future drift between `active_tools` and `blocked_prompts` bookkeeping] → Both
  counters follow the identical increment/decrement/reset pattern, and
  `agent-events`/`agent-state` specs document both explicitly, same as the
  existing `active_tools` precedent.

## Migration Plan

1. Land the FSM/state changes (`status_fsm.rs`, `state.rs`, `agent_event.rs`,
   `main.rs`) with full unit test coverage — no observable behavior change until
   the Claude hook set is also updated (pi's hooks never send
   `--type permission-prompt` and never rely on the
   `RunFinished`-from-non-`Working` wildcard today, so this is backward
   compatible).
2. Update `AGENTCTL_CLAUDE_HOOKS` in `src/commands/hooks.rs` (new command
   strings for `Notification`/unmatched `PostToolUse` perm-close entries) and
   update `README.md`'s hook table to match.
3. Users re-run `agentctl install claude` to pick up the updated entries
   (existing cleanup/add-or-skip pass handles this with no new code).

No rollback beyond reverting the change is needed — no persisted data format
becomes unreadable by older `agentctl` binaries (`blocked_prompts` is
`#[serde(default)]`).
