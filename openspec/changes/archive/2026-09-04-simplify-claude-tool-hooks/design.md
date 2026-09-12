## Context

`agentctl install claude` currently registers eleven hook entries. Two of them
exist to track "permission prompt" blocks as a `blocked` status distinct from
the `elicitation` blocks already driven by `PreToolUse`/`PostToolUse` on
`AskUserQuestion`/`ExitPlanMode`:

- `Notification` (matcher `permission_prompt`) opens the block:
  `TOOL_CALL_START --id "perm-<session_id>-$RANDOM" --type permission-prompt`
- An unmatched `PostToolUse` entry (the third `PostToolUse` registration,
  alongside the general untyped one and the `elicitation`-typed one) tries to
  close it:
  `TOOL_CALL_RESULT --id "perm-<session_id>-$RANDOM" --type permission-prompt`

Both commands generate their own `$RANDOM` in their own shell subprocess, so the
id used to open the block and the id used to (attempt to) close it are never the
same value. `agentctl agent event` correlates start/stop purely by `id` for the
FSM's bookkeeping (`events.jsonl` entries), and the `blocked_prompts` counter
was added specifically to keep a single premature "close" from clearing
`blocked` status while a second, concurrent permission prompt was still open — a
concern that only exists because the ids can't be trusted to pair up. Given they
never pair up at all, `blocked_prompts` doesn't actually protect anything: every
`permission_prompt` open is essentially orphaned, and every unmatched
`PostToolUse` "close" is a same-shaped guess fired after every single successful
tool call, matched or not.

Separately, `PostToolUse` only fires when a tool call **succeeds**. A failed
tool call (nonzero Bash exit, MCP tool error, aborted `Edit`, etc.) never fires
`PostToolUse`, so the `TOOL_CALL_START` opened by the general, untyped
`PreToolUse` registration is never closed by a `TOOL_CALL_RESULT`, and
`active_tools` leaks upward for the rest of the session.

## Goals / Non-Goals

**Goals:**

- Close every `TOOL_CALL_START` reliably, whether the tool call succeeds or
  fails, by adding `PostToolUseFailure` registrations that mirror `PostToolUse`
  exactly (same matchers, same `tool_use_id`-keyed command).
- Remove the permission-prompt mechanism entirely: the `Notification` hook, the
  extra unmatched `PostToolUse` entry, the `permission-prompt` `--type` value,
  and the `blocked_prompts` counter/field.
- Keep `blocked` status working for `AskUserQuestion`/`ExitPlanMode` exactly as
  it already does today, via ordinary `PreToolUse`/`PostToolUse`/
  `PostToolUseFailure` pairing on `tool_use_id` — no counter needed, since
  `tool_use_id` already correlates start and stop reliably.

**Non-Goals:**

- Reintroducing any permission-prompt/`Notification`-based signal. If a future
  need arises to detect Claude's permission dialog, it should be designed around
  an id that can actually be correlated (e.g. `PermissionRequest`'s own
  `tool_use_id`-less schema would need its own design), not resurrected as-is.
- Changing anything about `SessionStart`, `UserPromptSubmit`, `Stop`,
  `StopFailure`, or `SessionEnd`.
- Migrating existing `state.json` files' `blocked_prompts` value anywhere — it's
  simply dropped on next write; old files with the field must still deserialize
  (ignore unknown field) rather than error.

## Decisions

- **Mirror `PostToolUse` into `PostToolUseFailure` exactly.** Two new entries:
  unmatched (untyped, `TOOL_CALL_RESULT --id <tool_use_id>`) and matched
  (`AskUserQuestion|ExitPlanMode`, `--type elicitation`). Both async, same shape
  as their `PostToolUse` counterparts, since `PostToolUseFailure`'s input schema
  is documented as the same `tool_name`/`tool_input`/ `tool_use_id` shape as
  `PostToolUse` plus `error`/`is_interrupt`.
- **Delete the `Notification` hook and the extra unmatched `PostToolUse` entry**
  rather than trying to fix the id-pairing bug in place. The mechanism's purpose
  (signal "blocked" while Claude waits on a permission dialog) has no reliable
  id to correlate on without deeper changes to how Claude Code exposes
  `tool_use_id` on `Notification`, which it does not do today.
- **Drop `permission-prompt` as an accepted `--type` value** on
  `TOOL_CALL_START`/`TOOL_CALL_RESULT` rather than leaving it defined-but-
  unused, so `agentctl agent event --type bogus`-style validation errors reflect
  only the types that are actually dispatched (`custom`, `elicitation`).
- **Remove `blocked_prompts` entirely** rather than keeping it as a vestigial
  always-zero field. It was purpose-built for the permission-prompt concurrency
  problem; with that mechanism gone there's no remaining writer or reader for
  it. `elicitation`'s `Blocked` ↔ `Working` transition stays unconditional (no
  counter), matching how it already behaves today — two concurrent elicitations
  were never actually possible in practice (Claude only shows one blocking
  dialog at a time), so no replacement counter is needed there either.
- **Keep the field name `active_tools` and its semantics unchanged.**
  `PostToolUseFailure` decrements it exactly like `PostToolUse` does.

## Risks / Trade-offs

- [Old `state.json` files on disk still have a `blocked_prompts` key] →
  Deserialization must ignore unknown fields (`serde`'s default behavior already
  does this as long as we don't add `#[deny(unknown_fields)]`), so no migration
  step is required; the field simply disappears from newly written files.
- [Removing `Notification` drops the only previously-existing signal for "Claude
  is waiting on a permission dialog"] → Accepted: that signal never worked
  correctly, so removing it changes nothing observable for users; it only
  removes broken, noisy `events.jsonl` entries.
- [This is a breaking change to `agentctl agent event`'s public `--type`
  argument and to `state.json`'s schema] → Both are
  `agentctl`-internal/hook-generated surfaces (not a stable public API), and
  `agentctl install claude` regenerates `hooks/hooks.json` on every run, so a
  re-run of `agentctl install claude` after upgrading `agentctl` is the only
  migration step needed for the hook side.

## Migration Plan

1. Update `AGENTCTL_CLAUDE_HOOKS` in `src/commands/hooks.rs`: remove the
   `Notification` entry and the extra unmatched `PostToolUse` entry, add the two
   `PostToolUseFailure` entries.
2. Update `agentctl agent event`'s type validation (wherever
   `EventKind`/`--type` parsing lives, `src/commands/agent_event.rs`) to drop
   `permission-prompt`.
3. Update `src/status_fsm.rs` to drop the `PermissionPrompt`-typed transition
   and the `blocked_prompts` field from its event/result types.
4. Update `src/state.rs` to drop the `blocked_prompts` field from `AgentInfo`/
   `StateFile` (keeping backward-compatible deserialization of old files that
   still contain the key).
5. Update any `agentctl list`/`agentctl info` code path that surfaces
   `blocked_prompts` (`src/commands/list.rs`) to stop populating/threading it
   through.
6. Update tests across all touched files; run the full suite via `mise run test`
   (or equivalent).
7. Users re-run `agentctl install claude` to regenerate `hooks/hooks.json`.

## Open Questions

- None outstanding; if a future need for permission-prompt-style signaling
  resurfaces, it should be scoped as its own change with a correlatable id.
