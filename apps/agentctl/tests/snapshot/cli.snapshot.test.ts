import { describe, expect, it, vi } from "vitest";
import { CASES } from "./cases.ts";
import { renderSnapshot, runCli } from "./support/run-cli.ts";
import { FAKE_HOME, FIXED_NOW } from "./support/world.ts";

// Replace every adapter / ambient dependency of `src/cli.ts`. Factories are
// lazy so they resolve after `vi.resetModules()` in `runCli`.
vi.mock(
  "@agentctl/tmux",
  async () => (await import("./mocks/tmux.ts")).tmuxMock,
);
vi.mock(
  "@agentctl/state-fs",
  async () => (await import("./mocks/state-fs.ts")).stateFsMock,
);
vi.mock(
  "@agentctl/logging",
  async () => (await import("./mocks/logging.ts")).loggingMock,
);
vi.mock(
  "@agentctl/harness-pi",
  async () => (await import("./mocks/harnesses.ts")).harnessPiMock,
);
vi.mock(
  "@agentctl/harness-claude",
  async () => (await import("./mocks/harnesses.ts")).harnessClaudeMock,
);
vi.mock("@agentctl/core", async (importOriginal) => ({
  ...(await importOriginal<typeof import("@agentctl/core")>()),
  nowTimestamp: () => FIXED_NOW,
}));
vi.mock("node:os", async (importOriginal) => {
  const os = await importOriginal<typeof import("node:os")>();
  return {
    ...os,
    default: { ...os, homedir: () => FAKE_HOME },
    homedir: () => FAKE_HOME,
  };
});

const slug = (s: string) => s.replace(/[^A-Za-z0-9._-]+/g, "_");

describe("CLI output snapshots", () => {
  it.each(CASES)("$group / $name", async (c) => {
    const result = await runCli(c);
    await expect(renderSnapshot(c, result)).toMatchFileSnapshot(
      `__snapshots__/${slug(c.group)}/${slug(c.name)}.txt`,
    );
  });
});
