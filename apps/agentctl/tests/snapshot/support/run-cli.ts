import { vi } from "vitest";
import { WORLDS, type WorldName } from "../fixtures/worlds.ts";
import { type Recorded, setShared } from "./world.ts";

export interface CliCase {
  /** Snapshot sub-directory. */
  group: string;
  /** Snapshot file name (sanitised). */
  name: string;
  args: string[];
  world?: WorldName;
  /** Environment variables for the run. */
  env?: Record<string, string>;
  /** Pretend stdout is a terminal (enables colors). */
  tty?: boolean;
}

export interface CliResult extends Recorded {
  exitCode: number;
  stdout: string;
  stderr: string;
}

const ENV_VARS = ["TMUX_PANE", "AGENTCTL_TMUX_SOCKET"];

/**
 * Run the real `src/cli.ts` in-process (fresh module graph per call) against
 * the mocked adapters, capturing stdout, stderr and the exit code.
 */
export async function runCli(c: CliCase): Promise<CliResult> {
  const recorded: Recorded = { effects: [] };
  setShared({ world: WORLDS[c.world ?? "empty"], recorded });

  const stdout: string[] = [];
  const stderr: string[] = [];
  vi.spyOn(process.stdout, "write").mockImplementation((chunk) => {
    stdout.push(String(chunk));
    return true;
  });
  vi.spyOn(process.stderr, "write").mockImplementation((chunk) => {
    stderr.push(String(chunk));
    return true;
  });

  for (const name of ENV_VARS) vi.stubEnv(name, undefined);
  for (const [k, v] of Object.entries(c.env ?? {})) vi.stubEnv(k, v);

  const originalArgv = process.argv;
  const originalTty = process.stdout.isTTY;
  process.argv = ["node", "agentctl", ...c.args];
  process.stdout.isTTY = c.tty === true;
  process.exitCode = undefined;

  try {
    vi.resetModules();
    await import("../../../src/cli.ts");
    return {
      exitCode: Number(process.exitCode ?? 0),
      stdout: stdout.join(""),
      stderr: stderr.join(""),
      effects: recorded.effects,
    };
  } finally {
    // Never leak a non-zero exit code into the vitest process itself.
    process.exitCode = undefined;
    process.argv = originalArgv;
    process.stdout.isTTY = originalTty;
    vi.unstubAllEnvs();
    vi.restoreAllMocks();
  }
}

/** Stable, reviewable text form of a result. */
export function renderSnapshot(c: CliCase, r: CliResult): string {
  const section = (title: string, body: string) =>
    `--- ${title}\n${body === "" ? "(empty)\n" : body.endsWith("\n") ? body : `${body}\n`}`;
  const env = Object.entries(c.env ?? {}).map(([k, v]) => `${k}=${v}`);
  return [
    `$ agentctl ${c.args.join(" ")}`.trimEnd(),
    `world: ${c.world ?? "empty"}${c.tty ? " (tty)" : ""}${env.length ? ` env: ${env.join(" ")}` : ""}`,
    "",
    section("exit code", String(r.exitCode)),
    section("stdout", r.stdout),
    section("stderr", r.stderr),
    section("effects", r.effects.join("\n")),
  ].join("\n");
}
