# @agentctl/core

Holds everything that is independent of infrastructure: domain types, the status
state machine, `App` use cases, list rendering, pure helpers (time, path
formatting) and the port interfaces (`AgentRepository`, `Tmux`, `Harness`,
`Logger`). It must never import `node:*` modules, `pino`, `zod`, `commander` or
any other `@agentctl/*` package, and concrete implementations of its ports
belong in the adapter packages.
