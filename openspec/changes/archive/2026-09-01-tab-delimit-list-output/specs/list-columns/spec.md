## ADDED Requirements

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

- **WHEN** the user runs `agentctl list` without `--columns` and a pane's status
  is `working` with `active_tools: 1`
- **THEN** that row reads `<agent>\t<session>\t<pane>\t<status>\t<activeTools>`
  with exactly four tab characters and no padding

#### Scenario: Trailing empty column still emits its delimiting tab

- **WHEN** the user runs `agentctl list --columns agent,updatedAt` and a pane
  has no state file (so `updatedAt`, the last selected column, is empty for that
  row)
- **THEN** that row's line is `<agent>\t` — the agent name, a tab, and an empty
  final field — not `<agent>` alone

#### Scenario: Field count is consistent across rows regardless of content

- **WHEN** the user runs `agentctl list` (default columns) and one row's pane is
  `working` with one active tool while another row's pane is `idle` with zero
  active tools
- **THEN** splitting each row on `\t` yields exactly five fields for both rows,
  even though the second row's `activeTools` field is an empty string

### Requirement: Tab-delimited rendering is uniform regardless of selected columns

Tab-delimited, unpadded rendering and the fixed-field-count rule SHALL apply
identically whether the default columns or a custom `--columns` selection is
used, and regardless of column order or repeated column names.

#### Scenario: Tab-delimiting holds for a custom column subset

- **WHEN** the user runs `agentctl list --columns session,updatedAt` and session
  names vary in display width (including wide Unicode characters)
- **THEN** each row is exactly `<session>\t<updatedAt-or-empty>` with a single
  tab separator and no padding, regardless of the session name's display width

#### Scenario: Repeated columns each get their own delimiting tab

- **WHEN** the user runs `agentctl list --columns agent,agent`
- **THEN** each row reads `<agent>\t<agent>` with exactly one tab between the
  two identical fields

## REMOVED Requirements

### Requirement: Trailing whitespace is trimmed from each rendered row

**Reason**: This requirement only made sense under space-padded, aligned output,
where a trailing empty column would otherwise leave dangling padding spaces at
the end of a line. Tab-delimited output has no padding to trim; the "Rows are
tab-delimited with a fixed field count" requirement replaces it and explicitly
requires the opposite behavior for a trailing empty column — the delimiting tab
(and the empty field it introduces) MUST be kept, not trimmed, so every row has
the same field count.

**Migration**: Callers that previously relied on a trailing empty column being
trimmed away should instead split each row on `\t` and expect the column count
to always equal the number of columns requested; a trailing empty field is a
valid, expected value rather than an omission.

### Requirement: Column alignment applies uniformly regardless of selected columns

**Reason**: Column alignment (Unicode-display-width column widths, two-space
separation, fixed start position per column) was a visual-readability feature of
the previous space-padded output. It is superseded by the "Tab-delimited
rendering is uniform regardless of selected columns" requirement, which
optimizes for machine parseability instead. There is no column-width computation
or Unicode-display-width dependency in the new implementation.

**Migration**: Callers or users who relied on visually aligned columns in a
terminal can restore alignment locally by piping through a formatter, e.g.
`agentctl list | column -t -s $'\t'`.
