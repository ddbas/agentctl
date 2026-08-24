## ADDED Requirements

### Requirement: rd agent status accepts snoozed as a valid state

`rd agent status` SHALL accept `"snoozed"` as a valid state argument, parsing it
to `AgentState::Snoozed` and writing it to the pane's state file, following the
same resolution rules (pane flag, `$TMUX_PANE`, silent exit outside tmux) as the
other accepted states.

#### Scenario: rd agent status snoozed writes the snoozed state

- **WHEN** `rd agent status snoozed --harness claude --pane %12` is run
- **THEN** the state file for pane `%12` is written with `"state": "snoozed"`

### Requirement: Claude Monitor tool invocation is detected as an armed watch

The Claude hook integration SHALL detect when Claude invokes the `Monitor` tool
by registering a `PreToolUse` hook entry matched to `Monitor`, and SHALL record
that a background watch has been armed for the current pane (e.g. via a per-pane
marker file alongside the state file) so that a later `Stop` or `StopFailure`
hook can consult it.

#### Scenario: Monitor tool invocation records an armed watch

- **WHEN** Claude invokes the `Monitor` tool and the `PreToolUse` hook fires
  with `$TMUX_PANE` set
- **THEN** a marker recording an armed watch for that pane is created (or
  updated) alongside the pane's state file

### Requirement: A completed or cancelled Monitor watch clears the armed marker

The Claude hook integration SHALL clear the per-pane armed-watch marker when the
`Monitor` tool's watch ends, detected via `PostToolUse` for `Monitor` whose tool
response indicates the watch already finished, timed out, errored, or was
denied.

#### Scenario: Monitor watch ending via PostToolUse clears the marker

- **WHEN** the `PostToolUse` hook fires for a `Monitor` invocation whose
  response indicates the watch has ended
- **THEN** the armed-watch marker for that pane is removed

#### Scenario: Ambiguous Monitor tool response is treated conservatively

- **WHEN** the `PostToolUse` response for a `Monitor` invocation does not
  clearly indicate whether the watch is still running
- **THEN** the armed-watch marker is left in place (state defaults toward
  `Snoozed` rather than `Done` on the next `Stop`)

### Requirement: Stop and StopFailure set snoozed instead of done when a watch is armed

The Claude `Stop` and `StopFailure` hooks SHALL check for the per-pane
armed-watch marker before writing state. If the marker is present, the hook
SHALL invoke `rd agent status snoozed --harness claude` instead of
`rd agent status done --harness claude`. If the marker is absent, the hooks
SHALL behave exactly as before this change (writing `done`).

#### Scenario: Stop fires with an armed watch present

- **WHEN** the `Stop` hook fires for a pane that has an armed-watch marker
- **THEN** the agent's state file is updated to `"state": "snoozed"` instead of
  `"state": "done"`

#### Scenario: Stop fires with no armed watch

- **WHEN** the `Stop` hook fires for a pane with no armed-watch marker
- **THEN** the agent's state file is updated to `"state": "done"`, unchanged
  from prior behavior

#### Scenario: StopFailure fires with an armed watch present

- **WHEN** the `StopFailure` hook fires for a pane that has an armed-watch
  marker
- **THEN** the agent's state file is updated to `"state": "snoozed"` instead of
  `"state": "done"`

### Requirement: Snoozed transitions out automatically when the watch fires

The system SHALL transition state away from `Snoozed` automatically once the
armed mechanism fires and Claude resumes processing, using only the normal
Claude hook flow: `PostToolUse` and `UserPromptSubmit` write `working`, and the
subsequent `Stop` or `StopFailure` writes `done` or `snoozed` again per the
armed-marker check. No additional hook or manual intervention SHALL be required.
`Snoozed` SHALL NOT be a terminal or stuck state.

#### Scenario: Monitor event wakes Claude and resumes normal flow

- **WHEN** a pane is in `Snoozed` state, Claude's Monitor watch delivers an
  event, and Claude resumes processing (triggering `PostToolUse` for a
  subsequent tool call)
- **THEN** the pane's state file is updated to `"state": "working"`

#### Scenario: Session ends while snoozed

- **WHEN** `SessionEnd` fires for a pane that is in `Snoozed` state
- **THEN** the state file and the armed-watch marker for that pane are both
  deleted, following the same cleanup as any other state

### Requirement: rd install claude registers the Monitor detection hook entry

`rd install claude` SHALL add a `PreToolUse` hook entry matched to `Monitor`
that invokes an internal `rd` command marking the watch as armed, in addition to
the existing eight radr-managed hook entries. This entry SHALL be idempotent and
SHALL be recognized and removed by `rd uninstall claude` alongside the existing
radr-managed entries.

#### Scenario: Install adds the Monitor detection entry

- **WHEN** the user runs `rd install claude` on a clean settings file
- **THEN** `~/.claude/settings.json` includes a `PreToolUse` hook entry matched
  to `Monitor` that records an armed watch, alongside the existing eight entries

#### Scenario: Uninstall removes the Monitor detection entry

- **WHEN** the user runs `rd uninstall claude`
- **THEN** the `Monitor`-matched `PreToolUse` entry added by this change is
  removed along with the other radr-managed entries
