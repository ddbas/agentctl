## Context

`agentctl list` (`src/commands/list.rs`) currently hardcodes both the sort keys
(`SortKey` enum: `Agent`, `Session`, `Pane`, `State`, `UpdatedAt`, with kebab
`Display`/`ValueEnum` strings) and the rendered output columns. Since this
proposal was first drafted, two other changes have merged to `main` and changed
what `format_panes` actually renders:

- **`track-tool-calls`** added an unconditional, unnamed fifth column showing
  the pane's in-flight tool-call count as `N tool` / `N tools`, empty when
  `active_tools` is `0`, absent, or the pane's status is `unknown` — with a
  hardcoded rule that no trailing delimiter/whitespace is printed after `state`
  when that fifth column is empty.
- **`colorize agentctl list state column`** made `format_panes` take a
  `use_color: bool` and wrap the `state` column's plain text in an ANSI
  foreground color escape sequence (via `colorize_state`) keyed to the agent's
  `AgentState`, only when stdout is a terminal; column widths are always
  computed from the plain, uncolored text so the escapes never affect alignment.

`format_panes` today therefore emits five fixed columns in a fixed order —
`agent, session, window.pane, state, <tool-count>` — with `state` optionally
colorized. `AgentInfo.updated_at` (a `chrono::DateTime<Utc>`) is already loaded
into the `state_map` used for sorting but is never rendered.

This change (1) introduces a `--columns`/`-c` flag so users choose which fields
appear and in what order, generalizing this current five-column, color-aware
renderer rather than the four-column one originally targeted, and (2) unifies
the vocabulary used by `--sort-by` and `--columns` into a single shared,
camelCase key set: `agent`, `session`, `pane`, `status`, `updatedAt`,
`activeTools` (formalizing the existing unnamed fifth column and making it
sortable, alongside the four pre-existing sort keys).

## Goals / Non-Goals

**Goals:**

- One shared enum/key-set drives both `--sort-by` and `--columns` so the two
  flags can never drift out of sync — including `activeTools`, which is both
  selectable and sortable.
- `--columns` is order-preserving and duplicate-preserving: whatever the user
  types is rendered literally, left to right.
- Default output (no `--columns` given) stays visually identical to today's
  output except for the `state` → `status` rename.
- Adding `updatedAt` as a column does not change default behavior (it's opt-in
  only).
- Reuse the existing Unicode-width-aware column alignment logic (`format_panes`)
  rather than duplicating padding logic per column.

**Non-Goals:**

- No change to sorting _semantics_ for the four pre-existing keys (comparison
  logic, natural directions, stability) — only the string keys used to select
  them are renamed. `activeTools` is new sortable behavior, scoped narrowly to a
  straightforward numeric comparison (see Decision 7).
- No support for column-specific formatting flags (e.g. relative time vs.
  absolute time for `updatedAt`); a single fixed RFC 3339 format is used.
- No persistent/config-file default for `--columns` (CLI flag only, this
  session).
- No renaming of the underlying `AgentState` enum variants or JSON serialization
  keys in state files — only the CLI-facing sort/column key strings change.

## Decisions

### 1. One shared `Field` enum drives both `--sort-by` and `--columns`

Rename `SortKey` to a single `Field` enum with **six** variants: `Agent`,
`Session`, `Pane`, `Status`, `UpdatedAt`, `ActiveTools`, each with a
`clap::ValueEnum` mapping to the camelCase string (`agent`, `session`, `pane`,
`status`, `updatedAt`, `activeTools`). Both `--sort-by` and `--columns` parse
against this same enum, guaranteeing identical accepted values and identical
error messages for invalid input — including for `activeTools`, which is newly
sortable as part of this change (see Decision 7).

**Alternative considered**: keep `activeTools` as a `--columns`-only value,
splitting the vocabulary into a `SortKey` (five variants) and a separate,
superset `Field`/`Column` (six variants) enum. This was this design's first
draft, on the assumption that no existing requirement calls for sorting by
tool-call count. Rejected on reconsideration — the numeric sort is simple to
define unambiguously (Decision 7), and a fully unified vocabulary is both
simpler to implement (one enum, one `ValueEnum` impl, no equivalence test needed
to guard against drift) and matches user expectation that anything shown as a
column is also sortable.

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
string, `Field::ActiveTools` → the existing `N tool`/`N tools`/empty logic,
unchanged, now keyed off column selection instead of being unconditional).
Column widths are computed per position from this same per-row `Vec<String>` set
(max width in that position + 2), preserving the existing Unicode-width
alignment logic. Duplicate columns in the list simply produce duplicate cell
computations at their respective positions — no special-casing needed.

**Status coloring** (`colorize_state`) is applied after width computation, at
whichever position(s) hold `Field::Status` — width is always computed from the
plain (uncolored) text first, then the colorized version substituted in for
display, exactly as `format_panes` already does today for the fixed fourth
column; this generalizes to "wherever `Status` appears" rather than "the fourth
column specifically".

**Trailing whitespace** is handled by trimming each fully-rendered line with
`.trim_end()` (or equivalent) after all columns are padded and joined, rather
than the current hardcoded special case of only omitting the fifth column's
delimiter+text when it's empty. This is a strict generalization: for today's
default column set, the only column that can render an empty string
(`activeTools`) is already last, so the observable output is byte-for-byte
identical to today; for custom `--columns` selections where a different
normally-non-empty-feeling column (e.g. `updatedAt`) ends up last and empty for
a given row, the same trim now applies there too.

