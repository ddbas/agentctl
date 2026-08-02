## Why

The `rd list` command currently outputs agent harness panes in an arbitrary
order, making it hard to quickly locate what matters — especially when many
agents are running. Users need the ability to sort results by meaningful
properties so they can find blocked, idle, or active agents at a glance.

## What Changes

- Add a `--sort-by` flag (or short form `-s`) to `rd list` that accepts a sort
  key: `state`, `agent`, `session`, `pane`, or `updated-at`.
- Add a `--reverse` flag (short form `-r`) to reverse the sort order (boolean,
  absent = ascending).
- Define a canonical state ordering for `AgentState` so that `blocked` is always
  at one extreme of the sort (appears first in ascending order), enabling quick
  identification of agents that need attention.
- Default sort is by `updated-at` descending (most recently active agents
  surface first out of the box).

## Capabilities

### New Capabilities

- `list-sorting`: Sorting support for `rd list` — `--sort-by` flag, `--reverse`
  flag, and a canonical `AgentState` sort order where
  `blocked < idle < working < done < unknown`.

### Modified Capabilities

- `list-sessions`: The `list` subcommand gains two new optional flags
  (`--sort-by`, `--reverse`). Existing output format and reconciliation logic
  are unchanged; only the order of printed rows is affected.

## Impact

- `src/` (CLI flag parsing, list command logic, output rendering)
- `AgentState` type gains an `Ord` / `PartialOrd` impl (or an explicit priority
  mapping) to support state-based sorting
- No breaking changes: the two new flags are optional; existing callers with no
  flags continue to work
