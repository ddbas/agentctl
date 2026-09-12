## 1. Shared `Field` enum (sort-key/column vocabulary)

- [x] 1.1 Rename `SortKey` to a single `Field` enum with **six** variants
      (`Agent`, `Session`, `Pane`, `Status`, `UpdatedAt`, `ActiveTools`); rename
      `ValueEnum`/`Display` string mappings: `State` → `status`, `UpdatedAt` →
      `updatedAt` (camelCase), add `ActiveTools` → `activeTools`; keep
      `Agent`/`Session`/`Pane` as-is; update all doc comments referencing the
      old kebab-case/`state` names or the four-key vocabulary
- [x] 1.2 Both `--sort-by` and `--columns` parse against this same `Field` enum,
      so their accepted values (and rejection error messages) can never drift
      apart
- [x] 1.3 Update `--sort-by` help text and the `List` subcommand doc comment in
      `src/main.rs` to list the new accepted values
      (`agent, session, pane, status, updatedAt, activeTools`)

## 2. --columns flag

- [x] 2.1 Add `--columns`/`-c` argument to the `List` subcommand in
      `src/main.rs`, parsed as `Vec<Field>` via `value_delimiter = ','`
- [x] 2.2 Add a `DEFAULT_COLUMNS` constant
      (`[Agent, Session, Pane, Status,     ActiveTools]`) used when `--columns`
      is omitted — matches today's shipped five-column output exactly
- [x] 2.3 Add doc comments describing `--columns` behavior, default, and
      duplicate/order-preserving semantics
- [x] 2.4 Ensure `--columns`/`-c` help text displays the default value as the
      literal string `agent,session,pane,status,activeTools` (derived from
      `DEFAULT_COLUMNS` so it cannot drift from the actual default)

## 3. Generalized row rendering

- [x] 3.1 Rework `format_panes` to accept an ordered `&[Field]` column list
      instead of hardcoded fields
- [x] 3.2 Implement per-field cell-value resolution: `Agent` → agent name,
      `Session` → session name, `Pane` → `window_index.pane_index`, `Status` →
      resolved `AgentState` string (`unknown` fallback), `UpdatedAt` → formatted
      timestamp or empty string, `ActiveTools` → the existing
      `N     tool`/`N tools`/empty logic (empty when `active_tools` is `0`,
      absent, or status is `unknown`) unchanged in content, now driven by column
      selection instead of being unconditional
- [x] 3.3 Preserve existing Unicode-display-width column padding logic, computed
      per column position over the generalized cell values, guaranteeing: (a)
      exactly two spaces of separation between columns, (b) each column's width
      equals the display width of its widest cell, and (c) every row's cell for
      a given column starts at the same terminal column offset
- [x] 3.4 Support duplicate field entries rendering repeated columns with no
      deduplication
- [x] 3.5 Generalize status-column coloring: apply the existing `colorize_state`
      wrapping to whichever position(s) hold `Field::Status` (including when
      duplicated), computing column width from the plain (uncolored) text first
      as today, only when `use_color` is `true`
- [x] 3.6 Replace the current hardcoded "only the fixed fifth column can omit
      its trailing delimiter" logic with a generic per-row trailing-whitespace
      trim applied after all selected columns are padded and joined; verify this
      reproduces byte-for-byte identical output for the default column set

## 4. updatedAt formatting

- [x] 4.1 Implement `updated_at` → RFC 3339 UTC string formatting
      (`to_rfc3339()`) for the `updatedAt` column
- [x] 4.2 Render an empty string for `updatedAt` when no state file exists for a
      pane

## 5. activeTools sorting

- [x] 5.1 Implement numeric comparison logic in `sort_panes` for
      `Field::ActiveTools`, treating panes with no state file or `unknown`
      status as `active_tools == 0`
- [x] 5.2 Natural direction is descending (busiest first), matching
      `UpdatedAt`'s newest-first convention; `--reverse` flips it to ascending
      (quietest/idle-first)

## 6. Wiring and validation

- [x] 6.1 Update `run()` in `src/commands/list.rs` to resolve the effective
      column list (`--columns` value or `DEFAULT_COLUMNS`) and pass it to
      `format_panes`
- [x] 6.2 Confirm invalid `--columns` values produce a clap error listing valid
      values (reusing the shared `Field` enum's existing validation, no custom
      parsing needed)
- [x] 6.3 Confirm `--sort-by state` / `--sort-by updated-at` are now rejected by
      clap (old kebab-case/`state` values no longer accepted)

## 7. Tests

- [x] 7.1 Update existing `SortKey`/`sort_panes` unit tests in
      `src/commands/list.rs` to use `status`/`updatedAt` naming
- [x] 7.2 Update `format_panes` unit tests for the renamed `status` column and
      the new generalized-columns signature; confirm the existing color tests
      (`colorize_state_*`,
      `format_panes_use_color_true_wraps_state_in_ansi_codes`,
      `format_panes_color_does_not_break_alignment`) and active-tools tests
      (`format_panes_singular_tool_column`, `format_panes_plural_tool_column`,
      `format_panes_empty_tool_column_when_zero`,
      `format_panes_unknown_state_never_shows_tool_count`,
      `format_panes_column_alignment_mixed_tool_counts`) still pass, updated
      only for naming/signature, not behavior
- [x] 7.3 Add unit tests for `--columns` parsing: subset selection, custom
      order, duplicates, invalid column name rejection
- [x] 7.4 Add unit tests for `updatedAt` rendering: RFC 3339 output when a state
      file exists, empty string when absent
- [x] 7.5 Add regression tests asserting, for both the default column set and
      arbitrary `--columns` selections: (a) columns are separated by exactly two
      spaces, (b) each column's width equals its widest cell's display width
      (e.g. `agent` column with `pi`/`claude` values is exactly as wide as
      `claude`), and (c) every row's cell for a given column starts at the same
      terminal column offset as every other row
- [x] 7.6 Update `tests/list_sessions.rs` integration tests for the renamed
      `status` column, the renamed `activeTools` column, and default column
      output
- [x] 7.7 Add an integration test exercising `agentctl list --columns ...` end
      to end (custom subset/order, including `activeTools` and `updatedAt`) and
      `agentctl     list --sort-by status`/`updatedAt`/`activeTools`
- [x] 7.8 Add unit tests for `--sort-by activeTools`: numeric ordering,
      descending-by-default (busiest first), `--reverse` giving ascending
      (quietest/idle first), and panes with no state file / `unknown` status
      sorting as `0`
- [x] 7.9 Add a test exercising status coloring combined with a custom
      `--columns` order where `status` is not in its default (fourth) position
      (e.g. `--columns status,agent`), confirming only the `status` cell is
      colorized
- [x] 7.10 Add a test exercising the generalized trailing-whitespace trim with a
      custom `--columns` selection where a non-default column (e.g. `updatedAt`)
      is last and empty for some rows

## 8. Documentation

- [x] 8.1 Update any README/CLI help snapshots referencing old `--sort-by`
      values or the fixed five-column output
- [x] 8.2 Note the breaking rename (`state`→`status`, `updated-at`→`updatedAt`)
      and the new `activeTools` sort key in the change's commit message /
      CHANGELOG entry
