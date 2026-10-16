/// <reference path="../assets.d.ts" />
import { existsSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, join } from "node:path";
import type { Harness } from "@agentctl/core";
import { tildeCollapse } from "../paths.ts";
import manifestAsset from "./assets/.claude-plugin/plugin.json?raw";
import hooksJsonAsset from "./assets/hooks/hooks.json?raw";

const PLUGIN_NAME = "agentctl";
interface ClaudeHookCommand {
  command: string;
  async?: boolean;
}
interface ClaudeHookEntry {
  matcher?: string;
  hooks: ClaudeHookCommand[];
}
type ClaudeHooksFile = { hooks: Record<string, ClaudeHookEntry[]> };

/** Parsed `assets/hooks/hooks.json`, for the install summary. */
function claudeHookRows(): {
  event: string;
  matcher?: string;
  command: string;
  async: boolean;
}[] {
  const file = JSON.parse(hooksJsonAsset) as ClaudeHooksFile;
  return Object.entries(file.hooks).flatMap(([event, entries]) =>
    entries.flatMap((entry) =>
      entry.hooks.map((h) => ({
        event,
        matcher: entry.matcher,
        command: h.command,
        async: h.async === true,
      })),
    ),
  );
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
  const rows = claudeHookRows();
  const lines = [
    `Installed agentctl plugin for claude (${rows.length} hook entries):`,
    `  + ${tildeCollapse(manifestPath(pluginDir))}`,
    `  + ${tildeCollapse(hooksJsonPath(pluginDir))}`,
  ];
  for (const hook of rows) {
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
  writeFileSync(manifest, manifestAsset);

  const hooks = hooksJsonPath(pluginDir);
  mkdirSync(dirname(hooks), { recursive: true });
  writeFileSync(hooks, hooksJsonAsset);

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
