## Why

The Claude Code hook integration has three gaps that cause agentctl to misreport
agent state: resumed sessions appear as "unknown" instead of "idle", API errors
leave state permanently stuck at "working", and when Claude calls
`AskUserQuestion` or shows a plan approval dialog the agent appears "working"
rather than "blocked".

## What Changes

- Fix `SessionStart` matcher to fire on `resume` and `clear` sources, not just
  `startup`, so resuming a killed session correctly resets state to "idle"
- Add `StopFailure` hook so API errors (rate-limit, server errors, etc.)
  transition state to "done" instead of leaving it stuck at "working"
- Restructure `PreToolUse → working` signal to `PostToolUse → working` to
  eliminate the race condition that would occur when overriding the general hook
  for specific tools
- Add `PreToolUse(AskUserQuestion|ExitPlanMode) → blocked` so interactive
  question and plan-approval dialogs correctly report a "blocked" state
- Update `agentctl hooks install claude` to register all updated/new hook
  entries

## Capabilities

### New Capabilities

_(none — all changes are to existing capabilities)_

### Modified Capabilities

- `claude-state-hooks`: Extended requirements for session resume/clear handling,
  stop-failure state, and AskUserQuestion/ExitPlanMode blocked state;
  restructured working-state signal from PreToolUse to PostToolUse

## Impact

- `src/commands/hooks.rs`: Update `AGENTCTL_CLAUDE_HOOKS` constant — change
  `SessionStart` matcher, remove general `PreToolUse` entry, add `PostToolUse`
  entry, add `PreToolUse(AskUserQuestion)` entry, add `StopFailure` entry
- `openspec/specs/claude-state-hooks/spec.md`: Delta requirements for the new
  hook behaviours
- Tests in `src/commands/hooks.rs` must be updated to match the new hook set
  (currently asserts exactly 6 entries; will become 8)
