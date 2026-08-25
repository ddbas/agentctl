## Context

`rd list` (`src/commands/list.rs`) currently hardcodes both the sort keys
(`SortKey` enum: `Agent`, `Session`, `Pane`, `State`, `UpdatedAt`, with kebab
`Display`/`ValueEnum` strings) and the rendered output columns (`format_panes`
always emits `agent, session, window.pane, state` in that fixed order).
`AgentInfo.updated_at` (a `chrono::DateTime<Utc>`) is already loaded into the
`state_map` used for sorting but is never rendered.

This change (1) introduces a `--columns`/`-c` flag so users choose which fields
appear and in what order, and (2) unifies the vocabulary used by `--sort-by` and
`--columns` into one shared, camelCase key set: `agent`, `session`, `pane`,
`status`, `updatedAt`.

## Goals / Non-Goals

**Goals:**

- One shared enum/key-set drives both `--sort-by` and `--columns` so the two
  flags can never drift out of sync.
- `--columns` is order-preserving and duplicate-preserving: whatever the user
  types is rendered literally, left to right.
- Default output (no `--columns` given) stays visually identical to today's
  output except for the `state` → `status` rename.
- Adding `updatedAt` as a column does not change default behavior (it's opt-in
  only).
- Reuse the existing Unicode-width-aware column alignment logic (`format_panes`)
  rather than duplicating padding logic per column.

**Non-Goals:**

- No change to sorting _semantics_ (comparison logic, natural directions,
  stability) — only the string keys used to select a sort key are renamed.
- No support for column-specific formatting flags (e.g. relative time vs.
  absolute time for `updatedAt`); a single fixed RFC 3339 format is used.
- No persistent/config-file default for `--columns` (CLI flag only, this
  session).
- No renaming of the underlying `AgentState` enum variants or JSON serialization
  keys in state files — only the CLI-facing sort/column key strings change.

## Decisions

### 1. One shared `Field` enum replaces `SortKey` for key parsing

Rename `SortKey` to a single `Field` enum (or keep the name `SortKey` but reuse
it as the `--columns` value type too — final naming left to implementation) with
variants `Agent`, `Session`, `Pane`, `Status`, `UpdatedAt`, each with a
`clap::ValueEnum` mapping to the camelCase string (`agent`, `session`, `pane`,
`status`, `updatedAt`). Both `--sort-by` and `--columns` parse against this same
enum, guaranteeing identical accepted values and identical error messages for
invalid input.

**Alternative considered**: keep `SortKey` and add a separate `Column` enum with
its own string mapping. Rejected — this risks the two vocabularies silently
diverging over time (exactly the problem the user asked to avoid), and
duplicates the `ValueEnum`/`Display` boilerplate.

### 2. `--columns` value is `Vec<Field>` via a custom clap parser

Clap's `value_delimiter = ','` combined with `value_enum` on a
`Vec<Field>`-typed field parses `--columns agent,status,session` directly into
`vec![Field::Agent, Field::Status, Field::Session]`, reusing per-token
`ValueEnum` parsing (and its rejection message) with no hand-rolled string
splitting.

### 3. `format_panes` takes an ordered `&[Field]` instead of being column-fixed

Generalize row rendering: for each pane, build a `Vec<String>` of cell values by
mapping over the requested `Field` list (`Field::Agent` → `pane.agent`,
`Field::Pane` → `format!("{}.{}", window_index, pane_index)`, `Field::Status` →
resolved `AgentState` string, `Field::UpdatedAt` → formatted timestamp or empty
string). Column widths are computed per position from this same per-row
`Vec<String>` set (max width in that position + 2), preserving the existing
Unicode-width alignment logic. Duplicate fields in the list simply produce
duplicate cell computations at their respective positions — no special-casing
needed.

This preserves, unchanged, three pre-existing alignment guarantees that MUST NOT
regress regardless of which/how many columns are selected: (a) exactly two
spaces of separation between columns, (b) each column's width equal to the
Unicode display width of its widest cell in that column (no more, no less), and
(c) every row's cell for a given column position starting at the same terminal
column offset as every other row. These three properties are already covered by
the existing `format_panes_left_aligned_correct_widths` and
`format_panes_aligns_columns_with_wide_unicode` tests for the fixed four-column
case; equivalent tests MUST be added for arbitrary `--columns` selections (see
Tasks) to guard against regression during the generalization.

