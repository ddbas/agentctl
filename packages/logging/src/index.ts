import { mkdirSync, readdirSync, statSync, unlinkSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";
import type { Logger, LogLevel } from "@agentctl/core";
import pino from "pino";

const APP_NAME = "agentctl";

/**
 * Resolve the platform log directory: the XDG state dir (Linux:
 * `$XDG_STATE_HOME` or `~/.local/state`), else the local data dir
 * (macOS: `~/Library/Application Support`), joined with `<app>/logs`.
 * Pure: no I/O.
 */
export function logDir(
  appName: string = APP_NAME,
  platform: NodeJS.Platform = process.platform,
  env: NodeJS.ProcessEnv = process.env,
  home: string = homedir(),
): string | undefined {
  if (!home) return undefined;
  let base: string;
  if (platform === "darwin") {
    base = join(home, "Library", "Application Support");
  } else if (platform === "win32") {
    base = env.LOCALAPPDATA ?? join(home, "AppData", "Local");
  } else {
    const xdg = env.XDG_STATE_HOME;
    base = xdg?.startsWith("/") ? xdg : join(home, ".local", "state");
  }
  return join(base, appName, "logs");
}

function pad(n: number): string {
  return String(n).padStart(2, "0");
}

/** `<dir>/<app>.log.YYYY-MM-DD` using the local date. Pure: no I/O. */
export function currentLogFile(
  dir: string,
  appName: string = APP_NAME,
  now: Date = new Date(),
): string {
  const date = `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`;
  return join(dir, `${appName}.log.${date}`);
}

/** Keep only the `retentionCount` most recently modified `<app>.log.*` files. */
export function pruneLogs(
  dir: string,
  appName: string,
  retentionCount: number,
): void {
  const prefix = `${appName}.log.`;
  let names: string[];
  try {
    names = readdirSync(dir);
  } catch {
    return;
  }
  const files: { path: string; mtime: number }[] = [];
  for (const name of names) {
    if (!name.startsWith(prefix)) continue;
    const path = join(dir, name);
    try {
      files.push({ path, mtime: statSync(path).mtimeMs });
    } catch {
      // ignore unreadable entries
    }
  }
  files.sort((a, b) => b.mtime - a.mtime);
  for (const { path } of files.slice(retentionCount)) {
    try {
      unlinkSync(path);
    } catch {
      // best-effort
    }
  }
}

const NOOP_LOGGER: Logger = {
  debug() {},
  info() {},
  warn() {},
  error() {},
};

/**
 * Initialise file logging (pino, synchronous file destination). Best-effort:
 * returns a no-op logger when the directory cannot be resolved or written.
 */
export function initLogging(
  level: LogLevel,
  appName: string = APP_NAME,
): Logger {
  try {
    const dir = logDir(appName);
    if (!dir) return NOOP_LOGGER;
    mkdirSync(dir, { recursive: true });
    pruneLogs(dir, appName, 7);
    const destination = pino.destination({
      dest: currentLogFile(dir, appName),
      sync: true,
      mkdir: true,
    });
    const log = pino({ level, base: undefined }, destination);
    // Surface (and swallow) late write failures instead of crashing.
    destination.on("error", () => {});
    return {
      debug: (msg) => log.debug(msg),
      info: (msg) => log.info(msg),
      warn: (msg) => log.warn(msg),
      error: (msg) => log.error(msg),
    };
  } catch {
    return NOOP_LOGGER;
  }
}
