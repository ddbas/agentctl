## Purpose

Define the `--columns` flag for `rd list`: the vocabulary of selectable columns,
their per-column rendering rules, the default column set, status-column
coloring, and tab-delimited output with a fixed field count per row —
independent of which columns are selected.

## Requirements

### Requirement: rd list accepts --columns flag

The `list` subcommand SHALL accept a `--columns` flag (short: `-c`) whose value
is a comma-delimited, ordered list of column names. Accepted column names SHALL
be: `agent`, `session`, `pane`, `status`, `updatedAt`, `activeTools`. This
vocabulary is identical to the `--sort-by` flag's accepted values (see the
`list-sorting` capability) — every value accepted by one flag is accepted by the
other, using the same spelling and casing. The output SHALL render exactly the
columns listed, in the exact order given, including repeated column names if the
user repeats a name.

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

#### Scenario: --columns and --sort-by share one identical vocabulary

- **WHEN** a name is a valid value for `--sort-by`
- **THEN** the same name (identical spelling and casing) is also a valid value
  for `--columns`, and vice versa

#### Scenario: activeTools is valid for both --columns and --sort-by

- **WHEN** the user runs `rd list --columns activeTools`
- **THEN** the command succeeds and each row's `activeTools` column is rendered
- **WHEN** the user instead runs `rd list --sort-by activeTools`
- **THEN** the command succeeds and output rows are ordered by `active_tools`
  count (see the `list-sorting` capability)

### Requirement: Default column set when --columns is omitted

When `--columns` is not supplied, the `list` subcommand SHALL render the columns
`agent`, `session`, `pane`, `status`, `activeTools` in that order — matching the
output produced prior to the introduction of `--columns`, aside from the
`status` column's name (previously called `state`) and the `activeTools`
column's name (previously an unnamed, unconditional fifth column).

#### Scenario: Default output unchanged in content and order

- **WHEN** the user runs `rd list` without `--columns`
- **THEN** each output row contains five columns, in order: agent name, session
  name, `window.pane`, status, and active-tool count

#### Scenario: Help text shows the default column list

- **WHEN** the user runs `rd list --help`
- **THEN** the `--columns` flag's help text displays its default value as the
  literal, comma-delimited string a user would type:
  `agent,session,pane,status,activeTools`

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

### Requirement: activeTools column renders the in-flight tool-call count

The `activeTools` column SHALL render the pane's `active_tools` count from its
state file as `N tool` (singular, when `active_tools == 1`) or `N tools`
(plural, when `active_tools > 1`). When `active_tools` is `0`, absent, or the
pane's `status` is `unknown`, the `activeTools` column SHALL render as an empty
string for that row. This is unchanged, pre-existing rendering behavior,
formalized here as a named, selectable column rather than an unconditional fifth
column.

#### Scenario: Singular active tool count

- **WHEN** the user runs `rd list` (or with `activeTools` in `--columns`) and a
  pane's status is `working` with `active_tools: 1`
- **THEN** that row's `activeTools` column reads `1 tool`

#### Scenario: Plural active tool count

- **WHEN** the user runs `rd list` (or with `activeTools` in `--columns`) and a
  pane's status is `working` with `active_tools: 3`
- **THEN** that row's `activeTools` column reads `3 tools`

#### Scenario: Zero, absent, or unknown-status active tool count is empty

- **WHEN** a pane's `active_tools` is `0`, absent, or its status is `unknown`
- **THEN** that row's `activeTools` column is an empty string

### Requirement: Status column is colorized independent of its position

When standard output is a terminal, the `list` subcommand SHALL render the cell
at whichever column position holds `status` in the effective (default or
`--columns`-selected) column list wrapped in an ANSI foreground color escape
sequence keyed to the agent's state (grey for `idle`, yellow for `working`, red
for `blocked`, green for `done`, no color for `unknown`), identically regardless
of that column's position or whether `status` is repeated. Only `status` cells
are ever colorized; all other columns are always rendered as plain text. When
standard output is **not** a terminal, every column — including `status` — SHALL
be rendered as plain, uncolored text. The ANSI escape sequence wraps only the
cell's rendered text and never introduces or alters tab delimiters, so coloring
never affects the tab-separated field count of a row.

#### Scenario: Status colorized regardless of column position

- **WHEN** the user runs `rd list --columns status,agent` with stdout attached
  to a terminal and a matching pane's status is `working`
- **THEN** the first column's `working` text is wrapped in the yellow ANSI color
  escape sequence, and the second column (`agent`) is never colorized

#### Scenario: Repeated status columns are each colorized

- **WHEN** the user runs `rd list --columns status,agent,status` with stdout
  attached to a terminal
- **THEN** both `status` columns for a given row are wrapped in the same color
  escape sequence for that row's state

#### Scenario: No coloring when stdout is not a terminal

- **WHEN** the user runs `rd list` with stdout redirected to a file or piped to
  another process, regardless of `--columns`
- **THEN** no column, including `status`, contains ANSI escape sequences

### Requirement: Rows are tab-delimited with a fixed field count

The `list` subcommand SHALL separate adjacent rendered columns within a row with
a single tab character (`\t`) and SHALL NOT pad any column's value. Every
rendered row SHALL contain exactly as many tab-separated fields as the number of
columns selected (the default five, or the count given via `--columns`),
regardless of whether any column's rendered value — including a trailing
column's — is an empty string. A row whose last column is empty (e.g.
`activeTools` when zero/absent/unknown, or `updatedAt` with no state file) SHALL
still emit that column's delimiting tab, so the field count for that row matches
every other row's.

#### Scenario: Columns separated by tabs for the default column set

- **WHEN** the user runs `rd list` without `--columns` and a pane's status is
  `working` with `active_tools: 1`
- **THEN** that row reads `<agent>\t<session>\t<pane>\t<status>\t<activeTools>`
  with exactly four tab characters and no padding

#### Scenario: Trailing empty column still emits its delimiting tab

- **WHEN** the user runs `rd list --columns agent,updatedAt` and a pane has no
  state file (so `updatedAt`, the last selected column, is empty for that row)
- **THEN** that row's line is `<agent>\t` — the agent name, a tab, and an empty
  final field — not `<agent>` alone

#### Scenario: Field count is consistent across rows regardless of content

- **WHEN** the user runs `rd list` (default columns) and one row's pane is
  `working` with one active tool while another row's pane is `idle` with zero
  active tools
- **THEN** splitting each row on `\t` yields exactly five fields for both rows,
  even though the second row's `activeTools` field is an empty string

### Requirement: Tab-delimited rendering is uniform regardless of selected columns

Tab-delimited, unpadded rendering and the fixed-field-count rule SHALL apply
identically whether the default columns or a custom `--columns` selection is
used, and regardless of column order or repeated column names.

#### Scenario: Tab-delimiting holds for a custom column subset

- **WHEN** the user runs `rd list --columns session,updatedAt` and session names
  vary in display width (including wide Unicode characters)
- **THEN** each row is exactly `<session>\t<updatedAt-or-empty>` with a single
  tab separator and no padding, regardless of the session name's display width

#### Scenario: Repeated columns each get their own delimiting tab

- **WHEN** the user runs `rd list --columns agent,agent`
- **THEN** each row reads `<agent>\t<agent>` with exactly one tab between the
  two identical fields
