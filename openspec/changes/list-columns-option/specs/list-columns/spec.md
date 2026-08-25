## ADDED Requirements

### Requirement: rd list accepts --columns flag

The `list` subcommand SHALL accept a `--columns` flag (short: `-c`) whose value
is a comma-delimited, ordered list of column names. Accepted column names SHALL
be exactly the same vocabulary, spelling, and casing as the `--sort-by` flag's
accepted values: `agent`, `session`, `pane`, `status`, `updatedAt`. The output
SHALL render exactly the columns listed, in the exact order given, including
repeated column names if the user repeats a name.

#### Scenario: Custom column subset and order

- **WHEN** the user runs `rd list --columns status,agent`
- **THEN** each output row contains exactly two columns, in order: the agent's
  status, then the agent name

#### Scenario: Single column

- **WHEN** the user runs `rd list --columns agent`
- **THEN** each output row contains exactly one column: the agent name

#### Scenario: Repeated column name is rendered twice

- **WHEN** the user runs `rd list --columns agent,agent`
- **THEN** each output row contains the agent name twice, once per occurrence,
  with no deduplication

#### Scenario: Invalid column name is rejected

- **WHEN** the user runs `rd list --columns bogus`
- **THEN** the command exits with a non-zero exit code and prints an error
  message listing the valid column names

#### Scenario: --columns and --sort-by share one vocabulary

- **WHEN** a name is a valid value for `--sort-by`
- **THEN** the same name (identical spelling and casing) is also a valid value
  for `--columns`, and vice versa

### Requirement: Default column set when --columns is omitted

When `--columns` is not supplied, the `list` subcommand SHALL render the columns
`agent`, `session`, `pane`, `status` in that order — matching the output
produced prior to the introduction of `--columns`, aside from the `status`
column's name (previously called `state`).

#### Scenario: Default output unchanged in content and order

- **WHEN** the user runs `rd list` without `--columns`
- **THEN** each output row contains four columns, in order: agent name, session
  name, `window.pane`, and status

### Requirement: updatedAt column renders a timestamp

The `updatedAt` column SHALL render the agent's `updated_at` timestamp (from its
state file) formatted as RFC 3339 in UTC (e.g. `2024-06-01T14:32:05+00:00`).
When the pane has no corresponding state file, the `updatedAt` column SHALL
render as an empty string for that row.

#### Scenario: updatedAt shows RFC 3339 timestamp when state file exists

- **WHEN** the user runs `rd list --columns agent,updatedAt` and a matching pane
  has a state file with `updated_at` set to a known instant
- **THEN** the `updatedAt` column for that row shows the RFC 3339 representation
  (UTC) of that instant

#### Scenario: updatedAt is empty when no state file exists

- **WHEN** the user runs `rd list --columns agent,updatedAt` and a matching pane
  has no state file
- **THEN** the `updatedAt` column for that row is an empty string

### Requirement: Column alignment applies uniformly regardless of selected columns

Column widths SHALL continue to be computed using Unicode terminal display width
(not byte length or scalar count), padded per-column to the maximum display
width in that column plus two spaces, regardless of which columns or how many
are selected via `--columns`.

#### Scenario: Alignment holds for a custom column subset

- **WHEN** the user runs `rd list --columns session,updatedAt` and session names
  vary in display width (including wide Unicode characters)
- **THEN** the `updatedAt` column starts at the same terminal column position on
  every row
