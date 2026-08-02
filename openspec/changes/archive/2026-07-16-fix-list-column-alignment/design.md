## Context

The `rd list` command renders a four-column table of active agent panes. Column
widths are computed by taking the maximum `String::len()` across all values in
each column, then applying Rust's `{:<N$}` format specifier to left-pad each
cell.

`String::len()` returns byte length. `{:<N$}` pads by Unicode scalar count.
Neither maps to terminal display width. For session names containing emoji (e.g.
`🌳 radr (claude-hooks)`):

- `🌳` encodes as 4 UTF-8 bytes but is 1 scalar and 2 display columns.
- byte-based max inflates the column target, causing over-padding on that row.
- scalar-based padding under-pads (misses the extra display column of the wide
  character).

The result is visually misaligned columns when any cell contains a wide
character.

## Goals / Non-Goals

**Goals:**

- Columns in `rd list` output are visually aligned for all Unicode input,
  including session names containing emoji or other wide characters.
- Column width calculation uses terminal display width (via `unicode-width`).
- Padding is applied in display-column units, not byte or scalar units.

**Non-Goals:**

- Supporting right-to-left text or zero-width joiners in session names.
- Changing any other aspect of the output format (column order, separators,
  state labels).
- Handling ANSI colour codes inside column values (none are currently emitted).

## Decisions

### Decision: Use the `unicode-width` crate

**Chosen**: add `unicode-width = "0.1"` to `[dependencies]`.

**Rationale**: `unicode-width` is the de-facto standard crate for this in the
Rust ecosystem. It implements Unicode Standard Annex #11 (East Asian Width),
which is what terminal emulators use to decide how many columns a character
occupies. It has no transitive dependencies, is `no_std`-compatible, and has
been stable since 2015.

**Alternatives considered**:

- _Roll our own lookup_: not worth the maintenance burden or correctness risk.
- _`console` crate_: heavier dependency (includes ANSI stripping, progress bars,
  etc.); unnecessary here.

### Decision: Replace `{:<N$}` with manual space padding

**Chosen**: compute `pad = col_width - display_width(value)` and append
`" ".repeat(pad)` for each cell.

**Rationale**: Rust's `{:<N$}` format specifier pads to N _Unicode scalar
values_, which is still wrong for wide characters. There is no built-in format
specifier that pads by display width, so we must pad manually.

The helper is a one-liner; no abstraction layer is needed.

## Risks / Trade-offs

| Risk                                                                                                  | Mitigation                                                                                                   |
| ----------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| `unicode-width` returns 0 for control characters (e.g. `\t`) that could appear in a tmux session name | Column will be under-padded but won't panic; control chars in session names are already a cosmetic non-issue |
| Adding a new crate dependency slightly increases compile time                                         | `unicode-width` is tiny (~50 KB source, zero transitive deps) — negligible                                   |
