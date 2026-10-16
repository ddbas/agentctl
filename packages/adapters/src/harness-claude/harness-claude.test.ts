import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { afterEach, describe, expect, it, vi } from "vitest";
import { makeTmp, readFixture } from "../test-support.ts";
import manifestAsset from "./assets/.claude-plugin/plugin.json?raw";
import hooksAsset from "./assets/hooks/hooks.json?raw";
import {
  formatClaudeUninstallSummary,
  installClaudeTo,
  uninstallClaudeTo,
} from "./index.ts";

afterEach(() => vi.unstubAllEnvs());

const pluginDir = (home: string) => join(home, ".claude", "skills", "agentctl");
const assetHooks = (): Hooks => JSON.parse(hooksAsset).hooks;

type Hooks = Record<
  string,
  { matcher?: string; hooks: { async?: boolean; command: string }[] }[]
>;
const readHooks = (dir: string): Hooks =>
  JSON.parse(readFileSync(join(dir, "hooks", "hooks.json"), "utf8")).hooks;

describe("claude harness", () => {
  it("copies the asset files verbatim", () => {
    const dir = pluginDir(makeTmp());
    installClaudeTo(dir);
    expect(readFileSync(join(dir, "hooks", "hooks.json"), "utf8")).toBe(
      hooksAsset,
    );
    expect(
      readFileSync(join(dir, ".claude-plugin", "plugin.json"), "utf8"),
    ).toBe(manifestAsset);
  });

  it("install summary matches the Rust output", () => {
    const home = makeTmp();
    vi.stubEnv("HOME", home);
    expect(installClaudeTo(pluginDir(home))).toBe(
      readFixture("install-claude.stdout").replace(/\n$/, ""),
    );
  });

  it("registers eleven entries across the expected events", () => {
    const hooks = assetHooks();
    expect(Object.values(hooks).flat()).toHaveLength(11);
    for (const e of [
      "SessionStart",
      "UserPromptSubmit",
      "Stop",
      "StopFailure",
      "SessionEnd",
    ]) {
      expect(hooks[e]).toHaveLength(1);
    }
    for (const e of ["PreToolUse", "PostToolUse", "PostToolUseFailure"]) {
      expect(hooks[e]).toHaveLength(2);
    }
    expect(hooks.Notification).toBeUndefined();
  });

  it("async flags, matchers and commands", () => {
    const hooks = assetHooks();
    const first = (e: string) => hooks[e]?.[0];
    expect(first("SessionStart")?.matcher).toBe("startup|resume|clear");
    expect(first("SessionStart")?.hooks[0]?.async).toBeUndefined();
    expect(first("SessionEnd")?.hooks[0]?.async).toBeUndefined();
    expect(first("UserPromptSubmit")?.hooks[0]?.async).toBe(true);
    expect(hooks.PreToolUse?.[1]?.matcher).toBe("AskUserQuestion|ExitPlanMode");
    expect(hooks.PreToolUse?.[1]?.hooks[0]?.command).toContain(
      "--type elicitation",
    );
    expect(first("Stop")?.hooks[0]?.command).toContain('--active-tools "$n"');
    expect(first("StopFailure")?.matcher).toBeUndefined();
    expect(hooks.PostToolUseFailure?.[0]?.hooks[0]?.command).toBe(
      hooks.PostToolUse?.[0]?.hooks[0]?.command,
    );
  });

  it("creates the skills dir and re-install replaces the directory", () => {
    const dir = pluginDir(makeTmp());
    installClaudeTo(dir);
    writeFileSync(join(dir, "stale.txt"), "x");
    installClaudeTo(dir);
    expect(existsSync(join(dir, "stale.txt"))).toBe(false);
    expect(readHooks(dir).Stop).toHaveLength(1);
  });

  it("uninstall removes only the plugin dir, is idempotent", () => {
    const home = makeTmp();
    const dir = pluginDir(home);
    installClaudeTo(dir);
    mkdirSync(join(home, ".claude", "skills", "other"), { recursive: true });
    expect(uninstallClaudeTo(dir)).toContain("  - ");
    expect(existsSync(dir)).toBe(false);
    expect(existsSync(join(home, ".claude", "skills", "other"))).toBe(true);
    expect(uninstallClaudeTo(dir)).toContain("(nothing to remove)");
  });

  it("uninstall summaries", () => {
    expect(formatClaudeUninstallSummary("/x/agentctl", true)).toBe(
      "Uninstalled agentctl plugin for claude:\n  - /x/agentctl",
    );
    expect(formatClaudeUninstallSummary("/x/agentctl", false)).toBe(
      "Uninstalled agentctl plugin for claude:\n  = /x/agentctl (nothing to remove)",
    );
  });
});
