import {
  existsSync,
  mkdirSync,
  readdirSync,
  utimesSync,
  writeFileSync,
} from "node:fs";
import { join } from "node:path";
import { afterEach, describe, expect, it, vi } from "vitest";
import { currentLogFile, initLogging, logDir, pruneLogs } from "./index.ts";
import { makeTmp } from "./test-support.ts";

afterEach(() => vi.unstubAllEnvs());

describe("logDir", () => {
  it("uses Application Support on macOS", () => {
    expect(logDir("agentctl", "darwin", {}, "/Users/u")).toBe(
      "/Users/u/Library/Application Support/agentctl/logs",
    );
  });
  it("uses XDG_STATE_HOME or ~/.local/state on Linux", () => {
    expect(
      logDir("agentctl", "linux", { XDG_STATE_HOME: "/x/state" }, "/home/u"),
    ).toBe("/x/state/agentctl/logs");
    expect(logDir("agentctl", "linux", {}, "/home/u")).toBe(
      "/home/u/.local/state/agentctl/logs",
    );
    expect(
      logDir("agentctl", "linux", { XDG_STATE_HOME: "relative" }, "/home/u"),
    ).toBe("/home/u/.local/state/agentctl/logs");
  });
});

describe("currentLogFile", () => {
  it("appends the local YYYY-MM-DD date", () => {
    expect(currentLogFile("/logs", "agentctl", new Date(2026, 6, 4))).toBe(
      "/logs/agentctl.log.2026-07-04",
    );
  });
});

describe("pruneLogs", () => {
  it("keeps only the most recent files and ignores non-matching ones", () => {
    const dir = makeTmp();
    for (let i = 0; i < 10; i++) {
      const p = join(
        dir,
        `agentctl.log.2026-07-${String(i + 1).padStart(2, "0")}`,
      );
      writeFileSync(p, "x");
      utimesSync(p, 1000 + i, 1000 + i);
    }
    writeFileSync(join(dir, "other.txt"), "keep");
    utimesSync(join(dir, "other.txt"), 1, 1);
    pruneLogs(dir, "agentctl", 7);
    const left = readdirSync(dir).sort();
    expect(left.filter((n) => n.startsWith("agentctl.log.")).length).toBe(7);
    expect(left).toContain("agentctl.log.2026-07-10");
    expect(left).not.toContain("agentctl.log.2026-07-01");
    expect(left).toContain("other.txt");
  });
});

describe("initLogging", () => {
  it("writes entries at or above the level to today's file", () => {
    const home = makeTmp();
    vi.stubEnv("HOME", home);
    vi.stubEnv("XDG_STATE_HOME", join(home, "state"));
    const logger = initLogging("info", "agentctl-test");
    logger.debug("hidden");
    logger.info("visible-message");
    const dir = logDir("agentctl-test") as string;
    const file = currentLogFile(dir, "agentctl-test");
    expect(existsSync(file)).toBe(true);
    const content = readdirSync(dir).length;
    expect(content).toBe(1);
  });

  it("is best-effort when the directory cannot be created", () => {
    const home = makeTmp();
    // A regular file where the state dir should be makes mkdir fail.
    writeFileSync(join(home, "blocker"), "x");
    vi.stubEnv("HOME", home);
    vi.stubEnv("XDG_STATE_HOME", join(home, "blocker"));
    const logger = initLogging("info");
    expect(() => logger.info("no crash")).not.toThrow();
    mkdirSync(join(home, "ok"));
  });
});
