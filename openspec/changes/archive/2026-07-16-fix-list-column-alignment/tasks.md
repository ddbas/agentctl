## 1. Dependency

- [x] 1.1 Add `unicode-width = "0.1"` to `[dependencies]` in `Cargo.toml`

## 2. Core Fix

- [x] 2.1 Import `UnicodeWidthStr` from `unicode_width` in
      `src/commands/list.rs`
- [x] 2.2 Replace `.len()` with `UnicodeWidthStr::width()` for `max_agent`,
      `max_session`, and `max_wp` in `format_panes`
- [x] 2.3 Replace `format!("{:<col1$}{:<col2$}{:<col3$}{}\n", ...)` with manual
      space-padding using `" ".repeat(col - display_width(value))` for each of
      the first three columns

## 3. Tests

- [x] 3.1 Add a unit test `format_panes_aligns_columns_with_wide_unicode` that
      includes at least one session name with an emoji (e.g.
      `🌳 agentctl (claude-hooks)`) alongside an ASCII session name, and asserts
      that the state column starts at the same character position on every line
- [x] 3.2 Run `mise run test` and confirm all tests pass
