import { homedir } from "node:os";
import { sep } from "node:path";

/** Replace the home directory prefix with `~` for human-readable output. */
export function tildeCollapse(path: string, home: string = homedir()): string {
  if (home && path.startsWith(home + sep)) {
    return `~/${path.slice(home.length + 1)}`;
  }
  if (home && path === home) return "~/";
  return path;
}
