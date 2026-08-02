## Purpose

Define the sorting behaviour of the `rd list` command: available sort keys, the
`--sort-by` and `--reverse` CLI flags, and the canonical `AgentState` priority
ordering used when sorting by state.

## Requirements

### Requirement: AgentState has a canonical sort ordering

The `AgentState` type SHALL implement `Ord` and `PartialOrd` using an explicit
priority mapping: `Blocked(0) < Done(1) < Working(2) < Idle(3) < Unknown(4)`.
This ordering is independent of enum variant declaration order.

#### Scenario: Blocked is the lowest priority value

- **WHEN** `AgentState::Blocked` is compared to any other `AgentState` variant
- **THEN** `AgentState::Blocked` is less than all other variants

#### Scenario: Unknown is the highest priority value

- **WHEN** `AgentState::Unknown` is compared to any other `AgentState` variant
- **THEN** `AgentState::Unknown` is greater than all other variants

#### Scenario: Full ordering is stable

- **WHEN** all five variants are sorted in ascending order
- **THEN** the result is `[Blocked, Done, Working, Idle, Unknown]`

### Requirement: rd list accepts --sort-by flag

The `list` subcommand SHALL accept a `--sort-by` flag that controls which
property is used to sort the output rows. Accepted values are: `state`, `agent`,
`session`, `pane`, `updated-at`. The default value when the flag is omitted
SHALL be `updated-at`.

#### Scenario: Default sort is by updated-at descending

- **WHEN** the user runs `rd list` without any sort flags
- **THEN** the output rows are ordered by the `updated_at` timestamp descending
  (most recently active agent first; panes with no state file appear last)

#### Scenario: Sort by agent name

- **WHEN** the user runs `rd list --sort-by agent`
- **THEN** the output rows are ordered alphabetically by agent name (e.g.
  `claude` before `pi`)

#### Scenario: Sort by session name

- **WHEN** the user runs `rd list --sort-by session`
- **THEN** the output rows are ordered lexicographically by tmux session name

#### Scenario: Sort by pane position

- **WHEN** the user runs `rd list --sort-by pane`
- **THEN** the output rows are ordered numerically: by window index first, then
  pane index within the window

#### Scenario: Sort by updated-at timestamp

- **WHEN** the user runs `rd list --sort-by updated-at`
- **THEN** the output rows are ordered chronologically by the `updated_at` field
  in the state file; panes with no state file are treated as having the oldest
  possible timestamp (sort last in the default newest-first direction)

#### Scenario: Invalid sort-by value is rejected

- **WHEN** the user runs `rd list --sort-by foobar`
- **THEN** the command exits with a non-zero exit code and prints an error
  message listing valid values

### Requirement: rd list accepts --reverse flag

The `list` subcommand SHALL accept a `--reverse` flag (short: `-r`) that
reverses the sort order. It is a boolean flag: absent means the natural
direction for the active key, present flips it. For `updated-at` the natural
direction is newest-first; `--reverse` gives oldest-first. For all other keys
the natural direction is ascending; `--reverse` gives descending.

#### Scenario: Ascending order (default for non-time keys)

- **WHEN** the user runs `rd list --sort-by state` without `--reverse`
- **THEN** rows are sorted in ascending order (Blocked first)

#### Scenario: Descending order when --reverse is present

- **WHEN** the user runs `rd list --reverse`
- **THEN** rows are sorted in the opposite of the natural direction for the
  active sort key

### Requirement: sort_panes helper function

The `list` module SHALL expose a `sort_panes` function that accepts a mutable
slice of `PaneEntry` references, a `SortKey`, a `reverse: bool`, and a reference
to the state map, and sorts in-place according to the given key and direction.

#### Scenario: sort_panes sorts by state ascending

- **WHEN** `sort_panes` is called with `SortKey::State` and `reverse: false` on
  a mixed list
- **THEN** the slice is ordered by `AgentState` priority ascending (Blocked
  first)

#### Scenario: sort_panes sorts by state descending

- **WHEN** `sort_panes` is called with `SortKey::State` and `reverse: true` on a
  mixed list
- **THEN** the slice is ordered by `AgentState` priority descending (Unknown
  first)

#### Scenario: sort_panes stable-sorts equal keys

- **WHEN** multiple entries share the same sort key value
- **THEN** their relative order is preserved (stable sort)
