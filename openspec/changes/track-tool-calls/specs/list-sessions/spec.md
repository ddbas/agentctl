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
   pane's `pane_current_command`, treat the pane's state as `unknown` (the state
   file belongs to a different harness that previously ran in that pane; do
   **not** delete it).
5. **Sort output** — apply the sort key and order specified by `--sort-by` and
   `--reverse` (defaulting to `updated-at` descending) to the filtered pane list
   before rendering.
6. **Render output** — print one line per live agent pane. A pane with no
   matching state file, or whose state file is contradictory (step 4), displays
   `unknown` in the state column.

If tmux is unreachable the reconciliation steps above are skipped entirely (no
state files are deleted) and the command exits 0 with no output.

Each output line SHALL contain four left-aligned columns:
`<agent-name>  <session-name>  <window-index>.<pane-index>  <state>`. The state
column SHALL display the current `AgentState` for the agent (as lowercase
string), or `unknown` if the pane has no state file or a contradictory one. When
the pane's state file has a non-zero `active_tools` count, the state column
SHALL additionally append a single space followed by the count and the word
`tool` (singular, when `active_tools == 1`) or `tools` (plural, when
`active_tools > 1`) — e.g. `working 1 tool` or `working 3 tools`. When
`active_tools` is `0`, absent, or the state is `unknown`, no such suffix is
appended and the state column shows only the state word.

Column widths SHALL be computed using **Unicode terminal display width** (the
number of terminal columns a string occupies), not byte length or Unicode scalar
count. Each column SHALL be padded to the maximum display width in that column
plus two spaces, so that all rows are visually aligned in a fixed-width
terminal.

#### Scenario: Agent panes exist across multiple sessions

- **WHEN** the user runs `rd list` and tmux has a pane in session `dev` running
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

- **WHEN** the user runs `rd list` and no pane's `pane_current_command` matches
  `claude` or `pi`
- **THEN** the command exits with code 0 and produces no output

#### Scenario: No tmux server running

- **WHEN** the user runs `rd list` and no tmux server is active
- **THEN** the command exits with code 0 and produces no output

#### Scenario: State column shows the singular suffix for exactly one active tool

- **WHEN** a pane's state is `working` and its state file has `active_tools: 1`
- **THEN** that pane's state column reads `working 1 tool`

#### Scenario: State column shows the plural suffix for more than one active tool

- **WHEN** a pane's state is `working` and its state file has `active_tools: 3`
- **THEN** that pane's state column reads `working 3 tools`

#### Scenario: State column omits the suffix when no tools are active

- **WHEN** a pane's state is `working` and its state file has `active_tools: 0`
  (or the field is absent)
- **THEN** that pane's state column reads `working` with no tool-count suffix

#### Scenario: Unknown state never shows a tool-count suffix

- **WHEN** a pane has no matching state file (state is `unknown`)
- **THEN** that pane's state column reads `unknown` with no tool-count suffix,
  regardless of any stale `active_tools` value
