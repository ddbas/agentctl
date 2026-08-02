## Why

The `rd list` command produces misaligned columns when session names contain
emoji or other Unicode wide characters (e.g. `🌳 radr (claude-hooks)`). Column
padding is calculated using byte lengths and Rust's `{:<N$}` formatter (which
pads by Unicode scalar count), neither of which accounts for the extra terminal
columns occupied by wide characters.

## What Changes

- The column-width calculation in `format_panes` is updated to use Unicode
  display width (terminal columns) instead of byte length.
- Manual space-padding replaces `{:<N$}` format specifiers so that padding is
  computed in display columns, not character counts.
- A new crate dependency `unicode-width` is added.
- A unit test with an emoji-bearing session name is added to `format_panes`.

## Capabilities

### New Capabilities

_(none — this is a bug fix with no new user-facing capability)_

### Modified Capabilities

- `list-sessions`: The column-alignment requirement of `rd list` output is
  tightened: columns MUST be aligned by terminal display width, not byte or
  scalar count.

## Impact

- **Code**: `src/commands/list.rs` (`format_panes` function)
- **Dependencies**: adds `unicode-width = "0.1"` to `Cargo.toml`
- **Tests**: extends existing unit tests in `list.rs`
- **No breaking changes** — output format is identical for ASCII-only names;
  only fixes visual misalignment for wide-character names
