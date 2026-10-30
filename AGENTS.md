# agentctl

This project implements a tmux-based agent controller. It tracks and manages
agent processes running across the user's tmux sessions, providing a unified
interface for querying their status. The project is a pnpm Turborepo monorepo.
It uses `mise` to manage tools and tasks.

## Rules

1. You MUST never modify AGENTS.md files without explicit permission from the
   user.
1. You MUST never create, modify or delete documentation without explicit
   permission from the user (including README.md and CONTRIBUTING.md).
1. You MUST never skip git hooks without exclicit permission from the user. If a
   hook fails, fix the issue.
1. You MUST always use Mise for running all tasks and dev tools.
1. You MUST always use OpenSpec for formalizing all requirements.
1. You MUST always separate functional (and non-functional) requirements from
   architectural/technical requirements. They should NEVER be in the same spec.
1. You MUST always commit and push your changes once you've completed a task.
1. All new features MUST include unit and/or integration tests (vitest).
1. All E2E and integration tests MUST use
   [testcontainers](https://node.testcontainers.org/) so they are fully isolated
   from the host environment (filesystem, git config, hooks, env vars, etc.).

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
