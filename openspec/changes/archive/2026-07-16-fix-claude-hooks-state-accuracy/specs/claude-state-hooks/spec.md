## MODIFIED Requirements

### Requirement: SessionStart hook calls rd set-state idle

The Claude `SessionStart` hook SHALL invoke `rd set-state idle --harness claude`
when the session starts fresh, resumes from a prior run, or is cleared. The hook
SHALL be registered with `matcher: "startup|resume|clear"` so it fires for all
three sources. `rd set-state` exits 0 silently when `$TMUX_PANE` is not set.

#### Scenario: SessionStart fires on new session inside tmux

- **WHEN** Claude fires the `SessionStart` hook with source `startup` and
  `$TMUX_PANE` is set
- **THEN** the state file at `$XDG_DATA_HOME/radr/agents/$TMUX_PANE/state.json`
  is written with `"state": "idle"` and the current `updated_at` timestamp

#### Scenario: SessionStart fires on resumed session inside tmux

- **WHEN** a Claude session is killed and later resumed (via `--resume`,
  `--continue`, or `/resume`), causing `SessionStart` to fire with source
  `resume`, and `$TMUX_PANE` is set
- **THEN** the state file is written with `"state": "idle"`, replacing any
  previous state (including "unknown" from a missing file)

#### Scenario: SessionStart fires after /clear inside tmux

- **WHEN** the user runs `/clear` in Claude, causing `SessionStart` to fire with
  source `clear`, and `$TMUX_PANE` is set
- **THEN** the state file is written with `"state": "idle"`

#### Scenario: SessionStart does not fire on compaction

- **WHEN** Claude auto-compacts or the user runs `/compact`, causing
  `SessionStart` to fire with source `compact`
- **THEN** the hook does NOT fire (filtered by Claude's `matcher`) and no state
  change occurs

### Requirement: PostToolUse hook calls rd set-state working

The Claude `PostToolUse` hook SHALL invoke
`rd set-state working --harness claude` (async) after any tool completes
successfully. This replaces the former `PreToolUse` general hook as the primary
"still working" refresh signal.

#### Scenario: PostToolUse fires inside tmux

- **WHEN** Claude fires the `PostToolUse` hook for any tool and `$TMUX_PANE` is
  set
- **THEN** the agent's state file is updated to `"state": "working"`

### Requirement: PreToolUse hook calls rd set-state blocked for interactive tools

The Claude `PreToolUse` hook SHALL invoke
`rd set-state blocked --harness claude` (async) when registered with
`matcher: "AskUserQuestion|ExitPlanMode"`. This covers tools that show a
blocking interactive dialog and wait for user input before continuing.

#### Scenario: AskUserQuestion tool fires inside tmux

- **WHEN** Claude calls the `AskUserQuestion` tool and `$TMUX_PANE` is set
- **THEN** the agent's state file is updated to `"state": "blocked"` before the
  interactive dialog is shown

#### Scenario: ExitPlanMode tool fires inside tmux

- **WHEN** Claude calls the `ExitPlanMode` tool (plan approval dialog) and
  `$TMUX_PANE` is set
- **THEN** the agent's state file is updated to `"state": "blocked"` before the
  plan approval dialog is shown

#### Scenario: Non-interactive PreToolUse does not set blocked

- **WHEN** Claude calls any tool other than `AskUserQuestion` or `ExitPlanMode`
- **THEN** this hook does NOT fire (filtered by Claude's `matcher`) and no
  "blocked" state change occurs

### Requirement: StopFailure hook calls rd set-state done

The Claude `StopFailure` hook SHALL invoke `rd set-state done --harness claude`
(async) so that API errors (rate-limit, server overload, authentication failure,
etc.) do not leave state permanently stuck at "working".

#### Scenario: StopFailure fires inside tmux

- **WHEN** Claude's turn ends due to an API error, firing the `StopFailure`
  hook, and `$TMUX_PANE` is set
- **THEN** the agent's state file is updated to `"state": "done"`

### Requirement: rd hooks install claude registers handlers in user Claude settings

The `rd hooks install claude` subcommand SHALL patch `~/.claude/settings.json`
to register eight Claude hook entries using `rd set-state` and `rd clear-state`.
`SessionStart` and `SessionEnd` are registered without `async` so they complete
before Claude proceeds; all other hooks use `async: true`. The command SHALL be
idempotent and SHALL merge into any existing hook configuration without
overwriting it.

The eight entries are:

| Event              | Matcher                         | Command                                 | Async |
| ------------------ | ------------------------------- | --------------------------------------- | ----- |
| `SessionStart`     | `startup\|resume\|clear`        | `rd set-state idle --harness claude`    | no    |
| `UserPromptSubmit` | —                               | `rd set-state working --harness claude` | yes   |
| `PostToolUse`      | —                               | `rd set-state working --harness claude` | yes   |
| `PreToolUse`       | `AskUserQuestion\|ExitPlanMode` | `rd set-state blocked --harness claude` | yes   |
| `Stop`             | —                               | `rd set-state done --harness claude`    | yes   |
| `StopFailure`      | —                               | `rd set-state done --harness claude`    | yes   |
| `Notification`     | `permission_prompt`             | `rd set-state blocked --harness claude` | yes   |
| `SessionEnd`       | —                               | `rd clear-state`                        | no    |

#### Scenario: First-time install on a clean settings file

- **WHEN** the user runs `rd hooks install claude` and `~/.claude/settings.json`
  contains no radr hook entries
- **THEN** the file is updated with all eight entries listed above

#### Scenario: Idempotent re-install

- **WHEN** the user runs `rd hooks install claude` a second time
- **THEN** no duplicate entries are added and the file content is identical to
  after the first run

#### Scenario: Install merges with existing hooks

- **WHEN** `~/.claude/settings.json` already contains non-radr hook entries
- **THEN** those existing entries are preserved and the radr entries are added
  alongside them
