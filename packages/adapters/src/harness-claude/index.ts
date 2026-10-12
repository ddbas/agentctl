import { existsSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, join } from "node:path";
import type { Harness } from "@agentctl/core";
import { tildeCollapse } from "../paths.ts";

const PLUGIN_NAME = "agentctl";

interface ClaudeHookDef {
  event: string;
  matcher?: string;
  command: string;
  async: boolean;
}

const readSessionId = `id=$(cat | jq -r '.session_id'); `;
const readToolUseId = `id=$(cat | jq -r '.tool_use_id'); `;
const stopCommand = `payload=$(cat); id=$(echo "$payload" | jq -r '.session_id'); n=$(echo "$payload" | jq -r '(.background_tasks // []) | length'); agentctl agent event RUN_FINISHED --id "$id" --active-tools "$n" --harness claude`;
const ELICITATION_MATCHER = "AskUserQuestion|ExitPlanMode";

/**
 * The eleven hook entries registered in the plugin's `hooks.json`.
 * `PreToolUse`, `PostToolUse` and `PostToolUseFailure` each appear twice: once
 * unmatched (untyped, drives `active_tools`) and once matched (interactive
 * tools only, `--type elicitation`).
 */
export const CLAUDE_HOOKS: readonly ClaudeHookDef[] = [
  {
    event: "SessionStart",
    matcher: "startup|resume|clear",
    command: `${readSessionId}agentctl agent event SESSION_STARTED --id "$id" --harness claude`,
    async: false,
  },
  {
    event: "UserPromptSubmit",
    command: `${readSessionId}agentctl agent event RUN_STARTED --id "$id" --harness claude`,
    async: true,
  },
  {
    event: "PreToolUse",
    command: `${readToolUseId}agentctl agent event TOOL_CALL_START --id "$id" --harness claude`,
    async: true,
  },
  {
    event: "PreToolUse",
    matcher: ELICITATION_MATCHER,
    command: `${readToolUseId}agentctl agent event TOOL_CALL_START --id "$id" --type elicitation --harness claude`,
    async: true,
  },
  {
    event: "PostToolUse",
    command: `${readToolUseId}agentctl agent event TOOL_CALL_RESULT --id "$id" --harness claude`,
    async: true,
  },
  {
    event: "PostToolUse",
    matcher: ELICITATION_MATCHER,
    command: `${readToolUseId}agentctl agent event TOOL_CALL_RESULT --id "$id" --type elicitation --harness claude`,
    async: true,
  },
  {
    event: "PostToolUseFailure",
    command: `${readToolUseId}agentctl agent event TOOL_CALL_RESULT --id "$id" --harness claude`,
    async: true,
  },
  {
    event: "PostToolUseFailure",
    matcher: ELICITATION_MATCHER,
    command: `${readToolUseId}agentctl agent event TOOL_CALL_RESULT --id "$id" --type elicitation --harness claude`,
    async: true,
  },
  { event: "Stop", command: stopCommand, async: true },
  { event: "StopFailure", command: stopCommand, async: true },
  { event: "SessionEnd", command: "agentctl agent clear", async: false },
];

/** `serde_json` (without `preserve_order`) emits object keys sorted. */
function sortKeys(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(sortKeys);
  if (value && typeof value === "object") {
    return Object.fromEntries(
      Object.entries(value as Record<string, unknown>)
        .sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0))
        .map(([k, v]) => [k, sortKeys(v)]),
    );
  }
  return value;
}

function prettyJson(value: unknown): string {
  return JSON.stringify(sortKeys(value), null, 2);
}

/** Minimal manifest; `hooks/hooks.json` is Claude Code's default hooks path. */
export function claudePluginManifest(): object {
  return {
    name: PLUGIN_NAME,
    description: "agentctl agent-state lifecycle hooks for Claude Code",
  };
}

function buildHookEntry(hook: ClaudeHookDef): object {
  const cmd: Record<string, unknown> = {
    type: "command",
    command: hook.command,
  };
  if (hook.async) cmd.async = true;
  const entry: Record<string, unknown> = {};
  if (hook.matcher !== undefined) entry.matcher = hook.matcher;
  entry.hooks = [cmd];
  return entry;
}

/** Build `{ "hooks": { <event>: [ <entry>, ... ] } }`. */
export function buildClaudeHooksJson(): object {
  const hooks: Record<string, object[]> = {};
  for (const hook of CLAUDE_HOOKS) {
    let list = hooks[hook.event];
    if (!list) {
      list = [];
      hooks[hook.event] = list;
    }
    list.push(buildHookEntry(hook));
  }
  return { hooks };
}

/** Default plugin directory: `~/.claude/skills/agentctl`. */
export function defaultClaudePluginDir(home: string = homedir()): string {
  if (!home) throw new Error("could not determine home directory");
  return join(home, ".claude", "skills", PLUGIN_NAME);
}

const manifestPath = (dir: string) =>
  join(dir, ".claude-plugin", "plugin.json");
const hooksJsonPath = (dir: string) => join(dir, "hooks", "hooks.json");

/** Rust `{:<35}` pads by char count. */
function padEnd35(s: string): string {
  const len = Array.from(s).length;
  return len >= 35 ? s : s + " ".repeat(35 - len);
}

export function formatClaudeInstallSummary(pluginDir: string): string {
  const lines = [
    `Installed agentctl plugin for claude (${CLAUDE_HOOKS.length} hook entries):`,
    `  + ${tildeCollapse(manifestPath(pluginDir))}`,
    `  + ${tildeCollapse(hooksJsonPath(pluginDir))}`,
  ];
  for (const hook of CLAUDE_HOOKS) {
    const eventDisplay =
      hook.matcher !== undefined
        ? `${hook.event} (${hook.matcher})`
        : hook.event;
    lines.push(
      `    ${padEnd35(eventDisplay)} → ${hook.command}${hook.async ? " [async]" : ""}`,
    );
  }
  lines.push("(requires `jq` on PATH to parse hook payloads)");
  return lines.join("\n");
}

/** Remove-then-write install of the plugin into `pluginDir`. */
export function installClaudeTo(pluginDir: string): string {
  mkdirSync(dirname(pluginDir), { recursive: true });
  if (existsSync(pluginDir))
    rmSync(pluginDir, { recursive: true, force: true });

  const manifest = manifestPath(pluginDir);
  mkdirSync(dirname(manifest), { recursive: true });
  writeFileSync(manifest, prettyJson(claudePluginManifest()));

  const hooks = hooksJsonPath(pluginDir);
  mkdirSync(dirname(hooks), { recursive: true });
  writeFileSync(hooks, prettyJson(buildClaudeHooksJson()));

  return formatClaudeInstallSummary(pluginDir);
}

export function formatClaudeUninstallSummary(
  pluginDir: string,
  removed: boolean,
): string {
  const display = tildeCollapse(pluginDir);
  return removed
    ? `Uninstalled agentctl plugin for claude:\n  - ${display}`
    : `Uninstalled agentctl plugin for claude:\n  = ${display} (nothing to remove)`;
}

/** Delete the plugin directory if present; never touches siblings. */
export function uninstallClaudeTo(pluginDir: string): string {
  const removed = existsSync(pluginDir);
  if (removed) rmSync(pluginDir, { recursive: true, force: true });
  return formatClaudeUninstallSummary(pluginDir, removed);
}

export class ClaudeHarness implements Harness {
  readonly name = "claude";

  install(): string {
    return installClaudeTo(defaultClaudePluginDir());
  }

  uninstall(): string {
    return uninstallClaudeTo(defaultClaudePluginDir());
  }
}
