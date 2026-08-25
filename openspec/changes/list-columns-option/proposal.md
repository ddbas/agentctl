## Why

`rd list` currently always prints a fixed set of five columns (`agent`,
`session`, `pane`, `state`, and an active-tool-count column) in a fixed order,
with the `state` column additionally colorized by ANSI escape codes when stdout
is a terminal. Users who only care about a subset of fields (e.g. just `agent`
and `status`), or who want a different column order for scripting/eyeballing,
have no way to customize the output short of piping through `awk`/`cut`.
Additionally, the already-tracked `updated_at` timestamp is never shown, even
though it's useful for spotting stale or recently-active agents.

> **Note:** this proposal was originally written against a fixed four-column
> `rd list` (no active-tool-count column, no colorized output). Two other
> changes (`track-tool-calls`, `colorize rd list state column`) have since
> merged to `main` and shipped a fifth `active_tools` column and TTY-only status
> coloring. This revision updates the proposal, design, tasks, and specs to
> account for both, so `--columns` generalizes the **current** five-column,
> color-aware renderer rather than the four-column one this proposal originally
> targeted.

## What Changes

- Add a `--columns` (short: `-c`) flag to `rd list` that accepts a
  comma-delimited, ordered list of column names to display (e.g.
  `--columns agent,status,session`), replacing the fixed five-column output.
- Default columns (when `--columns` is omitted) remain
  `agent,session,pane,status,activeTools` to preserve current visual behaviour
  exactly, aside from the renamed `state` → `status` and `active-tools` →
  `activeTools` names.
- The `--columns` help text SHALL display the default value as the literal,
  comma-delimited string a user would type:
  `agent,session,pane,status,activeTools`.
- Add a new selectable column, `updatedAt`, which renders the agent's last
  state-file update timestamp in RFC 3339 UTC form (e.g.
  `2024-06-01T14:32:05Z`); panes with no state file render an empty value for
  this column.
- Add `activeTools` as a formally-named, selectable `--columns` value for the
  already-shipped active-tool-count column (previously an unconditional fifth
  column with no name of its own). Rendering is unchanged from today: `N tool`
  (singular), `N tools` (plural), or empty when `active_tools` is `0`, absent,
  or the pane's status is `unknown`.
- **`activeTools` is a `--columns`-only value; it is intentionally NOT added to
  `--sort-by`'s vocabulary.** `--columns` and `--sort-by` therefore no longer
  share one identical vocabulary; `--columns` accepts a strict superset
  (`agent`, `session`, `pane`, `status`, `updatedAt`, `activeTools`) of what
  `--sort-by` accepts (`agent`, `session`, `pane`, `status`, `updatedAt`). No
  existing requirement or shipped behavior calls for sorting by concurrent tool
  count, so adding it is out of scope for this change; `--sort-by activeTools`
  is rejected the same way any other unrecognized value is.
- **BREAKING**: Rename the `state` column/sort-key to `status` and the
  `updated-at` sort-key to `updatedAt` so that `--sort-by` and `--columns` use
  one consistent, camelCase spelling for every key they do share. Existing
  scripts passing `--sort-by state` or `--sort-by updated-at` must be updated.
- Invalid column names passed to `--columns` are rejected the same way invalid
  `--sort-by` values are: non-zero exit, error message listing valid values.
- Duplicate column names in `--columns` are permitted and rendered as repeated
  columns (no deduplication), matching the flag's literal, order-preserving
  contract.
- The existing TTY-only ANSI coloring of the status column (grey/yellow/red/
  green/uncolored keyed to agent state) is preserved and generalized: whichever
  position `status` occupies in the selected `--columns` list (including when
  duplicated) is colorized the same way; columns other than `status` are never
  colorized.
- The existing "no dangling trailing whitespace" behaviour (today hardcoded to
  the fixed fifth column) is generalized to a plain **per-row trailing
  whitespace trim**: after all selected columns are padded and joined, each
  rendered line has any trailing whitespace removed. This reproduces today's
  behavior exactly for the default columns and extends it to any `--columns`
  selection where the last selected column happens to be empty for a given row
  (e.g. `updatedAt` or `activeTools` placed last).
- **No regression** to existing column alignment behaviour, now made an
  explicit, strict requirement regardless of which columns are selected: (1)
  columns are always separated by at least two spaces, (2) each column's width
  always equals the display width of its widest cell (e.g. an `agent` column
  with only `pi`/`claude` is exactly as wide as `claude`), and (3) every row's
  cell for a given column always starts at the same terminal column position as
  every other row's cell for that column.

## Capabilities

### New Capabilities

- `list-columns`: Defines the `--columns`/`-c` flag, the full column vocabulary
  (`agent`, `session`, `pane`, `status`, `updatedAt`, `activeTools` — a strict
  superset of the `--sort-by` vocabulary), per-column rendering rules (including
  the `updatedAt` RFC 3339 format and the existing `activeTools` N/tool(s)/empty
  format), the generalized status-column coloring rule, the generalized per-row
  trailing-whitespace trim, default column set/order, and validation/error
  behaviour for unknown column names.

### Modified Capabilities

- `list-sorting`: Rename the `state` sort-key to `status` and the `updated-at`
  sort-key to `updatedAt`; all `--sort-by` accepted values become camelCase.
  Document explicitly that `activeTools` is a valid `--columns` value but not a
  valid `--sort-by` value.
- `list-sessions`: Replace the fixed five-column output description with a
  reference to the configurable column set defined by `list-columns`; rename the
  `state` column to `status` and the unnamed fifth column to `activeTools` in
  the default output; move the detailed per-column rendering/coloring/ alignment
  rules to `list-columns` to avoid duplication.

## Impact

- `src/commands/list.rs`: new `Column` enum (superset of `SortKey`, adding
  `ActiveTools`) and column-list parsing; a generalized row-rendering function
  that takes an ordered `&[Column]` list instead of hardcoded fields, threading
  through the existing `colorize_state` call (applied only to `Column::Status`
  cells, wherever they occur) and the existing `active_tools` N/tool(s)/empty
  formatting (now driven by `Column::ActiveTools` instead of being
  unconditional); `AgentInfo.updated_at` formatting for the new `updatedAt`
  column; a generalized per-row trailing-whitespace trim replacing the current
  hardcoded "only trim after the fixed fifth column" logic.
- `src/main.rs`: new `--columns`/`-c` CLI argument on the `List` subcommand;
  rename of `--sort-by` accepted value strings (`state` → `status`, `updated-at`
  → `updatedAt`) and the `SortKey` `Display`/`ValueEnum` mappings; updated
  `List` subcommand doc comment (no longer claims a fixed "four left-aligned
  columns").
- `openspec/specs/list-sorting/spec.md`, `openspec/specs/list-sessions/spec.md`:
  updated requirements.
- Tests in `src/commands/list.rs` and `tests/list_sessions.rs` referencing the
  old `state`/`updated-at` names, the fixed five-column format, and the existing
  color/active-tools tests (which must keep passing unchanged in content, only
  reachable via the new generalized code path).
- Documentation/help text (`--sort-by` doc comment, `List` subcommand doc
  comment) referencing old key names and column count.
