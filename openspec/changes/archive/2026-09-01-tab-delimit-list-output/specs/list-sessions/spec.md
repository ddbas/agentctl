## MODIFIED Requirements

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
   pane's `pane_current_command`, treat the pane's status as `unknown` (the
   state file belongs to a different harness that previously ran in that pane;
   do **not** delete it).
5. **Sort output** — apply the sort key and order specified by `--sort-by` and
   `--reverse` (defaulting to `updatedAt` descending) to the filtered pane list
   before rendering.
6. **Select and render columns** — render the columns specified by `--columns`,
   or the default column set (`agent`, `session`, `pane`, `status`,
   `activeTools`) when `--columns` is omitted, one line per live agent pane. A
   pane with no matching state file, or whose state file is contradictory (step
   4), displays `unknown` in the `status` column.

If tmux is unreachable the reconciliation steps above are skipped entirely (no
state files are deleted) and the command exits 0 with no output.

By default (no `--columns` given), each output line SHALL contain five
tab-delimited columns:
`<agent-name>\t<session-name>\t<window-index>.<pane-index>\t<status>\t<active-tools>`.
The `status` column SHALL display the current `AgentState` for the agent (as
lowercase string), or `unknown` if the pane has no state file or a contradictory
one. The full set of selectable columns, their names, per-column rendering rules
(including `activeTools`'s `N tool`/`N tools`/empty format and `updatedAt`'s RFC
3339 format), status-column coloring, the tab-delimiter and fixed-field-count
rules, and `--columns`-driven customization are defined by the `list-columns`
capability.

Columns SHALL be separated by a single tab character (`\t`) with no padding.
This behaviour is unchanged by the introduction of `--columns` and MUST NOT
regress; the precise, testable delimiter rules (single tab between columns, no
padding, fixed field count per row regardless of empty trailing values) are
defined by the `list-columns` capability and apply regardless of which columns
are selected.

#### Scenario: Agent panes exist across multiple sessions

- **WHEN** the user runs `agentctl list` and tmux has a pane in session `dev`
  running `claude` at window 0 pane 0, and a pane in session `work` running `pi`
  at window 1 pane 0, and other panes running `zsh` or `nvim`
- **THEN** the output contains exactly two lines — one for `claude` and one for
  `pi` — and panes running unrelated processes are omitted

#### Scenario: Default output is tab-delimited

- **WHEN** two or more matching panes are printed with no `--columns` flag
- **THEN** all five default columns (agent, session, window.pane, status,
  activeTools) are separated by a single tab character, with no padding

#### Scenario: Agent name appears in the first column by default

- **WHEN** matching panes are displayed with no `--columns` flag
- **THEN** the first column contains the agent process name (`claude` or `pi`),
  the second column contains the tmux session name, the third column contains
  the `window_index.pane_index` in the form `N.M`, the fourth column contains
  the agent status in lowercase, and the fifth column contains the active tool
  count (or is empty)

#### Scenario: No matching panes

- **WHEN** the user runs `agentctl list` and no pane's `pane_current_command`
  matches `claude` or `pi`
- **THEN** the command exits with code 0 and produces no output

#### Scenario: No tmux server running

- **WHEN** the user runs `agentctl list` and no tmux server is active
- **THEN** the command exits with code 0 and produces no output (treats absence
  of a server as zero panes)

#### Scenario: Agent has no state file

- **WHEN** the user runs `agentctl list` and a matching pane has no
  corresponding state file
- **THEN** the `status` column for that pane shows `unknown`

#### Scenario: Agent status is reflected in list output

- **WHEN** the user runs `agentctl list` and a matching pane has a valid state
  file with `"status": "working"`
- **THEN** the `status` column for that pane shows `working`

#### Scenario: Stale state file deleted when pane no longer exists

- **WHEN** the user runs `agentctl list` and a state file exists whose `pane_id`
  does not correspond to any live tmux pane
- **THEN** that state file is deleted before the output is printed

#### Scenario: Contradictory state file results in unknown status (not deleted)

- **WHEN** the user runs `agentctl list` and a state file exists whose `pane_id`
  corresponds to a live pane but the pane's current command does not match the
  `harness` field in the state file
- **THEN** the `status` column for that pane shows `unknown` and the state file
  is **not** deleted

#### Scenario: Stale validation skipped when tmux is unreachable

- **WHEN** the user runs `agentctl list` and the tmux query fails
- **THEN** no state files are deleted and the command exits with code 0 and no
  output (consistent with existing no-tmux behaviour)

#### Scenario: Session names containing wide Unicode characters render normally

- **WHEN** the user runs `agentctl list` and one session name contains an emoji
  (e.g. `🌳 agentctl (claude-hooks)`) while another contains only ASCII (e.g.
  `dotfiles`)
- **THEN** each row renders that session name verbatim as one tab-delimited
  field, with no padding or alignment applied (wide characters no longer affect
  column positioning since there is no padding to compute)

#### Scenario: Default sort puts most recently active agents first

- **WHEN** the user runs `agentctl list` without sort flags and multiple agents
  are present
- **THEN** agents are ordered by `updated_at` descending (most recently active
  first; agents with no state file appear last)

#### Scenario: Sort by agent name when flag is provided

- **WHEN** the user runs `agentctl list --sort-by agent`
- **THEN** output rows are ordered alphabetically by agent name

#### Scenario: Descending sort reverses order

- **WHEN** the user runs `agentctl list --sort-by status --reverse`
- **THEN** output rows are ordered with `unknown` first and `blocked` last
