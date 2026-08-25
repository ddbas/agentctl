## 1. Shared Field enum (sort-key/column vocabulary)

- [ ] 1.1 Rename `SortKey` variants' `ValueEnum`/`Display` string mappings:
      `State` → `status`, `UpdatedAt` → `updatedAt` (camelCase), keep
      `Agent`/`Session`/`Pane` as-is; update all doc comments referencing the
      old kebab-case/`state` names
- [ ] 1.2 Decide final type naming (keep `SortKey` reused for `--columns`, or
      rename to `Field`) and apply consistently across `list.rs`/`main.rs`
- [ ] 1.3 Update `--sort-by` help text and the `List` subcommand doc comment in
      `src/main.rs` to list the new accepted values
      (`agent, session, pane, status, updatedAt`)

## 2. --columns flag

- [ ] 2.1 Add `--columns`/`-c` argument to the `List` subcommand in
      `src/main.rs`, parsed as `Vec<SortKey>` (or `Vec<Field>`) via
      `value_delimiter = ','`
- [ ] 2.2 Add a `DEFAULT_COLUMNS` constant (`[Agent, Session, Pane, Status]`)
      used when `--columns` is omitted
- [ ] 2.3 Add doc comments describing `--columns` behavior, default, and
      duplicate/order-preserving semantics

## 3. Generalized row rendering

- [ ] 3.1 Rework `format_panes` to accept an ordered `&[SortKey]` (or
      `&[Field]`) column list instead of hardcoded fields
- [ ] 3.2 Implement per-field cell-value resolution: `Agent` → agent name,
      `Session` → session name, `Pane` → `window_index.pane_index`, `Status` →
      resolved `AgentState` string (`unknown` fallback), `UpdatedAt` → formatted
      timestamp or empty string
- [ ] 3.3 Preserve existing Unicode-display-width column padding logic, computed
      per column position over the generalized cell values
- [ ] 3.4 Support duplicate field entries rendering repeated columns with no
      deduplication

## 4. updatedAt formatting

- [ ] 4.1 Implement `updated_at` → RFC 3339 UTC string formatting
      (`to_rfc3339()`) for the `updatedAt` column
- [ ] 4.2 Render an empty string for `updatedAt` when no state file exists for a
      pane

## 5. Wiring and validation

- [ ] 5.1 Update `run()` in `src/commands/list.rs` to resolve the effective
      column list (`--columns` value or `DEFAULT_COLUMNS`) and pass it to
      `format_panes`
- [ ] 5.2 Confirm invalid `--columns` values produce a clap error listing valid
      values (reusing the shared enum's existing validation, no custom parsing
      needed)
- [ ] 5.3 Confirm `--sort-by state` / `--sort-by updated-at` are now rejected by
      clap (old kebab-case/`state` values no longer accepted)

## 6. Tests

- [ ] 6.1 Update existing `SortKey`/`sort_panes` unit tests in
      `src/commands/list.rs` to use `status`/`updatedAt` naming
- [ ] 6.2 Update `format_panes` unit tests for the renamed `status` column and
      the new generalized-columns signature
- [ ] 6.3 Add unit tests for `--columns` parsing: subset selection, custom
      order, duplicates, invalid column name rejection
- [ ] 6.4 Add unit tests for `updatedAt` rendering: RFC 3339 output when a state
      file exists, empty string when absent
- [ ] 6.5 Update `tests/list_sessions.rs` integration tests for the renamed
      `status` column and default column output
- [ ] 6.6 Add an integration test exercising `rd list --columns ...` end to end
      (custom subset/order) and `rd list --sort-by status`/`updatedAt`

## 7. Documentation

- [ ] 7.1 Update any README/CLI help snapshots referencing old `--sort-by`
      values or the fixed four-column output
- [ ] 7.2 Note the breaking rename (`state`→`status`, `updated-at`→`updatedAt`)
      in the change's commit message / CHANGELOG entry
