## Context

The `dv list` command discovers tmux panes running agent harnesses (`claude`,
`pi`) and prints them in a fixed columnar format. Currently there is no user
control over output order — rows appear in whatever order `tmux list-panes -a`
returns them, which is effectively session/window/pane index order.

`AgentState` is a plain enum (`Idle`, `Working`, `Done`, `Blocked`, `Unknown`)
with `Display` but no ordering trait. The list output pipeline is: query tmux →
filter → load state map → `format_panes`.

## Goals / Non-Goals

**Goals:**

- Let callers control which column the output is sorted by via `--sort-by`
  (`-s`)
- Let callers control ascending vs descending order via `--reverse` (`-r`)
- Establish a canonical `AgentState` priority ordering where `blocked` is at one
  extreme so it can surface first
- Keep the change additive — no existing behaviour changes for callers who omit
  the new flags

**Non-Goals:**

- Multi-key / compound sort (sort by state, then session — out of scope)
- Machine-readable / JSON output (separate concern)
- Filtering (e.g. `--state blocked`) — separate concern

## Decisions

### Decision: Sort keys

Expose five sort keys:

| Key          | Sorts by                                                                        |
| ------------ | ------------------------------------------------------------------------------- |
| `state`      | `AgentState` priority (see below)                                               |
| `agent`      | Agent name lexicographically                                                    |
| `session`    | Tmux session name lexicographically                                             |
| `pane`       | `window_index.pane_index` numerically (window major, pane minor)                |
| `updated-at` | `updated_at` timestamp from state file; panes with no state file sort as oldest |

**Default**: `state` ascending (blocked agents surface first without any flags).

**Alternatives considered**: Alphabetical-only keys — rejected because
`state`-first default is the most useful out of the box.

### Decision: Sort order flag

Use `--reverse` / `-r` as a boolean flag to flip the sort direction. Absent =
ascending, present = descending.

**Alternatives considered**: `--order asc`/`desc` value flag — more explicit but
more verbose; `--reverse` is idiomatic (mirrors `ls -r`, `sort -r`,
`git log --reverse`) and saves a keystroke.

### Decision: AgentState priority ordering

Define a canonical ascending priority order:

```
blocked(0) < idle(1) < working(2) < done(3) < unknown(4)
```

- `blocked` at index 0: represents agents that need human attention _right now_
  — surfacing them first in the default (`state asc`) view is the most valuable
  default behaviour.
- `idle` at index 1: live but not active — worth knowing, but less urgent than
  blocked.
- `working` at index 2: making progress — healthy state, no action needed.
- `done` at index 3: finished — can be reviewed or cleaned up.
- `unknown` at index 4: no state file or contradictory state — least
  informative, shown last.

With `--reverse`, `unknown` surfaces first and `blocked` is last (useful when
looking for completed agents).

Implement via `Ord` / `PartialOrd` on `AgentState` using the explicit numeric
priority (not enum discriminant order, so variant definition order can be
changed freely without affecting sort).

**Alternatives considered**:

- `unknown` at extreme instead of `blocked` — rejected; `blocked` is more
  actionable so it should be the extreme that surfaces in the default sort.
- Using enum discriminant order (derive `Ord`) — rejected; discriminant order
  would change if variants are reordered, making it a fragile implicit contract.

### Decision: Sort applied to the filtered pane list before formatting

Sort is applied to the `Vec<&PaneEntry>` (or the owned `Vec<PaneEntry>`) after
filtering but before `format_panes`. The state map is passed in as context for
state-based or updated-at-based sort comparisons.

A helper `sort_panes(panes, key, reverse, state_map)` is introduced in `list.rs`.

**Alternatives considered**: Sort inside `format_panes` — rejected;
`format_panes` is a pure formatter and should not own sort logic.

### Decision: CLI flags

Add to the `list` subcommand in `main.rs`:

- `--sort-by` / `-s`: accepts `state` (default), `agent`, `session`, `pane`,
  `updated-at`. Parse into a `SortKey` enum in `list.rs`.
- `--reverse` / `-r`: boolean flag; when present the sort order is reversed.

Update `run(server, sort_by, reverse)` signature accordingly.

## Risks / Trade-offs

- [Risk] Short flag `-r` may conflict with a future flag on the `list`
  subcommand → Mitigation: acceptable now; `-r` for reverse is a widely
  understood convention.
- [Risk] Sorted output differs from the implicit tmux order users may have grown
  accustomed to → Mitigation: state-asc is strictly more useful; old order was
  implementation-defined, not specified.
- [Trade-off] `Ord` on `AgentState` encodes policy (which state is "most
  urgent") in the type → Mitigation: the ordering is documented in the spec and
  type-level comments so it is explicit, not accidental.
