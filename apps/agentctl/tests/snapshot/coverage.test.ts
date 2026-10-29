import type { App } from "@agentctl/core";
import type { Command } from "commander";
import { describe, expect, it } from "vitest";
import { buildProgram } from "../../src/program.ts";
import { CASES } from "./cases.ts";

function commandPaths(cmd: Command, prefix: string[] = []): string[][] {
  return cmd.commands.flatMap((c) => {
    if (c.name() === "help") return [[...prefix, c.name()]];
    const path = [...prefix, c.name()];
    return [path, ...commandPaths(c, path)];
  });
}

const startsWith = (args: string[], path: string[]) =>
  path.every((p, i) => args[i] === p);

describe("snapshot coverage", () => {
  const program = buildProgram(
    { createApp: () => ({}) as App, write: () => {} },
    ["agentctl"],
  );
  const paths = commandPaths(program);

  it.each(paths.map((p) => [p.join(" "), p] as const))(
    "'%s' has an invocation case",
    (_label, path) => {
      const hit = CASES.some(
        (c) => c.group !== "help" && startsWith(c.args, path),
      );
      expect(hit).toBe(true);
    },
  );

  it.each(paths.map((p) => [p.join(" "), p] as const))(
    "'%s' has a --help case",
    (_label, path) => {
      if (path[0] === "help") return;
      expect(
        CASES.some((c) => c.group === "help" && startsWith(c.args, path)),
      ).toBe(true);
    },
  );

  it("has unique snapshot names", () => {
    const keys = CASES.map((c) => `${c.group}/${c.name}`);
    expect(new Set(keys).size).toBe(keys.length);
  });
});
