## Why

`rd list` currently always prints a fixed set of four columns (`agent`,
`session`, `pane`, `state`) in a fixed order. Users who only care about a subset
of fields (e.g. just `agent` and `status`), or who want a different column order
for scripting/eyeballing, have no way to customize the output short of piping
through `awk`/`cut`. Additionally, the already-tracked `updated_at` timestamp is
never shown, even though it's useful for spotting stale or recently-active
agents.

## What Changes

- Add a `--columns` (short: `-c`) flag to `rd list` that accepts a
  comma-delimited, ordered list of column names to display (e.g.
  `--columns agent,status,session`), replacing the fixed four-column output.
- Default columns (when `--columns` is omitted) remain
  `agent,session,pane,status` to preserve current visual behaviour, aside from
  the renamed `state` → `status` header/key.
- Add a new selectable column, `updatedAt`, which renders the agent's last
  state-file update timestamp in RFC 3339 UTC form (e.g.
  `2024-06-01T14:32:05Z`); panes with no state file render an empty value for
  this column.
- **BREAKING**: Rename the `state` column/sort-key to `status` and the
  `updated-at` sort-key to `updatedAt` so that `--sort-by` and `--columns` share
  one identical, camelCase set of key names: `agent`, `session`, `pane`,
  `status`, `updatedAt`. Existing scripts passing `--sort-by state` or
  `--sort-by updated-at` must be updated.
- Invalid column names passed to `--columns` are rejected the same way invalid
  `--sort-by` values are: non-zero exit, error message listing valid values.
- Duplicate column names in `--columns` are permitted and rendered as repeated
  columns (no deduplication), matching the flag's literal, order-preserving
  contract.

## Capabilities

### New Capabilities

- `list-columns`: Defines the `--columns`/`-c` flag, the shared column/sort-key
  vocabulary (`agent`, `session`, `pane`, `status`, `updatedAt`), per-column
  rendering rules (including the `updatedAt` RFC 3339 format), default column
  set/order, and validation/error behaviour for unknown column names.

### Modified Capabilities

- `list-sorting`: Rename the `state` sort-key to `status` and the `updated-at`
  sort-key to `updatedAt`; all `--sort-by` accepted values become camelCase and
  identical to the `--columns` vocabulary.
- `list-sessions`: Replace the fixed four-column output description with a
  reference to the configurable column set; rename the `state` column to
  `status` in the default output.

## Impact

- `src/commands/list.rs`: new `Column`/column-list parsing (likely reusing or
  extending the existing `SortKey` enum/naming), a generalized row-rendering
  function that takes an ordered column list instead of hardcoded fields, and
  `AgentInfo.updated_at` formatting for the new `updatedAt` column.
- `src/main.rs`: new `--columns`/`-c` CLI argument on the `List` subcommand;
  rename of `--sort-by` accepted value strings (`state` → `status`, `updated-at`
  → `updatedAt`) and the `SortKey` `Display`/`ValueEnum` mappings.
- `openspec/specs/list-sorting/spec.md`, `openspec/specs/list-sessions/spec.md`:
  updated requirements.
- Tests in `src/commands/list.rs` and `tests/list_sessions.rs` referencing the
  old `state`/`updated-at` names and the fixed four-column format.
- Documentation/help text (`--sort-by` doc comment, `List` subcommand doc
  comment) referencing old key names and column count.
