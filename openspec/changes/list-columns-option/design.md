## Context

`rd list` (`src/commands/list.rs`) currently hardcodes both the sort keys
(`SortKey` enum: `Agent`, `Session`, `Pane`, `State`, `UpdatedAt`, with kebab
`Display`/`ValueEnum` strings) and the rendered output columns. Since this
proposal was first drafted, two other changes have merged to `main` and changed
what `format_panes` actually renders:

- **`track-tool-calls`** added an unconditional, unnamed fifth column showing
  the pane's in-flight tool-call count as `N tool` / `N tools`, empty when
  `active_tools` is `0`, absent, or the pane's status is `unknown` — with a
  hardcoded rule that no trailing delimiter/whitespace is printed after `state`
  when that fifth column is empty.
- **`colorize rd list state column`** made `format_panes` take a
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
the vocabulary used by `--sort-by` and `--columns` as much as is sensible: both
use one shared, camelCase spelling for the keys they have in common (`agent`,
`session`, `pane`, `status`, `updatedAt`), but `--columns` additionally accepts
`activeTools` (formalizing the existing unnamed fifth column), which `--sort-by`
deliberately does not accept (see Decision 1).

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

### 1. `SortKey` (sort-only) and `Column` (columns-only, superset) as two enums

Rename `SortKey`'s `ValueEnum`/`Display` string mappings to camelCase (`status`,
`updatedAt`; `agent`/`session`/`pane` unchanged) — no new variants, no behavior
change to sort comparisons.

Introduce a separate `Column` enum for `--columns` with **six** variants:
`Agent`, `Session`, `Pane`, `Status`, `UpdatedAt`, `ActiveTools`, each with a
`clap::ValueEnum` mapping to the camelCase string (`agent`, `session`, `pane`,
`status`, `updatedAt`, `activeTools`). `--columns` parses against `Column`;
`--sort-by` continues to parse against `SortKey`. The two enums' first five
variants use identical strings by construction (a unit test asserts this — see
Tasks), but they are not the same Rust type, and `--sort-by` simply has no
`ActiveTools` variant to accept.

**Why not one shared enum for both flags (as originally proposed)?** The
original design assumed `--sort-by` and `--columns` would end up with identical
vocabularies. That's no longer true: the already-shipped `active_tools` column
has no defined sort semantics or requirement anywhere in the spec set, and
adding sortability for it is scope this change doesn't need to take on. A single
enum would force `--sort-by` to either (a) accept `activeTools` and need bespoke
rejection logic anyway (clap can't reject one `ValueEnum` variant for only one
flag without a custom parser/validator, negating the simplicity a shared enum
was meant to provide), or (b) silently sort by something undocumented. Two enums
with a guaranteed-identical shared prefix, verified by a test, gets the "no
accidental drift" property the original design wanted without over-generalizing
the sort-key surface.

**Alternative considered**: keep `SortKey` and add a separate `Column` enum with
its own string mapping and no cross-checking. Rejected on its own (this risks
the two vocabularies silently diverging over time), but the underlying structure
— two enums — is retained here specifically _because_ the vocabularies are no
longer meant to be identical; the drift risk is instead mitigated with an
explicit equivalence test over the shared prefix.

### 2. `--columns` value is `Vec<Column>` via a custom clap parser

Clap's `value_delimiter = ','` combined with `value_enum` on a
`Vec<Column>`-typed field parses `--columns agent,status,session` directly into
`vec![Column::Agent, Column::Status, Column::Session]`, reusing per-token
`ValueEnum` parsing (and its rejection message) with no hand-rolled string
splitting.

### 3. `format_panes` takes an ordered `&[Column]` instead of being column-fixed

Generalize row rendering: for each pane, build a `Vec<String>` of cell values by
mapping over the requested `Column` list (`Column::Agent` → `pane.agent`,
`Column::Pane` → `format!("{}.{}", window_index, pane_index)`, `Column::Status`
→ resolved `AgentState` string, `Column::UpdatedAt` → formatted timestamp or
empty string, `Column::ActiveTools` → the existing `N tool`/`N tools`/empty
logic, unchanged, now keyed off column selection instead of being
unconditional). Column widths are computed per position from this same per-row
`Vec<String>` set (max width in that position + 2), preserving the existing
Unicode-width alignment logic. Duplicate columns in the list simply produce
duplicate cell computations at their respective positions — no special-casing
needed.

