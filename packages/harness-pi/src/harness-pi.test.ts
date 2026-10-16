import {
  existsSync,
  mkdirSync,
  readFileSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { join } from "node:path";
import { afterEach, describe, expect, it, vi } from "vitest";
import { installPiTo, PI_EXTENSION_CONTENT, uninstallPiTo } from "./index.ts";
import { makeTmp, readFixture } from "./test-support.ts";

afterEach(() => vi.unstubAllEnvs());

const extPath = (home: string) =>
  join(home, ".pi", "agent", "extensions", "agentctl.ts");

describe("pi harness", () => {
  it("writes content identical to the Rust-generated file", () => {
    expect(PI_EXTENSION_CONTENT).toBe(
      readFixture("home/.pi/agent/extensions/agentctl.ts"),
    );
  });

  it("install summary matches the Rust output", () => {
    const home = makeTmp();
    vi.stubEnv("HOME", home);
    expect(installPiTo(extPath(home))).toBe(
      readFixture("install-pi.stdout").replace(/\n$/, ""),
    );
  });

  it("creates the file and directory when absent", () => {
    const dest = extPath(makeTmp());
    installPiTo(dest);
    expect(readFileSync(dest, "utf8")).toBe(PI_EXTENSION_CONTENT);
  });

  it("is idempotent: second run does not rewrite and reports '='", () => {
    const dest = extPath(makeTmp());
    installPiTo(dest);
    const mtime = statSync(dest).mtimeMs;
    const summary = installPiTo(dest);
    expect(statSync(dest).mtimeMs).toBe(mtime);
    expect(summary).toContain("= ");
    expect(summary).toContain("already up to date");
  });

  it("overwrites a stale file and reports '~'", () => {
    const dest = extPath(makeTmp());
    mkdirSync(join(dest, ".."), { recursive: true });
    writeFileSync(dest, "// stale");
    expect(installPiTo(dest)).toContain("  ~ ");
    expect(readFileSync(dest, "utf8")).toBe(PI_EXTENSION_CONTENT);
  });

  it("has all handlers", () => {
    for (const h of [
      "session_start",
      "before_agent_start",
      "agent_settled",
      "tool_call",
      "tool_result",
      "session_shutdown",
    ]) {
      expect(PI_EXTENSION_CONTENT).toContain(`pi.on("${h}"`);
    }
  });

  it("uninstall removes the file but not the directory; no-op when absent", () => {
    const dest = extPath(makeTmp());
    installPiTo(dest);
    expect(uninstallPiTo(dest)).toContain("  - ");
    expect(existsSync(dest)).toBe(false);
    expect(existsSync(join(dest, ".."))).toBe(true);
    expect(uninstallPiTo(dest)).toContain("(nothing to remove)");
  });
});
