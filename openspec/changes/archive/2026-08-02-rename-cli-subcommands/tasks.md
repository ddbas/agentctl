## 1. CLI surface (`src/main.rs`)

- [x] 1.1 Remove `Commands::Hooks` and `HooksSubcommand::Install`; add a
      top-level `Commands::Install { harness: Harness }` variant.
- [x] 1.2 Update the `Install` variant's doc comment / help text to describe
      installing radr on a harness generically (not hook-specific wording).
- [x] 1.3 Remove `Commands::SetState` and `Commands::ClearState`; add a
      `Commands::State { subcommand: StateSubcommand }` variant with
      `StateSubcommand::Set { state, harness, pane }` and
      `StateSubcommand::Clear { pane }`, preserving existing field
      names/types/docs.
- [x] 1.4 Update the `run()` match arms to dispatch `Install` to
      `commands::hooks::run_install(harness)` and `State::Set` / `State::Clear`
      to `commands::set_state::run(...)` / `commands::clear_state::run(...)`.

## 2. Generated content and migration (`src/commands/hooks.rs`)

- [x] 2.1 Update `PI_EXTENSION_CONTENT` to call `rd("state", "set", ...)` /
      `rd("state", "clear")` instead of `rd("set-state", ...)` /
      `rd("clear-state")`, and update its doc comment references to
      `rd install pi`.
- [x] 2.2 Update `RADR_CLAUDE_HOOKS` command strings to `rd state set ...` /
      `rd state clear`.
- [x] 2.3 Update `is_radr_command` to recognize both the old command forms
      (`rd set-state `, `rd clear-state`) and the new ones (`rd state set `,
      `rd state clear`) so old installs are correctly identified as radr-managed
      during the cleanup pass.
- [x] 2.4 Update module-level doc comment and any printed summary text
      (`print_pi_summary`, `format_claude_summary` usage) that references old
      command names or `rd hooks install`.

## 3. Tests

- [x] 3.1 Update existing unit tests in `src/commands/hooks.rs` that assert on
      literal `rd set-state` / `rd clear-state` strings (pi extension content
      tests, Claude hook command/matcher tests, summary formatting tests) to
      assert the new `rd state set` / `rd state clear` strings.
- [x] 3.2 Add a migration test: pre-populate `settings.json` with old-style
      `rd set-state ...` / `rd clear-state` radr hook entries, run
      `install_claude_to`, and assert the old entries are removed and the new
      `rd state set ...` / `rd state clear` entries are present with no
      duplicates.
- [x] 3.3 Update `tests/hooks.rs` and `tests/state_commands.rs` integration
      tests to invoke `rd install <harness>` and `rd state set` /
      `rd state     clear` instead of the old command names.
- [x] 3.4 Run `mise run test` (or the project's configured test task) and fix
      any remaining references to old command names found by the build/tests.

## 4. Documentation

- [x] 4.1 Search the repo for remaining references to `rd hooks install`,
      `rd set-state`, `rd clear-state` (README, other docs, code comments) and
      update them to the new command names.
