import { spawnSync } from "node:child_process";
import type { PaneEntry, Tmux } from "@agentctl/core";

const FORMAT =
  "#{pane_current_command}\t#{session_name}\t#{window_index}\t#{pane_index}\t#{pane_pid}\t#{pane_id}";

const MAX_BUFFER = 64 * 1024 * 1024;

/** Parse like Rust `u32::parse().unwrap_or(0)`. */
function parseU32(s: string): number {
  if (!/^\+?\d+$/.test(s)) return 0;
  const n = Number(s);
  return n <= 0xffff_ffff ? n : 0;
}

/** Split into at most `limit` parts; the last part keeps remaining separators. */
function splitN(line: string, sep: string, limit: number): string[] {
  const parts: string[] = [];
  let rest = line;
  while (parts.length < limit - 1) {
    const i = rest.indexOf(sep);
    if (i < 0) break;
    parts.push(rest.slice(0, i));
    rest = rest.slice(i + sep.length);
  }
  parts.push(rest);
  return parts;
}

/** Rust `str::lines`: split on `\n`, strip a trailing `\r`, no trailing empty line. */
function lines(text: string): string[] {
  const out = text.split("\n");
  if (out.at(-1) === "") out.pop();
  return out.map((l) => (l.endsWith("\r") ? l.slice(0, -1) : l));
}

/** Parse the output of `tmux list-panes -a -F <FORMAT>`. */
export function parsePanes(stdout: string): PaneEntry[] {
  const panes: PaneEntry[] = [];
  for (const line of lines(stdout)) {
    const parts = splitN(line, "\t", 6);
    if (parts.length < 6) continue;
    const [agent, session, win, pane, pid, paneId] = parts as [
      string,
      string,
      string,
      string,
      string,
      string,
    ];
    panes.push({
      agent,
      session,
      windowIndex: parseU32(win),
      paneIndex: parseU32(pane),
      panePid: parseU32(pid),
      paneId,
    });
  }
  return panes;
}

/** Parse `ps -A -o pid=,ppid=,args=` into parent-PID → child argv[0] basenames. */
export function parseProcessTable(stdout: string): Map<number, string[]> {
  const table = new Map<number, string[]>();
  for (const line of lines(stdout)) {
    const tokens = line.split(/\s+/).filter((t) => t !== "");
    const pid = tokens[0] !== undefined ? parseNumber(tokens[0]) : undefined;
    const ppid = tokens[1] !== undefined ? parseNumber(tokens[1]) : undefined;
    const argv0 = tokens[2] ?? "";
    const basename = argv0.slice(argv0.lastIndexOf("/") + 1);
    if (pid !== undefined && ppid !== undefined) {
      const children = table.get(ppid);
      if (children) children.push(basename);
      else table.set(ppid, [basename]);
    }
  }
  return table;
}

function parseNumber(s: string): number | undefined {
  return /^\+?\d+$/.test(s) && Number(s) <= 0xffff_ffff ? Number(s) : undefined;
}

/** `Tmux` port implementation shelling out to `tmux` and `ps`. */
export class ExecTmux implements Tmux {
  listPanes(server?: string): PaneEntry[] {
    const args = server !== undefined ? ["-S", server] : [];
    args.push("list-panes", "-a", "-F", FORMAT);
    const result = spawnSync("tmux", args, {
      encoding: "utf8",
      maxBuffer: MAX_BUFFER,
    });
    if (result.error) {
      const code = (result.error as NodeJS.ErrnoException).code;
      if (code === "ENOENT") {
        throw new Error("tmux not found — is tmux installed and on PATH?");
      }
      throw new Error(`failed to run tmux: ${result.error.message}`);
    }
    // Non-zero exit means no server running → empty list.
    if (result.status !== 0) return [];
    return parsePanes(result.stdout);
  }

  processTable(): Map<number, string[]> {
    const result = spawnSync("ps", ["-A", "-o", "pid=,ppid=,args="], {
      encoding: "utf8",
      maxBuffer: MAX_BUFFER,
    });
    if (result.error || result.status !== 0) return new Map();
    return parseProcessTable(result.stdout);
  }
}
