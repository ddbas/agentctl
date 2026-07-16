## 1. AgentState Ordering

- [ ] 1.1 Add `Ord` and `PartialOrd` impls to `AgentState` in `src/state.rs`
      using an explicit priority map:
      `Blocked=0, Idle=1, Working=2, Done=3, Unknown=4`
- [ ] 1.2 Add unit tests for the ordering: all five variants sort correctly,
      `Blocked` is minimum, `Unknown` is maximum, full ascending slice matches
      expected order

## 2. SortKey and SortOrder Types

- [ ] 2.1 Define `SortKey` enum in `src/commands/list.rs` with variants `State`,
      `Agent`, `Session`, `Pane`, `UpdatedAt`; derive `Debug`, `Clone`, `Copy`,
      `PartialEq`
- [ ] 2.2 Define `SortOrder` enum in `src/commands/list.rs` with variants `Asc`,
      `Desc`; derive `Debug`, `Clone`, `Copy`, `PartialEq`
- [ ] 2.3 Implement `FromStr` (or clap `ValueEnum`) for both enums so they parse
      from CLI string values (`state`, `agent`, `session`, `pane`, `updated-at`,
      `asc`, `desc`); invalid values produce a clear error

## 3. sort_panes Helper

- [ ] 3.1 Implement
      `sort_panes(panes: &mut Vec<&PaneEntry>, key: SortKey, order: SortOrder, state_map: &HashMap<String, AgentInfo>)`
      in `src/commands/list.rs`
- [ ] 3.2 Implement each sort arm: `State` (use `AgentState` `Ord`), `Agent`
      (lexicographic), `Session` (lexicographic), `Pane` (window_index then
      pane_index), `UpdatedAt` (state file timestamp, missing =
      `DateTime::<Utc>::MIN_UTC`)
- [ ] 3.3 Use stable sort (`sort_by`) to preserve relative order of equal-key
      entries
- [ ] 3.4 Reverse the comparator (or call `.reverse()`) when `SortOrder::Desc`
- [ ] 3.5 Add unit tests for `sort_panes`: each key asc/desc, stability with
      equal keys, `updated-at` with and without state file entries

## 4. CLI Flag Wiring

- [ ] 4.1 Add `--sort-by` / `-s` argument to the `list` subcommand in
      `src/main.rs` (clap); default `state`
- [ ] 4.2 Add `--order` / `-o` argument to the `list` subcommand in
      `src/main.rs` (clap); default `asc`
- [ ] 4.3 Update `run(server, sort_by, order)` signature in
      `src/commands/list.rs` to accept `SortKey` and `SortOrder`
- [ ] 4.4 Call `sort_panes` on the filtered agent pane list after state map
      reconciliation and before `format_panes`
- [ ] 4.5 Update all call sites of `run(...)` in `main.rs` and tests to pass the
      two new parameters

## 5. Tests

- [ ] 5.1 Add integration / unit test: `dv list` with no flags sorts blocked
      agents before idle/working/done/unknown
- [ ] 5.2 Add unit test: `dv list --sort-by agent` sorts alphabetically by agent
      name
- [ ] 5.3 Add unit test: `dv list --sort-by session` sorts by session name
- [ ] 5.4 Add unit test: `dv list --sort-by pane` sorts by window index then
      pane index
- [ ] 5.5 Add unit test: `dv list --order desc --sort-by state` puts unknown
      first and blocked last
- [ ] 5.6 Add unit test: invalid `--sort-by` / `--order` value exits non-zero
      with error message

## 6. Documentation

- [ ] 6.1 Update `--help` text for the `list` subcommand to describe `--sort-by`
      and `--order` flags with valid values and defaults
- [ ] 6.2 Update `CHANGELOG.md` or commit message with a conventional commit
      (`feat(list): add --sort-by and --order flags`)
