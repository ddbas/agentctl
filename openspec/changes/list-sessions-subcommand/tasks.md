## 1. Project Structure

- [ ] 1.1 Create `src/commands/mod.rs` to serve as the commands module root
- [ ] 1.2 Create `src/commands/list.rs` with the list subcommand handler
      skeleton
- [ ] 1.3 Wire the `commands` module into `src/main.rs`

## 2. CLI Wiring

- [ ] 2.1 Add `List` variant to the `Commands` enum in `src/main.rs` with
      `--server` / `-s` optional flag
- [ ] 2.2 Dispatch `Commands::List` to `commands::list::run(server)` in the
      `run()` function

## 3. Core Implementation

- [ ] 3.1 Implement
      `tmux_windows(server: Option<&str>) -> anyhow::Result<Vec<WindowEntry>>`
      that shells out to
      `tmux list-windows -a -F '#{session_name}\t#{window_index}\t#{window_name}'`
- [ ] 3.2 Define
      `struct WindowEntry { session: String, window_index: u32, window_name: String }`
      in `src/commands/list.rs`
- [ ] 3.3 Handle the "no tmux server running" case: treat non-zero exit from
      `tmux list-windows` as an empty list
- [ ] 3.4 Handle the "tmux binary not found" case: surface a user-friendly error
      containing "tmux not found"
- [ ] 3.5 Implement
      `filter_harness_windows(windows: &[WindowEntry]) -> Vec<&WindowEntry>`
      that keeps entries whose session name starts with `claude` or `pi`
- [ ] 3.6 Implement columnar formatting: pad session names to the longest name +
      2 spaces, right-align window index in a 2-char field, append window name
- [ ] 3.7 Implement `commands::list::run(server: Option<String>)` that calls the
      above helpers and prints formatted lines to stdout

## 4. Tests

- [ ] 4.1 Unit test `filter_harness_windows`: entries with `claude`/`pi` prefix
      are kept, others are dropped
- [ ] 4.2 Unit test `filter_harness_windows`: empty input returns empty output
- [ ] 4.3 Unit test `filter_harness_windows`: prefix case-sensitivity (`Claude`
      is excluded)
- [ ] 4.4 Unit test columnar formatting: session names are padded to the correct
      width, window index is right-aligned
- [ ] 4.5 Integration/E2E test using `testcontainers` + `alpine/git` container
      verifying `dv list` exits 0 and produces no output when tmux is absent
      (binary not on PATH edge case handled gracefully)

## 5. Validation

- [ ] 5.1 Run `mise run build` and confirm zero compile errors
- [ ] 5.2 Run `mise run test` and confirm all tests pass
- [ ] 5.3 Manual smoke-test: `dv list` against a live tmux server with at least
      one `pi` or `claude` session
