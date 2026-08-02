## 1. Project Structure

- [x] 1.1 Create `src/commands/mod.rs` to serve as the commands module root
- [x] 1.2 Create `src/commands/list.rs` with the list subcommand handler
      skeleton
- [x] 1.3 Wire the `commands` module into `src/main.rs`

## 2. CLI Wiring

- [x] 2.1 Add `List` variant to the `Commands` enum in `src/main.rs` with
      `--server` / `-s` optional flag
- [x] 2.2 Dispatch `Commands::List` to `commands::list::run(server)` in the
      `run()` function

## 3. Core Implementation

- [x] 3.1 Define
      `struct PaneEntry { agent: String, session: String, window_index: u32, pane_index: u32 }`
      in `src/commands/list.rs`
- [x] 3.2 Implement
      `tmux_panes(server: Option<&str>) -> anyhow::Result<Vec<PaneEntry>>` that
      shells out to
      `tmux list-panes -a -F '#{pane_current_command}\t#{session_name}\t#{window_index}\t#{pane_index}'`
      and parses each tab-delimited line into a `PaneEntry`
- [x] 3.3 Handle the "no tmux server running" case: treat non-zero exit from
      `tmux list-panes` as an empty list
- [x] 3.4 Handle the "tmux binary not found" case: surface a user-friendly error
      containing "tmux not found"
- [x] 3.5 Implement `filter_agent_panes(panes: &[PaneEntry]) -> Vec<&PaneEntry>`
      that keeps entries whose `agent` field exactly equals `claude` or `pi`
- [x] 3.6 Implement columnar formatting: compute the max width of each column
      (agent, session, window.pane), left-align all three with per-column
      padding of longest + 2 spaces
- [x] 3.7 Implement `commands::list::run(server: Option<String>)` that calls the
      above helpers and prints formatted lines to stdout

## 4. Tests

- [x] 4.1 Unit test `filter_agent_panes`: panes with `agent == "claude"` or
      `agent == "pi"` are kept; others are dropped
- [x] 4.2 Unit test `filter_agent_panes`: empty input returns empty output
- [x] 4.3 Unit test `filter_agent_panes`: near-matches like `"pilot"` or
      `"Claude"` are excluded (exact, case-sensitive match)
- [x] 4.4 Unit test columnar formatting: all three columns are left-aligned and
      padded to the correct widths across a mixed-length dataset
- [x] 4.5 Integration/E2E test using `testcontainers` + `alpine/git` container
      verifying `rd list` exits 0 and produces no output when tmux is absent
      (binary not on PATH edge case handled gracefully)

## 5. Validation

- [x] 5.1 Run `mise run build` and confirm zero compile errors
- [x] 5.2 Run `mise run test` and confirm all tests pass
- [x] 5.3 Manual smoke-test: `rd list` against a live tmux server with at least
      one `pi` or `claude` pane running

## 6. `pi`-via-`node` Detection

- [x] 6.1 Add `pane_pid: u32` field to `PaneEntry` and extend the tmux format
      string to include `#{pane_pid}` as the fifth tab-delimited field
- [x] 6.2 Implement `build_process_table() -> HashMap<u32, Vec<String>>` that
      runs `ps -A -o pid=,ppid=,args=` and returns a map of ppid → vec of
      argv[0] basenames of direct children
- [x] 6.3 Implement pure `resolve_from_process_table(panes, table)` that mutates
      any `PaneEntry` whose `agent == "node"` to `"pi"` when the pane's shell
      PID has a direct child with argv[0] basename `== "pi"`
- [x] 6.4 Implement `resolve_node_agents(panes)` that skips the `ps` call when
      no `node` panes exist, otherwise calls 6.2 then 6.3
- [x] 6.5 Call `resolve_node_agents` inside `tmux_panes` after parsing output
- [x] 6.6 Unit test `resolve_from_process_table`: a `node` pane whose shell PID
      has a child with argv0 `pi` is resolved to `pi`
- [x] 6.7 Unit test `resolve_from_process_table`: a `node` pane with no matching
      child is left as `node`
- [x] 6.8 Unit test `resolve_from_process_table`: `claude` panes are unchanged
- [x] 6.9 Run `mise run build` and `mise run test` and confirm all pass
