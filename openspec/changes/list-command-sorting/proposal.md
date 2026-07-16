## Why

The `dv list` command currently outputs agent harness panes in an arbitrary
order, making it hard to quickly locate what matters — especially when many
agents are running. Users need the ability to sort results by meaningful
properties so they can find blocked, idle, or active agents at a glance.

## What Changes

- Add a `--sort-by` flag (or short form `-s`) to `dv list` that accepts a sort
  key: `state`, `agent`, `session`, `pane`, or `updated-at`.
- Add a `--order` flag (short form `-o`) to control sort direction: `asc`
  (default) or `desc`.
- Define a canonical state ordering for `AgentState` so that `blocked` is always
  at one extreme of the sort (appears first in ascending order), enabling quick
  identification of agents that need attention.
- Default sort is by `state` ascending so blocked agents surface first out of
  the box.

## Capabilities

### New Capabilities

- `list-sorting`: Sorting support for `dv list` — `--sort-by` flag, `--order`
  flag, and a canonical `AgentState` sort order where
  `blocked < idle < working < done < unknown`.

### Modified Capabilities

- `list-sessions`: The `list` subcommand gains two new optional flags
  (`--sort-by`, `--order`). Existing output format and reconciliation logic are
  unchanged; only the order of printed rows is affected.

## Impact

- `src/` (CLI flag parsing, list command logic, output rendering)
- `AgentState` type gains an `Ord` / `PartialOrd` impl (or an explicit priority
  mapping) to support state-based sorting
- No breaking changes: the two new flags are optional; existing callers with no
  flags continue to work
