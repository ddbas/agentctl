/**
 * Shared e2e helpers.
 *
 * Every e2e test that touches tmux MUST use `TmuxEnv`: it backs each test with
 * a Docker container (via testcontainers) running an isolated tmux server on a
 * Unix socket in a bind-mounted temp dir. The agentctl binary under test runs
 * on the host and is directed at that socket via `AGENTCTL_TMUX_SOCKET`.
 *
 * The binary is selected via `AGENTCTL_BIN` (default: the built TS bundle,
 * `apps/agentctl/dist/cli.cjs`). A `.js`/`.cjs`/`.mjs` path is run through the
 * current node executable; anything else is executed directly.
 *
 * Build the Docker image with `docker build -t am-tmux-test:latest tests/`.
 */

import { type SpawnSyncReturns, spawnSync } from "node:child_process";
import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  realpathSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import {
  GenericContainer,
  type StartedTestContainer,
  Wait,
} from "testcontainers";
import { afterEach, beforeEach } from "vitest";

const here = dirname(fileURLToPath(import.meta.url));
const appRoot = resolve(here, "../../..");

export const TMUX_IMAGE = "am-tmux-test:latest";

/** Resolved path of the binary under test. */
export function agentctlBin(): string {
  const configured = process.env.AGENTCTL_BIN;
  return configured
    ? resolve(process.cwd(), configured)
    : join(appRoot, "dist", "cli.cjs");
}

function isScript(path: string): boolean {
  return /\.(c|m)?js$/.test(path);
}

export interface RunResult {
  status: number | null;
  stdout: string;
  stderr: string;
}

export interface RunOptions {
  env?: Record<string, string | undefined>;
  input?: string;
}

function toResult(r: SpawnSyncReturns<string>): RunResult {
  return { status: r.status, stdout: r.stdout ?? "", stderr: r.stderr ?? "" };
}

// ── hello-world canary ───────────────────────────────────────────────────────

export async function startGenericContainer(): Promise<StartedTestContainer> {
  return await new GenericContainer("hello-world:latest")
    .withWaitStrategy(Wait.forLogMessage("Hello from Docker!"))
    .start();
}

// ── TmuxEnv ──────────────────────────────────────────────────────────────────

export class TmuxEnv {
  private constructor(
    private readonly container: StartedTestContainer,
    private readonly tmpRoot: string,
    /** Canonical host path of the bind-mounted workspace (`/workspace`). */
    readonly workspacePath: string,
    readonly socketPath: string,
    /** Isolated XDG data home for this test. */
    readonly xdgPath: string,
    /** Directory containing an `agentctl` entry pointing at the binary. */
    readonly binDir: string,
  ) {}

  static async create(): Promise<TmuxEnv> {
    // Canonicalise so Docker Desktop's bind mount works on macOS.
    const tmpRoot = realpathSync(mkdtempSync(join(tmpdir(), "agentctl-e2e-")));
    const workspacePath = join(tmpRoot, "workspace");
    mkdirSync(workspacePath);

    const container = await new GenericContainer(TMUX_IMAGE)
      // tmux only serves clients with the same uid as the server, so run the
      // server as the host user that will invoke agentctl.
      .withUser(`${process.getuid?.() ?? 0}:${process.getgid?.() ?? 0}`)
      .withEntrypoint(["sh"])
      .withCommand([
        "-c",
        "tmux -S /workspace/tmux.sock new-session -d -s _init && echo 'tmux-ready' && tail -f /dev/null",
      ])
      .withBindMounts([{ source: workspacePath, target: "/workspace" }])
      .withWaitStrategy(Wait.forLogMessage("tmux-ready"))
      .start();

    const xdgPath = join(workspacePath, "xdg");
    mkdirSync(xdgPath, { recursive: true });

    // Hook commands invoke `agentctl` via PATH, so expose the binary under
    // that name regardless of what AGENTCTL_BIN is called.
    const binDir = join(tmpRoot, "bin");
    mkdirSync(binDir);
    symlinkSync(agentctlBin(), join(binDir, "agentctl"));

    return new TmuxEnv(
      container,
      tmpRoot,
      workspacePath,
      join(workspacePath, "tmux.sock"),
      xdgPath,
      binDir,
    );
  }

  async dispose(): Promise<void> {
    await this.container.stop({ remove: true, timeout: 1 });
    rmSync(this.tmpRoot, { recursive: true, force: true });
  }