### 4. `updatedAt` renders as RFC 3339 in UTC, empty when absent

`AgentInfo.updated_at: DateTime<Utc>` is formatted with `to_rfc3339()` (e.g.
`2024-06-01T14:32:05+00:00`) — chrono's standard, unambiguous, sortable-as-text
format. Panes with no state file render an empty string in that column position
(consistent with how `status` renders `unknown` in that case — `unknown` still
applies to `status`, only `updatedAt` has no meaningful "unknown" timestamp so
empty is used instead).

**Alternative considered**: relative/humanized time (`"5m ago"`). Rejected for
v1 — RFC 3339 is unambiguous, trivially testable, and avoids introducing a
"humanize" dependency; can be revisited later as a formatting flag.

### 5. Default columns constant: `[Agent, Session, Pane, Status]`

Matches today's fixed four-column output exactly (module-level constant, e.g.
`const DEFAULT_COLUMNS: &[Field] = &[Field::Agent, Field::Session, Field::Pane, Field::Status];`),
used when `--columns` is omitted. The clap `--columns` argument's help text
SHALL render this default using `default_value_t`/an explicit string built from
the same constant (e.g. via a helper that joins the `Field` `Display` impl with
commas), so the help text and the constant can never drift apart:
`[default: agent,session,pane,status]`.

### 6. Breaking rename handled as a hard cutover, no deprecation shim

`--sort-by state` and `--sort-by updated-at` will be rejected outright (clap
error) rather than silently accepted as aliases. This is a deliberate
**BREAKING** change per the proposal; a compatibility shim was considered and
rejected to keep the vocabulary singular and avoid permanently carrying two
spellings.

## Risks / Trade-offs

- **[Risk]** Existing scripts/aliases using `--sort-by state` or
  `--sort-by updated-at` will break silently at runtime (non-zero exit). →
  **Mitigation**: Clap's rejection message will list valid values
  (`agent, session, pane, status, updatedAt`), and the proposal explicitly flags
  this as breaking; release notes/CHANGELOG should call it out.
- **[Risk]** `Vec<Field>` with duplicates could produce confusingly wide output
  if a user passes `--columns agent,agent,agent`. → **Mitigation**: This is
  accepted as intentional, literal behavior per the proposal (no dedup);
  documented in `--help` text.
- **[Risk]** RFC 3339 with subsecond/offset formatting could make `updatedAt`
  columns wide, disrupting alignment when mixed with short columns. →
  **Mitigation**: existing per-column max-width padding handles arbitrary-width
  cells already (validated by the emoji-alignment test); no new alignment logic
  required.
- **[Risk]** Generalizing `format_panes` from four hardcoded fields to a generic
  per-`Field` loop could silently regress the two-space separation, widest-cell
  column sizing, or fixed start-position guarantees. → **Mitigation**: treat
  these three properties as explicit, strict spec requirements (see
  `list-columns` capability) with dedicated regression tests covering both the
  default column set and arbitrary `--columns` selections, run before/after the
  refactor.
- **[Trade-off]** Reworking `format_panes` from fixed four-field logic to a
  generic field-mapping loop touches all existing rendering tests; they must be
  updated to reflect `status` naming and, where relevant, exercise custom
  `--columns` orderings.

## Migration Plan

1. Introduce the shared `Field` enum with camelCase `ValueEnum` mappings; update
   `--sort-by` to use it (rename-only, no behavior change to sort comparisons).
2. Generalize `format_panes` to accept `&[Field]` and add the `DEFAULT_COLUMNS`
   constant; wire the existing call site in `run()` to use it when `--columns`
   is absent.
3. Add the `--columns`/`-c` clap argument, wired to the same `Field` enum.
4. Add `updatedAt` formatting support.
5. Update all existing tests/doc comments referencing `state`/`updated-at` to
   `status`/`updatedAt`; add new tests for `--columns` parsing, ordering,
   duplicates, invalid values, and `updatedAt` rendering (including the
   empty-when-absent case).
6. No data migration needed (state files on disk are unaffected — only CLI flag
   strings and column headers change).

## Open Questions

- Should the shared enum be renamed `Field` in code, or should `SortKey` be kept
  as the type name for minimal diff (with only its `ValueEnum` strings changed)
  and reused for `--columns`? (Naming only; no behavioral impact.)
- Should `--columns` have a short alias other than `-c` if `-c` is later needed
  for something else on `list`? (No current conflict; `-c` is free today.)
