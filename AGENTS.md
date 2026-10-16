# agentctl

This project implements a tmux-based agent lifecycle tracker. The project is an
pnpm Turborepo monorepo. It uses `mise` to manage tools and tasks.

## Rules

- You MUST never update any `AGENTS.md` files without explicit permission from
  the user.
- You MUST never skip git hooks. If a git hook fails, you MUST address or fix
  the reported issue.

## Layout

This project implements a Hexagonal Architecture (Core + Ports & Adapters).

- `apps/` — The executable applications. These are thin wrappers that wire
  together the core and the adapters. They must not contain business logic.
- `packages/` — The libraries that implement the core and the adapters. The core
  must not depend on any adapter or infrastructure code.

## Running Tasks

Get the list of available tasks.

```bash
mise tasks
```

Get the list of available tools.

```bash
mise list
```

## Testing

All new features MUST include unit and/or integration tests (vitest).

All E2E and integration tests MUST use
[testcontainers](https://node.testcontainers.org/) so they are fully isolated
from the host environment (filesystem, git config, hooks, env vars, etc.).
