## Purpose

Provide a `list` subcommand that discovers and displays tmux panes running agent
harness processes (`claude`, `pi`) across all sessions, reconciling live tmux
state with persisted state files.

## Requirements

### Requirement: List agent harness panes by running process

The CLI SHALL provide a `list` subcommand that is the single point responsible
for reconciling live tmux agent sessions with persisted state files. It performs
the following steps in order:

1. **Discover live sessions** — query every pane across all tmux sessions via
   `tmux list-panes -a` and collect all panes whose `pane_current_command`
   exactly matches a known agent harness name (`claude` or `pi`).
2. **Load state files** — read every `state.json` found under the XDG data
   directory and index them by `pane_id`.
3. **Clean up orphaned state files** — for each state file whose `pane_id` does
   not correspond to any live pane in the set discovered above, delete that
   state file (the session is gone).
4. **Reconcile contradictory state** — for each live pane, if a state file
   exists for that pane's `pane_id` but its `harness` field does not match the
   pane's `pane_current_command`, treat the pane's state as `unknown` (the state
   file belongs to a different harness that previously ran in that pane; do
   **not** delete it).
5. **Render output** — print one line per live agent pane. A pane with no
   matching state file, or whose state file is contradictory (step 4), displays
   `unknown` in the state column.

If tmux is unreachable the reconciliation steps above are skipped entirely (no
state files are deleted) and the command exits 0 with no output.

Each output line SHALL contain four left-aligned columns:
`<agent-name>  <session-name>  <window-index>.<pane-index>  <state>`. The state
column SHALL display the current `AgentState` for the agent (as lowercase
string), or `unknown` if the pane has no state file or a contradictory one.

Column widths SHALL be computed using **Unicode terminal display width** (the
number of terminal columns a string occupies), not byte length or Unicode scalar
count. Each column SHALL be padded to the maximum display width in that column
plus two spaces, so that all rows are visually aligned in a fixed-width
terminal.

#### Scenario: Agent panes exist across multiple sessions

- **WHEN** the user runs `dv list` and tmux has a pane in session `dev` running
  `claude` at window 0 pane 0, and a pane in session `work` running `pi` at
  window 1 pane 0, and other panes running `zsh` or `nvim`
- **THEN** the output contains exactly two lines — one for `claude` and one for
  `pi` — and panes running unrelated processes are omitted

#### Scenario: Output is columnar and left-aligned

- **WHEN** two or more matching panes are printed
- **THEN** all four columns (agent, session, window.pane, state) are
  left-aligned, each padded to the display width of the longest value in that
  column plus two spaces, where display width is measured in terminal columns

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

- **WHEN** the user runs `dv list` and a matching pane has no corresponding
  state file
- **THEN** the state column for that pane shows `unknown`

#### Scenario: Agent state is reflected in list output

- **WHEN** the user runs `dv list` and a matching pane has a valid state file
  with `"state": "working"`
- **THEN** the state column for that pane shows `working`

#### Scenario: Stale state file deleted when pane no longer exists

- **WHEN** the user runs `dv list` and a state file exists whose `pane_id` does
  not correspond to any live tmux pane
- **THEN** that state file is deleted before the output is printed

#### Scenario: Contradictory state file results in unknown state (not deleted)

- **WHEN** the user runs `dv list` and a state file exists whose `pane_id`
  corresponds to a live pane but the pane's current command does not match the
  `harness` field in the state file
- **THEN** the state column for that pane shows `unknown` and the state file is
  **not** deleted

#### Scenario: Stale validation skipped when tmux is unreachable

- **WHEN** the user runs `dv list` and the tmux query fails
- **THEN** no state files are deleted and the command exits with code 0 and no
  output (consistent with existing no-tmux behaviour)

#### Scenario: Columns align correctly when session name contains wide Unicode characters

- **WHEN** the user runs `dv list` and one session name contains an emoji (e.g.
  `🌳 drovr (claude-hooks)`) while another contains only ASCII (e.g. `dotfiles`)
- **THEN** the state column for every row starts at the same terminal column
  position, despite the emoji occupying two terminal columns
