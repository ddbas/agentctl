## MODIFIED Requirements

### Requirement: agentctl list accepts --sort-by flag

The `list` subcommand SHALL accept a `--sort-by` flag that controls which
property is used to sort the output rows. Accepted values are: `agent`,
`session`, `pane`, `status`, `updatedAt`, `activeTools`. This vocabulary SHALL
be identical (same names, same casing) to the vocabulary accepted by the
`--columns` flag. The default value when the flag is omitted SHALL be
`updatedAt`.

#### Scenario: Default sort is by updatedAt descending

- **WHEN** the user runs `agentctl list` without any sort flags
- **THEN** the output rows are ordered by the `updated_at` timestamp descending
  (most recently active agent first; panes with no state file appear last)

#### Scenario: Sort by agent name

- **WHEN** the user runs `agentctl list --sort-by agent`
- **THEN** the output rows are ordered alphabetically by agent name (e.g.
  `claude` before `pi`)

#### Scenario: Sort by session name

- **WHEN** the user runs `agentctl list --sort-by session`
- **THEN** the output rows are ordered lexicographically by tmux session name

#### Scenario: Sort by pane position

- **WHEN** the user runs `agentctl list --sort-by pane`
- **THEN** the output rows are ordered numerically: by window index first, then
  pane index within the window

#### Scenario: Sort by status

- **WHEN** the user runs `agentctl list --sort-by status`
- **THEN** the output rows are ordered by `AgentState` priority as defined by
  the canonical `AgentState` ordering

#### Scenario: Sort by updatedAt timestamp

- **WHEN** the user runs `agentctl list --sort-by updatedAt`
- **THEN** the output rows are ordered chronologically by the `updated_at` field
  in the state file; panes with no state file are treated as having the oldest
  possible timestamp (sort last in the default newest-first direction)

#### Scenario: Sort by activeTools count

- **WHEN** the user runs `agentctl list --sort-by activeTools`
- **THEN** the output rows are ordered numerically by `active_tools` count,
  descending (busiest agents first); panes with no state file or whose status is
  `unknown` are treated as having `active_tools == 0` and sort with the other
  zero-count panes

#### Scenario: Old kebab-case/state-named values are rejected

- **WHEN** the user runs `agentctl list --sort-by state` or
  `agentctl list --sort-by updated-at`
- **THEN** the command exits with a non-zero exit code and prints an error
  message listing the valid values (`agent`, `session`, `pane`, `status`,
  `updatedAt`, `activeTools`)

#### Scenario: Invalid sort-by value is rejected

- **WHEN** the user runs `agentctl list --sort-by foobar`
- **THEN** the command exits with a non-zero exit code and prints an error
  message listing valid values

### Requirement: activeTools has a descending natural sort direction

`activeTools` SHALL have a descending natural sort direction (busiest agents
first), unlike `agent`, `session`, `pane`, and `status` (whose natural direction
is ascending), matching `updatedAt`'s existing newest-first convention. Passing
`--reverse` SHALL flip this to ascending (quietest/idle agents first).

#### Scenario: Default activeTools direction is busiest first

- **WHEN** the user runs `agentctl list --sort-by activeTools` without
  `--reverse`
- **THEN** the agent with the highest `active_tools` count appears first

#### Scenario: --reverse flips activeTools to quietest first

- **WHEN** the user runs `agentctl list --sort-by activeTools --reverse`
- **THEN** the agent with the lowest `active_tools` count (including `0` for
  panes with no state file or `unknown` status) appears first

### Requirement: sort_panes helper function

The `list` module SHALL expose a `sort_panes` function that accepts a mutable
slice of `PaneEntry` references, a sort-key value (the shared `Field`/`SortKey`
enum, with `state` renamed to represent `status`, `updated-at` renamed to
`updatedAt`, and a new `activeTools` value added), a `reverse: bool`, and a
reference to the state map, and sorts in-place according to the given key and
direction. The comparison logic for the four pre-existing keys is unchanged from
prior behavior; only their CLI-facing names change to `status`/`updatedAt`
respectively. `activeTools` is new: it compares numerically by `active_tools`
count (treating panes with no state file or `unknown` status as `0`), with a
descending natural direction.

#### Scenario: sort_panes sorts by status ascending

- **WHEN** `sort_panes` is called with the status key and `reverse: false` on a
  mixed list
- **THEN** the slice is ordered by `AgentState` priority ascending (Blocked
  first)

#### Scenario: sort_panes sorts by status descending

- **WHEN** `sort_panes` is called with the status key and `reverse: true` on a
  mixed list
- **THEN** the slice is ordered by `AgentState` priority descending (Unknown
  first)

#### Scenario: sort_panes stable-sorts equal keys

- **WHEN** multiple entries share the same sort key value
- **THEN** their relative order is preserved (stable sort)

#### Scenario: sort_panes sorts by activeTools descending

- **WHEN** `sort_panes` is called with the `activeTools` key and
  `reverse: false` on a mixed list of panes with different `active_tools`
  counts, including some with no state file
- **THEN** the slice is ordered by `active_tools` count descending, with
  no-state-file panes treated as `0` and sorted alongside other zero-count panes

#### Scenario: sort_panes sorts by activeTools ascending when reversed

- **WHEN** `sort_panes` is called with the `activeTools` key and `reverse: true`
  on the same mixed list
- **THEN** the slice is ordered by `active_tools` count ascending (quietest
  first)