  /** Environment for invoking agentctl: isolated socket + XDG, no TMUX vars. */
  baseEnv(extra: Record<string, string | undefined> = {}): NodeJS.ProcessEnv {
    const env: NodeJS.ProcessEnv = {
      ...process.env,
      AGENTCTL_TMUX_SOCKET: this.socketPath,
      XDG_DATA_HOME: this.xdgPath,
    };
    delete env.TMUX;
    delete env.TMUX_PANE;
    for (const [k, v] of Object.entries(extra)) {
      if (v === undefined) delete env[k];
      else env[k] = v;
    }
    return env;
  }

  /** Run the binary under test synchronously. */
  agentctl(args: string[], options: RunOptions = {}): RunResult {
    const bin = agentctlBin();
    const [cmd, argv] = isScript(bin)
      ? [process.execPath, [bin, ...args]]
      : [bin, args];
    return toResult(
      spawnSync(cmd, argv, {
        env: this.baseEnv(options.env),
        input: options.input,
        encoding: "utf8",
      }),
    );
  }

  /** Run a command inside the container; throws on non-zero exit. */
  async exec(args: string[]): Promise<void> {
    const r = await this.container.exec(args);
    if (r.exitCode !== 0) {
      throw new Error(
        `command ${JSON.stringify(args)} exited with status ${r.exitCode}:\n${r.stderr}`,
      );
    }
  }

  /** Create a fake agent pane in the container's tmux; returns its pane ID. */
  async createFakeAgentPane(agent: string): Promise<string> {
    await this.exec([
      "tmux",
      "-S",
      "/workspace/tmux.sock",
      "new-window",
      "-n",
      agent,
    ]);
    const paneIdFile = `pane_id_${agent}.txt`;
    await this.exec([
      "sh",
      "-c",
      `tmux -S /workspace/tmux.sock list-panes -t '_init:${agent}' -F '#{pane_id}' > /workspace/${paneIdFile}`,
    ]);
    const paneId = readFileSync(
      join(this.workspacePath, paneIdFile),
      "utf8",
    ).trim();
    if (!paneId) throw new Error("got empty pane_id");
    await this.exec([
      "tmux",
      "-S",
      "/workspace/tmux.sock",
      "send-keys",
      "-t",
      paneId,
      `exec bash -c 'exec -a ${agent} bash -c "sleep 9999; :"'`,
      "Enter",
    ]);
    // Block until tmux reports the pane running the agent (no fixed sleep).
    await this.exec([
      "sh",
      "-c",
      `until [ "$(tmux -S /workspace/tmux.sock display-message -p -t ${paneId} '#{pane_current_command}')" = '${agent}' ]; do sleep 0.05; done`,
    ]);
    return paneId;
  }

  /** Write a minimal state file for the pane under this env's XDG dir. */
  writeStateFile(paneId: string, harness: string, status: string): void {
    const dir = join(this.xdgPath, "agentctl", "agents", paneId);
    mkdirSync(dir, { recursive: true });
    writeFileSync(
      join(dir, "state.json"),
      `{"pane_id":"${paneId}","harness":"${harness}","status":"${status}","updated_at":"2026-01-01T00:00:00Z"}`,
    );
  }

  stateFilePath(paneId: string): string {
    return join(this.xdgPath, "agentctl", "agents", paneId, "state.json");
  }

  eventsFilePath(paneId: string): string {
    return join(this.xdgPath, "agentctl", "agents", paneId, "events.jsonl");
  }

  /** Run an arbitrary shell command with the binary dir prepended to PATH. */
  runShell(
    command: string,
    env: Record<string, string | undefined>,
    input?: string,
  ): RunResult {
    return toResult(
      spawnSync("sh", ["-c", command], {
        env: {
          ...this.baseEnv(env),
          PATH: `${this.binDir}:${process.env.PATH ?? ""}`,
        },
        input,
        encoding: "utf8",
      }),
    );
  }
}

/** Registers per-test setup/teardown and returns an accessor for the env. */
export function useTmuxEnv(): () => TmuxEnv {
  let env: TmuxEnv | undefined;
  beforeEach(async () => {
    env = await TmuxEnv.create();
  });
  afterEach(async () => {
    await env?.dispose();
    env = undefined;
  });
  return () => {
    if (!env) throw new Error("TmuxEnv not initialised");
    return env;
  };
}