**Status coloring** (`colorize_state`) is applied after width computation, at
whichever position(s) hold `Column::Status` — width is always computed from the
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
`const DEFAULT_COLUMNS: &[Column] = &[Column::Agent, Column::Session, Column::Pane, Column::Status, Column::ActiveTools];`),
used when `--columns` is omitted. The clap `--columns` argument's help text
SHALL render this default using `default_value_t`/an explicit string built from
the same constant (e.g. via a helper that joins the `Column` `Display` impl with
commas), so the help text and the constant can never drift apart:
`[default: agent,session,pane,status,activeTools]`.

### 6. Breaking rename handled as a hard cutover, no deprecation shim

`--sort-by state` and `--sort-by updated-at` will be rejected outright (clap
error) rather than silently accepted as aliases. This is a deliberate
**BREAKING** change per the proposal; a compatibility shim was considered and
rejected to keep the vocabulary singular and avoid permanently carrying two
spellings.

### 7. `activeTools` is columns-only; `--sort-by activeTools` is rejected

`Column::ActiveTools` has no `SortKey` counterpart. `--sort-by activeTools`
fails the same way `--sort-by foobar` does: a clap error listing the actual
valid `--sort-by` values (`agent, session, pane, status, updatedAt` — note:
without `activeTools`). A unit test (`invalid_sort_by_rejects_active_tools` or
similar) locks this in explicitly, since it's easy to assume from the "shared
vocabulary" framing of the original proposal that `activeTools` would be
sortable too.

**Alternative considered**: make `activeTools` sortable (numeric, e.g. busiest
first by default like `updatedAt` is newest-first). Rejected — out of scope; no
existing requirement calls for this, and it would silently expand the change's
surface beyond reconciling `--columns` with what already shipped.

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
- **[Trade-off]** Reworking `format_panes` from fixed five-field logic to a
  generic field-mapping loop touches all existing rendering tests, including the
  color and active-tools tests added by the two changes that merged after this
  proposal was first drafted; they must be updated to reflect `status` naming
  and, where relevant, exercise custom `--columns` orderings, while continuing
  to assert the same color and tool-count behavior unchanged.
- **[Risk]** `--columns` and `--sort-by` no longer share one identical
  vocabulary (`--columns` also accepts `activeTools`), which could read as an
  inconsistency to users coming from the original proposal's "single shared
  vocabulary" framing. → **Mitigation**: both flags' `--help` text and the spec
  explicitly document `--columns`'s vocabulary as a superset of `--sort-by`'s,
  and `--sort-by activeTools`'s error message lists only the values `--sort-by`
  actually accepts.

## Migration Plan

1. Rename `SortKey`'s camelCase `ValueEnum` mappings (`status`, `updatedAt`); no
   new variants, no behavior change to sort comparisons.
2. Introduce the `Column` enum (`Agent`, `Session`, `Pane`, `Status`,
   `UpdatedAt`, `ActiveTools`) with camelCase `ValueEnum` mappings, plus a test
   asserting its first five variants' strings match `SortKey`'s.
3. Generalize `format_panes` to accept `&[Column]`, threading through the
   existing `colorize_state` call (keyed to `Column::Status` position(s)) and
   the existing active-tools N/tool(s)/empty formatting (keyed to
   `Column::ActiveTools`); replace the hardcoded fifth-column trailing-space
   special case with a generic per-row trailing-whitespace trim; add the
   `DEFAULT_COLUMNS` constant (`[Agent, Session, Pane, Status, ActiveTools]`);
   wire the existing call site in `run()` to use it when `--columns` is absent.
4. Add the `--columns`/`-c` clap argument, wired to `Column`.
5. Add `updatedAt` formatting support.
6. Update all existing tests/doc comments referencing `state`/`updated-at` to
   `status`/`updatedAt`; add new tests for `--columns` parsing, ordering,
   duplicates, invalid values (including `activeTools` being valid for
   `--columns` but not `--sort-by`), and `updatedAt` rendering (including the
   empty-when-absent case). Confirm existing color and active-tools tests pass
   unchanged through the generalized code path.
7. No data migration needed (state files on disk are unaffected — only CLI flag
   strings and column headers change).

## Open Questions

- Should `Column` and `SortKey` be merged into one enum with a `--sort-by`-side
  custom validator that rejects `ActiveTools`, instead of two separate enums?
  (Naming/structure only; no behavioral impact either way, as long as the
  equivalence test and rejection behavior are preserved.)
- Should `--columns` have a short alias other than `-c` if `-c` is later needed
  for something else on `list`? (No current conflict; `-c` is free today.)
