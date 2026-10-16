import { homedir } from "node:os";
import {
  type App,
  App as AppClass,
  type LogLevel,
  nowTimestamp,
  type Runtime,
  tildeCollapse,
} from "@agentctl/core";
import { ClaudeHarness } from "@agentctl/harness-claude";
import { PiHarness } from "@agentctl/harness-pi";
import { currentLogFile, initLogging, logDir } from "@agentctl/logging";
import { FileAgentRepository } from "@agentctl/state-fs";
import { ExecTmux } from "@agentctl/tmux";
import { CommanderError } from "commander";
import { LONG_HELP, SHORT_HELP } from "./help.ts";
import { buildProgram, type ProgramDeps } from "./program.ts";
import { VERSION } from "./version.ts";

const write = (text: string): void => {
  process.stdout.write(text);
};

/** Composition root: the only place concrete infrastructure is chosen. */
function createApp(logLevel: LogLevel, argv: string[]): App {
  // Best-effort file logging; failures are non-fatal.
  const logger = initLogging(logLevel);
  logger.debug(`CLI invocation: agentctl ${JSON.stringify(argv)}`);

  const runtime: Runtime = {
    env: (name) => process.env[name],
    nowTimestamp,
    stdoutIsTerminal: process.stdout.isTTY === true,
    version: VERSION,
    logDir: () => logDir(),
    currentLogFile: (dir) => currentLogFile(dir),
    tildeCollapse: (path) => tildeCollapse(path, homedir()),
  };

  return new AppClass({
    repo: FileAgentRepository.xdg(),
    tmux: new ExecTmux(),
    harnesses: [new PiHarness(), new ClaudeHarness()],
    runtime,
    logger,
  });
}

/** Root-level help variants handled before commander (clap parity). */
function rootHelp(args: string[]): string | undefined {
  if (
    args.length === 0 ||
    (args.length === 1 && ["help", "--help"].includes(args[0] as string))
  ) {
    return LONG_HELP;
  }
  if (args.length === 1 && args[0] === "-h") return SHORT_HELP;
  return undefined;
}

function main(): void {
  const args = process.argv.slice(2);
  const help = rootHelp(args);
  if (help !== undefined) {
    write(help);
    return;
  }

  const deps: ProgramDeps = { createApp, write };
  try {
    buildProgram(deps, process.argv.slice(1)).parse(process.argv);
  } catch (err) {
    if (err instanceof CommanderError) {
      // Usage errors exit 2 like clap; help/version exit 0.
      process.exitCode = err.exitCode === 0 ? 0 : 2;
      return;
    }
    process.stderr.write(
      `error: ${err instanceof Error ? err.message : String(err)}\n`,
    );
    process.exitCode = 1;
  }
}

main();
