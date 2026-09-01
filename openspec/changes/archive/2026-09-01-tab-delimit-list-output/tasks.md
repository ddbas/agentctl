## 1. Core formatting fix

- [x] 1.1 Replace the manual per-cell loop + `.trim_end()` in `format_panes`
      (`src/commands/list.rs`) with building a `Vec<String>` of rendered cells
      per row and joining with `"\t"`, so every row has exactly `columns.len()`
      fields regardless of empty trailing cells
- [x] 1.2 Remove the column-width computation loop and the `unicode_width`
      import (no longer needed once padding is removed)
- [x] 1.3 Remove the `unicode-width` dependency from `Cargo.toml` (and
      `Cargo.lock`)

## 2. Tests

- [x] 2.1 Update `format_panes_empty_tool_column_when_zero` and
      `format_panes_unknown_state_never_shows_tool_count` to expect a trailing
      tab before the empty `activeTools` field
- [x] 2.2 Update `format_panes_updated_at_empty_when_no_state_file` to expect
      `"claude\t\n"` instead of `"claude\n"`
- [x] 2.3 Replace
      `format_panes_trims_trailing_whitespace_for_custom_last_column` with
      `format_panes_keeps_fixed_field_count_for_custom_last_column`, asserting
      every row splits into exactly 2 tab-separated fields
- [x] 2.4 Update remaining default-column-set tests
      (`format_panes_handles_wide_unicode_with_tabs`,
      `format_panes_left_aligned_correct_widths`, `format_panes_single_entry`,
      `format_panes_column_alignment_mixed_tool_counts`,
      `format_panes_use_color_true_wraps_state_in_ansi_codes`,
      `format_panes_use_color_false_matches_pre_change_plain_output`) to expect
      the trailing tab before an empty `activeTools` field
- [x] 2.5 Run `mise run test` and confirm all tests pass

## 3. Documentation

- [x] 3.1 Update the `src/commands/list.rs` module doc comment to describe
      tab-delimited output and the fixed-field-count guarantee
- [x] 3.2 Update the `list` subcommand's doc comment in `src/main.rs` (`--help`
      text) to describe tab-delimited columns
- [x] 3.3 Update `README.md`'s `rd list` section to describe tab-delimited
      output and the fixed-field-count guarantee

## 4. Spec sync

- [x] 4.1 Update `list-sessions` spec: default-output requirement describes
      tab-delimited columns instead of left-aligned padded columns
- [x] 4.2 Update `list-columns` spec: replace the alignment and
      trailing-whitespace-trim requirements with tab-delimiter and
      fixed-field-count requirements
- [x] 4.3 Run `openspec validate --changes tab-delimit-list-output --strict` and
      confirm it passes
