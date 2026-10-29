import { existsSync, mkdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { describe, expect, it } from "vitest";
import { useTmuxEnv } from "./support/tmux-env.ts";

const pluginDir = (home: string) => join(home, ".claude", "skills", "agentctl");

interface HookEntry {
  matcher?: string;
  hooks: { command?: string }[];
}

describe("agentctl install", () => {
  const env = useTmuxEnv();

  const fakeHome = (name: string) => {
    const home = join(env().workspacePath, name);
    mkdirSync(home, { recursive: true });
    return home;
  };

  it("unknown harness exits two", () => {
    const out = env().agentctl(["install", "not-a-real-harness"]);
    expect(out.status).toBe(2);
    expect(out.stderr).toContain("not-a-real-harness");
  });

  it("unknown harness names all exit two", () => {
    for (const harness of ["aider", "my-custom-harness"]) {
      expect(env().agentctl(["install", harness]).status).toBe(2);
    }
  });

  // ── Pi ──

  it("install pi creates extension file", () => {
    const home = fakeHome("home");
    const ext = join(home, ".pi", "agent", "extensions", "agentctl.ts");
    expect(existsSync(ext)).toBe(false);

    const out = env().agentctl(["install", "pi"], { env: { HOME: home } });
    expect(out.status, out.stderr).toBe(0);
    expect(existsSync(dirname(ext))).toBe(true);
    expect(existsSync(ext)).toBe(true);

    const content = readFileSync(ext, "utf8");
    for (const h of [
      "session_start",
      "before_agent_start",
      "agent_settled",
      "session_shutdown",
    ]) {
      expect(content, `missing ${h} handler`).toContain(`pi.on("${h}"`);
    }
    expect(out.stdout).toContain("+");
  });

  it("install pi is idempotent", async () => {
    const home = fakeHome("home2");
    const ext = join(home, ".pi", "agent", "extensions", "agentctl.ts");

    const out1 = env().agentctl(["install", "pi"], { env: { HOME: home } });
    expect(out1.status).toBe(0);
    const first = readFileSync(ext, "utf8");
    const mtime1 = statSync(ext).mtimeMs;

    await new Promise((r) => setTimeout(r, 20));

    const out2 = env().agentctl(["install", "pi"], { env: { HOME: home } });
    expect(out2.status).toBe(0);
    expect(readFileSync(ext, "utf8")).toBe(first);
    expect(statSync(ext).mtimeMs).toBe(mtime1);
    expect(out2.stdout).toContain("=");
  });

  // ── Claude ──

  it("install claude creates plugin directory", () => {
    const home = fakeHome("home_claude");
    const dir = pluginDir(home);
    const hooksJson = join(dir, "hooks", "hooks.json");
    const manifestPath = join(dir, ".claude-plugin", "plugin.json");
    expect(existsSync(dir)).toBe(false);

    const out = env().agentctl(["install", "claude"], { env: { HOME: home } });
    expect(out.status, out.stderr).toBe(0);
    expect(existsSync(manifestPath)).toBe(true);
    expect(existsSync(hooksJson)).toBe(true);

    expect(JSON.parse(readFileSync(manifestPath, "utf8")).name).toBe(
      "agentctl",
    );

    const hooks = JSON.parse(readFileSync(hooksJson, "utf8")).hooks;
    for (const event of [
      "SessionStart",
      "UserPromptSubmit",
      "Stop",
      "StopFailure",
      "SessionEnd",
    ]) {
      expect(hooks[event], event).toHaveLength(1);
    }
    expect(hooks.Notification).toBeUndefined();
    for (const event of ["PreToolUse", "PostToolUse", "PostToolUseFailure"]) {
      expect(hooks[event], event).toHaveLength(2);
    }
    expect(out.stdout).toContain("plugin.json");
    expect(out.stdout).toContain("hooks.json");
  });

  it("reinstall claude produces identical content", () => {
    const home = fakeHome("home_claude2");
    const hooksJson = join(pluginDir(home), "hooks", "hooks.json");

    expect(
      env().agentctl(["install", "claude"], { env: { HOME: home } }).status,
    ).toBe(0);
    const first = readFileSync(hooksJson, "utf8");
    expect(
      env().agentctl(["install", "claude"], { env: { HOME: home } }).status,
    ).toBe(0);
    expect(readFileSync(hooksJson, "utf8")).toBe(first);
  });

  it("uninstall claude removes plugin directory", () => {
    const home = fakeHome("home_claude_uninstall");
    const dir = pluginDir(home);

    env().agentctl(["install", "claude"], { env: { HOME: home } });
    expect(existsSync(dir)).toBe(true);

    expect(
      env().agentctl(["uninstall", "claude"], { env: { HOME: home } }).status,
    ).toBe(0);
    expect(existsSync(dir)).toBe(false);
    expect(
      env().agentctl(["uninstall", "claude"], { env: { HOME: home } }).status,
    ).toBe(0);
  });

  it("claude tool call tracking updates active_tools", () => {
    const e = env();
    const home = fakeHome("home_claude3");
    const hooksJson = join(pluginDir(home), "hooks", "hooks.json");

    expect(
      e.agentctl(["install", "claude"], { env: { HOME: home } }).status,
    ).toBe(0);
    const hooks = JSON.parse(readFileSync(hooksJson, "utf8")).hooks as Record<
      string,
      HookEntry[]
    >;

    const find = (event: string, pred: (cmd: string) => boolean) => {
      for (const entry of hooks[event] ?? []) {
        const cmd = entry.hooks[0]?.command;
        if (cmd && pred(cmd)) return cmd;
      }
      throw new Error(`untyped ${event} command should be present`);
    };
    const preCmd = find(
      "PreToolUse",
      (c) => c.includes("TOOL_CALL_START") && !c.includes("--type"),
    );
    const postCmd = find(
      "PostToolUse",
      (c) =>
        c.includes("TOOL_CALL_RESULT") &&
        !c.includes("--type") &&
        !c.includes("perm-"),
    );

    const pane = "%88";
    const payload = '{"tool_use_id":"tu1","session_id":"s1"}';
    const run = (cmd: string) => e.runShell(cmd, { TMUX_PANE: pane }, payload);
    const activeTools = () =>
      JSON.parse(readFileSync(e.stateFilePath(pane), "utf8")).active_tools;

    const pre = run(preCmd);
    expect(pre.status, pre.stderr).toBe(0);
    expect(activeTools()).toBe(1);

    const post = run(postCmd);
    expect(post.status, post.stderr).toBe(0);
    expect(activeTools()).toBe(0);
  });
});
