## Why

`agentctl list` currently renders columns as space-padded, visually-aligned text
(each column padded to its widest cell plus two spaces). This is convenient to
read at a glance but hostile to scripting: splitting on whitespace breaks
whenever a value itself contains spaces (session names commonly do, e.g.
`work session`), and there is no unambiguous, single-character field separator a
caller can rely on. The requirement for this change is to delimit
`agentctl list`'s columns with tab characters instead, so the output is reliably
machine-parseable (e.g. with `cut -f`, `awk -F'\t'`) while remaining readable as
plain text.

## What Changes

- **BREAKING**: `agentctl list` output no longer pads columns to a shared width
  for visual alignment. Columns are separated by a single tab character (`\t`)
  with no padding.
- Every rendered row has exactly `columns.len() - 1` tab separators (i.e. a
  fixed field count equal to the number of selected columns), even when a
  trailing column's value is empty (e.g. an idle `activeTools` cell or a missing
  `updatedAt` timestamp). This is a correctness fix over the initial prototype
  of this change, which trimmed trailing whitespace from each row and thereby
  silently dropped the trailing tab (and the empty field it delimits) whenever
  the last rendered column was empty — producing rows with an inconsistent
  number of tab-separated fields, which defeats the purpose of using a fixed
  delimiter for machine parsing.
- The Unicode-display-width column-alignment logic (and its `unicode-width`
  dependency) is removed, since there is no more padding to align.
- Status-column ANSI colorization (when stdout is a terminal) is unchanged in
  behavior, only in the surrounding delimiter.
- Documentation (module doc comment, `--help` text, `README.md`) is updated to
  describe the tab-delimited output format.

## Capabilities

### New Capabilities

_(none)_

### Modified Capabilities

- `list-sessions`: the default-output requirement is updated from "five
  left-aligned columns" separated by padding to "five tab-delimited columns"
  with no padding; the Unicode-display-width alignment requirement is removed
  (no longer applicable).
- `list-columns`: the column-alignment requirement (two-space separation, shared
  column width, fixed start position) is replaced with a tab-delimiter
  requirement (single tab between columns, no padding, fixed field count per
  row); the trailing-whitespace-trimming requirement is replaced with a
  fixed-field-count requirement (empty trailing columns keep their delimiting
  tab rather than being trimmed away).

## Impact

- **Code**: `src/commands/list.rs` (`format_panes`), `src/main.rs` (`--help`
  text)
- **Dependencies**: removes `unicode-width` from `Cargo.toml` (no longer used)
- **Docs**: `README.md`
- **Tests**: existing unit tests in `list.rs` updated to assert tab-delimited
  output with fixed field counts, including for rows with empty trailing columns
- **Breaking change**: any script or human relying on the previous space-padded,
  visually-aligned columns must be updated to split on `\t` instead of
  whitespace
