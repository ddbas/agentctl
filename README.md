# 📡 Agentctl

A tmux-based agent controller. Tracks and manages agent processes running across
your tmux sessions, providing a unified interface for querying their status.

## Installation

Requires [mise](https://mise.jdx.dev) and `tmux`.

```bash
mise install
mise run install # symlinks ~/.local/bin/agentctl
```

## Usage

Install hooks for the agent harness of your choice.

```sh
agentctl install '<harness>'
```

Run one or more instances of the agent harness.

Then monitor them.

```sh
agentctl list
# pi  🌳 foo (branch-1)  1.0  done
# pi  🌳 bar (branch-3)  0.0  working  1 tool
```
