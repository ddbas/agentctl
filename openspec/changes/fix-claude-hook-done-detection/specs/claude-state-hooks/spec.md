## MODIFIED Requirements

### Requirement: Unmatched PostToolUse hook closes out permission-prompt blocks

The Claude `PostToolUse` hook SHALL additionally be registered without a tool
matcher (firing for every tool) to invoke a command that reads the hook's JSON
payload from stdin, extracts `session_id`, and invokes
`rd agent event TOOL_CALL_RESULT --id "perm-<session_id>-<nonce>" --type permission-prompt --harness claude`
(async), where `<nonce>` is a per-invocation unique value (e.g. a random number)
distinguishing this invocation's `events.jsonl` entry from other
permission-prompt events in the same session. This closes out a `blocked` status
opened by the `Notification` hook's `permission_prompt` registration,
decrementing the pane's `blocked_prompts` counter by one; `blocked` is only
cleared once `blocked_prompts` returns to `0`, so a still-pending, concurrent
permission prompt opened separately is not prematurely cleared. Unlike the
previous `--type elicitation` form, this entry does NOT mutate the pane's
`active_tools` counter (see the `agent-events` spec's `blocked_prompts`
requirement).

#### Scenario: Tool call after a resolved permission prompt clears blocked

- **WHEN** a `permission_prompt` `Notification` set the pane's status to
  `blocked` (with `blocked_prompts` `1`), the user grants permission, the tool
  executes, and Claude fires the unmatched `PostToolUse` hook for that tool call
- **THEN** the agent's state file is updated to `"status": "working"` and
  `blocked_prompts` becomes `0`

#### Scenario: Tool call with no outstanding permission prompt is a no-op for status

- **WHEN** no `permission_prompt` `Notification` is outstanding for the current
  session (`blocked_prompts` is `0`) and Claude fires the unmatched
  `PostToolUse` hook after any tool call
- **THEN** the `TOOL_CALL_RESULT` event is dispatched and logged but the agent's
  status is unchanged (`blocked_prompts` remains `0`, clamped)

#### Scenario: Resolving one of two concurrent permission prompts does not clear blocked

- **WHEN** two `permission_prompt` `Notification`s have each set the pane's
  status to `blocked` (`blocked_prompts` is `2`), and Claude fires the unmatched
  `PostToolUse` hook once (closing one of the two)
- **THEN** the agent's state file's `status` remains `"blocked"` and
  `blocked_prompts` becomes `1`

#### Scenario: Unmatched PostToolUse close does not mutate active_tools

- **WHEN** a pane's `active_tools` is `1` (a real tool call genuinely in flight)
  and Claude fires the unmatched `PostToolUse` permission-close hook
- **THEN** `active_tools` remains `1`

### Requirement: Notification hook calls rd agent event TOOL_CALL_START on permission prompts

The Claude `Notification` hook SHALL invoke a command that reads the hook's JSON
payload from stdin, extracts `session_id`, and invokes
`rd agent event TOOL_CALL_START --id "perm-<session_id>-<nonce>" --type permission-prompt --harness claude`
(async) when registered with `matcher: "permission_prompt"`, where `<nonce>` is
a per-invocation unique value (e.g. a random number) distinguishing this
invocation's `events.jsonl` entry from other permission-prompt events in the
same session. Because Claude's hook system only fires the hook when the matcher
matches, no notification-type branching logic is required inside `rd`. This
increments the pane's `blocked_prompts` counter by one (see the `agent-events`
spec) and does NOT mutate `active_tools`.

#### Scenario: permission_prompt notification fires inside tmux

- **WHEN** Claude fires the `Notification` hook with `notification_type` of
  `permission_prompt` and `$TMUX_PANE` is set
- **THEN** the agent's state file is updated to `"status": "blocked"` and
  `blocked_prompts` is incremented by one

#### Scenario: Non-permission notification is not forwarded

- **WHEN** Claude fires the `Notification` hook with any `notification_type`
  other than `permission_prompt`
- **THEN** the hook is not triggered (filtered by Claude's `matcher`) and no
  status change occurs

#### Scenario: A second concurrent permission_prompt notification opens independently

- **WHEN** a pane's status is already `blocked` with `blocked_prompts` `1`, and
  Claude fires a second `Notification` hook with `notification_type` of
  `permission_prompt` (for a different, concurrent tool awaiting approval)
- **THEN** the agent's state file's `status` remains `"blocked"` and
  `blocked_prompts` becomes `2`

### Requirement: Stop hook calls rd agent event RUN_FINISHED

The Claude `Stop` hook SHALL invoke a command that reads the hook's JSON payload
from stdin, extracts `session_id`, and invokes
`rd agent event RUN_FINISHED --id <session_id> --harness claude` (async). This
is authoritative regardless of the pane's current status: it drives the pane to
`done` from any status (`idle`, `working`, `blocked`, or `done` itself),
provided the reconciled `active_tools` count is `0` (see the `agent-events`
spec's `RUN_FINISHED` transition).

#### Scenario: Stop hook fires inside tmux

- **WHEN** Claude fires the `Stop` hook, the pane's `status` is `working` with
  `active_tools` `0`, and `$TMUX_PANE` is set
- **THEN** the agent's state file is updated to `"status": "done"`

#### Scenario: Stop hook clears a stale blocked status

- **WHEN** Claude fires the `Stop` hook, the pane's `status` is `blocked` (e.g.
  a permission-prompt block that was never explicitly cleared) with
  `active_tools` `0`, and `$TMUX_PANE` is set
- **THEN** the agent's state file is updated to `"status": "done"`

#### Scenario: Stop hook does not force done while a background task is still running

- **WHEN** Claude fires the `Stop` hook with a non-empty `background_tasks`
  array (so `--active-tools` reconciles the pane's `active_tools` to a value
  greater than `0`) and `$TMUX_PANE` is set
- **THEN** the agent's state file's `status` is left unchanged (not forced to
  `done`)

### Requirement: StopFailure hook calls rd agent event RUN_FINISHED

The Claude `StopFailure` hook SHALL invoke a command that reads the hook's JSON
payload from stdin, extracts `session_id`, and invokes
`rd agent event RUN_FINISHED --id <session_id> --harness claude` (async) so that
API errors (rate-limit, server overload, authentication failure, etc.) do not
leave status permanently stuck at "working". This is authoritative regardless of
the pane's current status, gated on the reconciled `active_tools` count being
`0`, identically to the `Stop` hook.

#### Scenario: StopFailure fires inside tmux

- **WHEN** Claude's turn ends due to an API error, firing the `StopFailure`
  hook, the pane's `active_tools` is `0`, and `$TMUX_PANE` is set
- **THEN** the agent's state file is updated to `"status": "done"`

#### Scenario: StopFailure clears a stale blocked status

- **WHEN** Claude's turn ends due to an API error while the pane's `status` is
  `blocked`, `active_tools` is `0`, and `$TMUX_PANE` is set
- **THEN** the agent's state file is updated to `"status": "done"`
