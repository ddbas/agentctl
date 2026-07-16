## 1. AgentState Ordering

- [x] 1.1 Add `Ord` and `PartialOrd` impls to `AgentState` in `src/state.rs`
      using an explicit priority map:
      `Blocked=0, Done=1, Working=2, Idle=3, Unknown=4`
- [x] 1.2 Add unit tests for the ordering: all five variants sort correctly,
      `Blocked` is minimum, `Unknown` is maximum, full ascending slice matches
      expected order

## 2. SortKey Type

- [x] 2.1 Define `SortKey` enum in `src/commands/list.rs` with variants `State`,
      `Agent`, `Session`, `Pane`, `UpdatedAt`; derive `Debug`, `Clone`, `Copy`,
      `PartialEq`
- [x] 2.2 Implement `FromStr` (or clap `ValueEnum`) for `SortKey` so it parses
      from CLI string values (`state`, `agent`, `session`, `pane`,
      `updated-at`); invalid values produce a clear error

## 3. sort_panes Helper

- [x] 3.1 Implement
      `sort_panes(panes: &mut Vec<&PaneEntry>, key: SortKey, reverse: bool, state_map: &HashMap<String, AgentInfo>)`
      in `src/commands/list.rs`
- [x] 3.2 Implement each sort arm: `State` (use `AgentState` `Ord`), `Agent`
      (lexicographic), `Session` (lexicographic), `Pane` (window_index then
      pane_index), `UpdatedAt` (state file timestamp, missing =
      `DateTime::<Utc>::MIN_UTC`)
- [x] 3.3 Use stable sort (`sort_by`) to preserve relative order of equal-key
      entries
- [x] 3.4 Call `.reverse()` on the sorted slice when `reverse` is `true`
- [x] 3.5 Add unit tests for `sort_panes`: each key with and without `--reverse`,
      stability with equal keys, `updated-at` with and without state file entries

## 4. CLI Flag Wiring

- [x] 4.1 Add `--sort-by` / `-s` argument to the `list` subcommand in
      `src/main.rs` (clap); default `updated-at`
- [x] 4.2 Add `--reverse` / `-r` boolean flag to the `list` subcommand in
      `src/main.rs` (clap); default `false`
- [x] 4.3 Update `run(server, sort_by, reverse)` signature in
      `src/commands/list.rs` to accept `SortKey` and `reverse: bool`
- [x] 4.4 Call `sort_panes` on the filtered agent pane list after state map
      reconciliation and before `format_panes`
- [x] 4.5 Update all call sites of `run(...)` in `main.rs` and tests to pass
      the two new parameters (`sort_by`, `reverse`)

## 5. Tests

- [x] 5.1 Add unit test: `dv list` with no flags sorts by `updated_at`
      descending (most recently active first, no-state-file panes last)
- [x] 5.2 Add unit test: `dv list --sort-by agent` sorts alphabetically by agent
      name
- [x] 5.3 Add unit test: `dv list --sort-by session` sorts by session name
- [x] 5.4 Add unit test: `dv list --sort-by pane` sorts by window index then
      pane index
- [x] 5.5 Add unit test: `dv list --sort-by state --reverse` puts unknown first
      and blocked last
- [x] 5.6 Add unit test: invalid `--sort-by` value exits non-zero with error
      message

## 6. Documentation

- [x] 6.1 Update `--help` text for the `list` subcommand to describe `--sort-by`
      and `--reverse` flags with valid values and defaults
- [x] 6.2 Update `CHANGELOG.md` or commit message with a conventional commit
      (`feat(list): add --sort-by and --reverse flags`)