This preserves, unchanged, three pre-existing alignment guarantees that MUST NOT
regress regardless of which/how many columns are selected: (a) exactly two
spaces of separation between columns, (b) each column's width equal to the
Unicode display width of its widest cell in that column (no more, no less), and
(c) every row's cell for a given column position starting at the same terminal
column offset as every other row. These three properties are already covered by
the existing `format_panes_left_aligned_correct_widths` and
`format_panes_aligns_columns_with_wide_unicode` tests for the fixed five-column
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

### 5. Default columns constant: `[Agent, Session, Pane, Status, ActiveTools]`

Matches today's fixed five-column output exactly (module-level constant, e.g.
`const DEFAULT_COLUMNS: &[Field] = &[Field::Agent, Field::Session, Field::Pane, Field::Status, Field::ActiveTools];`),
used when `--columns` is omitted. The clap `--columns` argument's help text
SHALL render this default using `default_value_t`/an explicit string built from
the same constant (e.g. via a helper that joins the `Field` `Display` impl with
commas), so the help text and the constant can never drift apart:
`[default: agent,session,pane,status,activeTools]`.

### 6. Breaking rename handled as a hard cutover, no deprecation shim

`--sort-by state` and `--sort-by updated-at` will be rejected outright (clap
error) rather than silently accepted as aliases. This is a deliberate
**BREAKING** change per the proposal; a compatibility shim was considered and
rejected to keep the vocabulary singular and avoid permanently carrying two
spellings.

### 7. `activeTools` is sortable: numeric, descending (busiest-first) by default

`--sort-by activeTools` compares panes numerically by `active_tools` (panes with
no state file, or whose status is `unknown`, are treated as `0`). Its natural
direction is **descending** (busiest agents first) rather than the ascending
default used by `agent`/`session`/`pane`/`status`, matching `UpdatedAt`'s
existing precedent of a "most-interesting-first" natural direction for
count/time-like keys; `--reverse` flips it to ascending (quietest/idle-first).

**Alternative considered**: keep `activeTools` as a `--columns`-only value with
no sort semantics at all (this design's first draft). Rejected — see Decision
1's rationale: the comparison is simple to define unambiguously, and leaving it
unsortable diverges from the unified-vocabulary goal for no strong reason.

**Alternative considered**: ascending natural direction (quietest first, like
the other non-time keys). Rejected — users scanning for "which agents are doing
the most right now" is the more common motivating use case (matching the
original feature's proposal rationale for adding the column at all: "the
operator has no signal that anything unusual is going on"), so busiest-first is
the more useful default, consistent with `updatedAt`'s existing
newest-first-by-default precedent.

## Risks / Trade-offs

- **[Risk]** Existing scripts/aliases using `--sort-by state` or
  `--sort-by updated-at` will break silently at runtime (non-zero exit). →
  **Mitigation**: Clap's rejection message will list valid values
  (`agent, session, pane, status, updatedAt, activeTools`), and the proposal
  explicitly flags this as breaking; release notes/CHANGELOG should call it out.
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
- **[Trade-off]** Reworking `format_panes` from fixed five-field logic to a
  generic field-mapping loop touches all existing rendering tests, including the
  color and active-tools tests added by the two changes that merged after this
  proposal was first drafted; they must be updated to reflect `status` naming
  and, where relevant, exercise custom `--columns` orderings, while continuing
  to assert the same color and tool-count behavior unchanged.
- **[Risk]** `activeTools`'s natural sort direction (descending/busiest-first)
  differs from the ascending default used by `agent`/`session`/`pane`/`status`,
  which could surprise users who expect all keys to sort the same way by
  default. → **Mitigation**: this mirrors the already-shipped, already-
  understood precedent set by `updatedAt` (also descending by default); both are
  documented together in `--sort-by`'s help text and the `list-sorting` spec.

## Migration Plan

1. Introduce the shared `Field` enum (`Agent`, `Session`, `Pane`, `Status`,
   `UpdatedAt`, `ActiveTools`) with camelCase `ValueEnum` mappings, replacing
   `SortKey`; update `--sort-by` to use it (rename-only for the first four,
   behavior-preserving; `ActiveTools` is new, see step 3).
2. Generalize `format_panes` to accept `&[Field]` and add the `DEFAULT_COLUMNS`
   constant; wire the existing call site in `run()` to use it when `--columns`
   is absent.
3. Add numeric `activeTools` comparison logic to `sort_panes` (descending
   natural direction, `0` when absent/unknown).
4. Add the `--columns`/`-c` clap argument, wired to the same `Field` enum.
5. Add `updatedAt` formatting support.
6. Generalize status coloring (keyed to `Field::Status` position(s)) and replace
   the hardcoded fifth-column trailing-space special case with a generic per-row
   trailing-whitespace trim.
7. Update all existing tests/doc comments referencing `state`/`updated-at` to
   `status`/`updatedAt`; add new tests for `--columns` parsing, ordering,
   duplicates, invalid values, `updatedAt` rendering (including the
   empty-when-absent case), and `--sort-by activeTools` (including its
   descending default and `--reverse` behavior). Confirm existing color and
   active-tools tests pass unchanged through the generalized code path.
8. No data migration needed (state files on disk are unaffected — only CLI flag
   strings and column headers change).

## Open Questions

- Should the shared enum be renamed `Field` in code, or should `SortKey` be kept
  as the type name for minimal diff (with only its `ValueEnum` strings changed)
  and reused for `--columns`? (Naming only; no behavioral impact.)
- Should `--columns` have a short alias other than `-c` if `-c` is later needed
  for something else on `list`? (No current conflict; `-c` is free today.)
