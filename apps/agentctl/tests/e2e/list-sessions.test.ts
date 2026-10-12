import { existsSync, mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { useTmuxEnv } from "./support/tmux-env.ts";

// Tests that need live panes require host -> container Unix socket access,
// which fails on macOS Docker Desktop (VirtioFS); they are skipped there.
const needsSocket = process.platform === "darwin" ? it.skip : it;

describe("agentctl list", () => {
  const env = useTmuxEnv();

  it("no agent panes exits zero with no output", () => {
    const out = env().agentctl(["list"]);
    expect(out.status, out.stderr).toBe(0);
    expect(out.stdout).toBe("");
  });

  it("tmux not found exits non-zero with message", () => {
    const emptyBin = mkdtempSync(join(tmpdir(), "agentctl-empty-"));
    const out = env().agentctl(["list"], { env: { PATH: emptyBin } });
    expect(out.status).not.toBe(0);
    expect(out.stderr).toContain("tmux not found");
  });

  it("no server running exits zero with no output", () => {
    const e = env();
    const out = e.agentctl([
      "list",
      "--server",
      join(e.workspacePath, "no-server.sock"),
    ]);
    expect(out.status, out.stderr).toBe(0);
    expect(out.stdout).toBe("");
  });

  needsSocket("agent pane with state file shows state", async () => {
    const e = env();
    const paneId = await e.createFakeAgentPane("claude");
    e.writeStateFile(paneId, "claude", "working");
    const out = e.agentctl(["list"]);
    expect(out.status, out.stderr).toBe(0);
    expect(out.stdout).toContain("working");
    expect(out.stdout).toContain("claude");
  });

  needsSocket("agent pane without state file shows unknown", async () => {
    const e = env();
    await e.createFakeAgentPane("claude");
    const out = e.agentctl(["list"]);
    expect(out.status, out.stderr).toBe(0);
    expect(out.stdout).toContain("unknown");
  });

  it("orphaned state file is deleted", () => {
    const e = env();
    const phantom = "%9999";
    e.writeStateFile(phantom, "claude", "idle");
    expect(existsSync(e.stateFilePath(phantom))).toBe(true);
    const out = e.agentctl(["list"]);
    expect(out.status, out.stderr).toBe(0);
    expect(existsSync(e.stateFilePath(phantom))).toBe(false);
  });

  it("contradictory harness state file is deleted", async () => {
    const e = env();
    const paneId = await e.createFakeAgentPane("claude");
    e.writeStateFile(paneId, "pi", "idle");
    expect(existsSync(e.stateFilePath(paneId))).toBe(true);
    const out = e.agentctl(["list"]);
    expect(out.status, out.stderr).toBe(0);
    expect(existsSync(e.stateFilePath(paneId))).toBe(false);
  });

  needsSocket("unknown harness state file is not deleted", async () => {
    const e = env();
    const paneId = await e.createFakeAgentPane("claude");
    e.writeStateFile(paneId, "unknown", "idle");
    const out = e.agentctl(["list"]);
    expect(out.status, out.stderr).toBe(0);
    expect(existsSync(e.stateFilePath(paneId))).toBe(true);
  });

  it("--columns with no panes exits zero with no output", () => {
    const out = env().agentctl([
      "list",
      "--columns",
      "status,agent,updatedAt,activeTools",
    ]);
    expect(out.status, out.stderr).toBe(0);
    expect(out.stdout).toBe("");
  });

  it("--sort-by new keys with no panes exits zero", () => {
    for (const key of ["status", "updatedAt", "activeTools"]) {
      const out = env().agentctl(["list", "--sort-by", key]);
      expect(out.status, out.stderr).toBe(0);
    }
  });

  it("--sort-by old names rejected", () => {
    for (const key of ["state", "updated-at"]) {
      expect(env().agentctl(["list", "--sort-by", key]).status).not.toBe(0);
    }
  });

  needsSocket("--columns custom subset and order end to end", async () => {
    const e = env();
    const paneId = await e.createFakeAgentPane("claude");
    e.writeStateFile(paneId, "claude", "working");
    const out = e.agentctl([
      "list",
      "--columns",
      "status,agent,activeTools,updatedAt",
    ]);
    expect(out.status, out.stderr).toBe(0);
    const line = out.stdout.split("\n")[0] ?? "";
    expect(line.trimStart().startsWith("working")).toBe(true);
    expect(line).toContain("claude");
  });
});
