## Context

`rd list` renders one line per tmux pane, with columns selected via
`--columns`/`-c` (defaulting to `agent,session,pane,status,activeTools`). The
formatting logic lives in `format_panes` in `src/commands/list.rs`.

Historically (and up through the initial tab-delimiter prototype on this branch)
`format_panes` computed each column's Unicode display width across all rows,
padded every cell to its column's width plus two spaces, joined cells directly,
and then called `.trim_end()` on the assembled line to strip the trailing
padding spaces that the last column's cell would otherwise carry.

The prototype swapped space-padding for a literal `\t` between columns but kept
the `.trim_end()` call. That call does more than strip trailing whitespace left
over from padding — it also strips a genuinely-empty last column's rendered
value and, critically, the tab character immediately preceding it. `activeTools`
(idle/zero) and `updatedAt` (no state file) both legitimately render as an empty
string. Any row where one of these is the _last_ selected column loses its
trailing tab, while other rows in the same output (where that column is
non-empty) keep it. This makes the tab-delimited output inconsistent in field
count across rows, which defeats the purpose of using tabs as a fixed,
machine-parseable delimiter in the first place — the prototype's own test suite
encoded this as expected behavior (e.g.
`format_panes_updated_at_empty_when_no_state_file` asserted `"claude\n"` instead
of `"claude\t\n"`).

## Goals / Non-Goals

**Goals:**

- `rd list` output uses a single tab character (`\t`) as the sole column
  delimiter, with no padding.
- Every output row has exactly `columns.len()` tab-separated fields — including
  when a trailing column's rendered value is empty — so tools like `cut -f`,
  `awk -F'\t'`, or a naive `split("\t")` see a consistent field count regardless
  of row content.
- Status-column ANSI colorization continues to work identically, independent of
  the delimiter change.
- Remove the now-unused `unicode-width` dependency and its associated
  column-width computation.

**Non-Goals:**

- Adding a header row, a `--format json` mode, or any other structural change to
  `rd list` output beyond the delimiter itself.
- Changing which columns are selectable, their names, or their per-column
  rendering rules (`activeTools`'s `N tool`/`N tools`/empty format,
  `updatedAt`'s RFC 3339 format, etc.) — only the delimiter and the join/trim
  mechanics change.
- Preserving human-readability via visual alignment; tab-delimited output is
  optimized for machine parsing, and readability in a terminal is a secondary
  concern (a user piping to `column -t` or similar regains alignment).

## Decisions

### Decision: Join columns with `\t` directly; never trim the assembled line

**Chosen**: build a `Vec<String>` of rendered cells per row (colorizing the
`status` cell in place) and join with `"\t"` via `.join("\t")`. No padding-width
computation, no `.trim_end()` on the result.

**Rationale**: This is the simplest implementation that satisfies the
fixed-field-count goal — `Vec::join` always inserts exactly `n - 1` separators
for `n` elements, regardless of whether any element is empty. Trimming was only
ever needed to strip _padding_, which no longer exists; carrying it over to the
tab-delimited version was the prototype's core bug.

**Alternatives considered**:

- _Keep `.trim_end()` but only trim actual whitespace characters, not tabs_:
  fragile — still requires reasoning about which trailing characters are "real"
  content vs. delimiter, and provides no benefit over not trimming at all once
  padding is gone.
- _Special-case empty trailing cells to omit the column entirely_: would require
  callers to know how many columns were requested and diff against the actual
  field count received, which is strictly worse for machine parsing than a fixed
  field count.

### Decision: Remove the `unicode-width` dependency and column-width logic

**Chosen**: delete the per-column width computation loop and the `unicode-width`
crate dependency entirely.

**Rationale**: Display-width-aware padding existed solely to keep
visually-aligned columns correctly aligned in the presence of wide Unicode
characters (emoji, CJK, etc.) in cell values. With no padding, there is nothing
to align, so the dependency and the logic computing it are dead weight.

## Risks / Trade-offs

- **Breaking change for existing consumers**: any script or human relying on the
  previous space-padded, human-aligned columns must switch to splitting on `\t`.
  → Documented as a breaking change in the proposal and README; `rd list` has no
  stability guarantee predating this project's early stage, and the fix restores
  intended, well-defined behavior rather than introducing a new one.
- **Terminal readability regresses slightly** for interactive use without
  padding. → Users who want aligned output can pipe through `column -t -s $'\t'`
  or similar; this is standard practice for tab-delimited CLI output (cf.
  `git log --format`, `docker ps --format`).
