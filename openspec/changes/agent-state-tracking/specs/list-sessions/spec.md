## MODIFIED Requirements

### Requirement: List agent harness panes by running process

The CLI SHALL provide a `list` subcommand that queries every pane across all
tmux sessions via `tmux list-panes -a` and prints one line for each pane whose
`pane_current_command` exactly matches a known agent harness name (`claude` or
`pi`). Each line SHALL contain four left-aligned columns:
`<agent-name>  <session-name>  <window-index>.<pane-index>  <state>`. The state
column SHALL display the current `AgentState` for the agent (as lowercase
string), or `unknown` if no state file exists for that pane's `DROVR_AGENT_ID`.

#### Scenario: Agent panes exist across multiple sessions

- **WHEN** the user runs `dv list` and tmux has a pane in session `dev` running
  `claude` at window 0 pane 0, and a pane in session `work` running `pi` at
  window 1 pane 0, and other panes running `zsh` or `nvim`
- **THEN** the output contains exactly two lines — one for `claude` and one for
  `pi` — and panes running unrelated processes are omitted

#### Scenario: Output is columnar and left-aligned

- **WHEN** two or more matching panes are printed
- **THEN** all four columns (agent, session, window.pane, state) are
  left-aligned, each padded to the width of the longest value in that column
  plus two spaces

#### Scenario: Agent name appears in the first column

- **WHEN** matching panes are displayed
- **THEN** the first column contains the agent process name (`claude` or `pi`),
  the second column contains the tmux session name, the third column contains
  the `window_index.pane_index` in the form `N.M`, and the fourth column
  contains the agent state in lowercase

#### Scenario: No matching panes

- **WHEN** the user runs `dv list` and no pane's `pane_current_command` matches
  `claude` or `pi`
- **THEN** the command exits with code 0 and produces no output

#### Scenario: No tmux server running

- **WHEN** the user runs `dv list` and no tmux server is active
- **THEN** the command exits with code 0 and produces no output (treats absence
  of a server as zero panes)

#### Scenario: Agent has no state file

- **WHEN** the user runs `dv list` and a matching pane has no `DROVR_AGENT_ID`
  set or has no corresponding state file
- **THEN** the state column for that pane shows `unknown`

#### Scenario: Stale state files are cleaned up on list

- **WHEN** the user runs `dv list` and a state file exists for an agent ID that
  is not present in any live tmux pane's `DROVR_AGENT_ID`
- **THEN** that state file is deleted before the output is printed

#### Scenario: Cleanup skipped when tmux is unreachable

- **WHEN** the user runs `dv list` and the tmux query fails
- **THEN** no state files are deleted and the command exits with code 0 and no
  output (consistent with existing no-tmux behaviour)
