## MODIFIED Requirements

### Requirement: rd list accepts --sort-by flag

The `list` subcommand SHALL accept a `--sort-by` flag that controls which
property is used to sort the output rows. Accepted values are: `agent`,
`session`, `pane`, `status`, `updatedAt`. This vocabulary SHALL use the same
names and casing as the corresponding subset of the `--columns` flag's
vocabulary; unlike `--columns`, `--sort-by` does **not** accept `activeTools`
(see the dedicated requirement below). The default value when the flag is
omitted SHALL be `updatedAt`.

#### Scenario: Default sort is by updatedAt descending

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

#### Scenario: Sort by status

- **WHEN** the user runs `rd list --sort-by status`
- **THEN** the output rows are ordered by `AgentState` priority as defined by
  the canonical `AgentState` ordering

#### Scenario: Sort by updatedAt timestamp

- **WHEN** the user runs `rd list --sort-by updatedAt`
- **THEN** the output rows are ordered chronologically by the `updated_at` field
  in the state file; panes with no state file are treated as having the oldest
  possible timestamp (sort last in the default newest-first direction)

#### Scenario: Old kebab-case/state-named values are rejected

- **WHEN** the user runs `rd list --sort-by state` or
  `rd list --sort-by updated-at`
- **THEN** the command exits with a non-zero exit code and prints an error
  message listing the valid values (`agent`, `session`, `pane`, `status`,
  `updatedAt`)

#### Scenario: Invalid sort-by value is rejected

- **WHEN** the user runs `rd list --sort-by foobar`
- **THEN** the command exits with a non-zero exit code and prints an error
  message listing valid values

### Requirement: activeTools is not a valid --sort-by value

`--sort-by` SHALL NOT accept `activeTools` as a value, even though it is a valid
`--columns` value (see the `list-columns` capability). This is a deliberate,
documented divergence between the two flags' vocabularies: no requirement
defines a sort ordering for the in-flight tool-call count, so `--sort-by`'s
vocabulary remains the five values above.

#### Scenario: --sort-by activeTools is rejected

- **WHEN** the user runs `rd list --sort-by activeTools`
- **THEN** the command exits with a non-zero exit code and prints an error
  message listing only the five valid `--sort-by` values (`agent`, `session`,
  `pane`, `status`, `updatedAt`) — `activeTools` is not listed as accepted

### Requirement: sort_panes helper function

The `list` module SHALL expose a `sort_panes` function that accepts a mutable
slice of `PaneEntry` references, a sort-key value (renamed from `SortKey::State`
to represent `status`, and `SortKey::UpdatedAt`'s string mapping renamed to
`updatedAt`), a `reverse: bool`, and a reference to the state map, and sorts
in-place according to the given key and direction. The comparison logic for each
key is unchanged from prior behavior; only the CLI-facing name of the
`state`/`updated-at` keys changes to `status`/`updatedAt` respectively.

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
