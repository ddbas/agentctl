# @agentctl/state-fs

Holds the sole implementation of the `AgentRepository` port: reading and writing
agent state files (`state.json`, `events.jsonl`) under the XDG data directory,
including their `zod` schema. No other package may touch those files, and this
package must not contain tmux, logging, harness installation or business logic.
