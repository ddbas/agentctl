/// <reference path="../assets.d.ts" />
import {
  existsSync,
  mkdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { homedir } from "node:os";
import { dirname, join } from "node:path";
import type { Harness } from "@agentctl/core";
import { tildeCollapse } from "../paths.ts";
import PI_EXTENSION_CONTENT from "./assets/agentctl.ts?raw";

export { PI_EXTENSION_CONTENT };

type FileStatus = "new" | "unchanged" | "updated";

/** Default pi extension path: `~/.pi/agent/extensions/agentctl.ts`. */
export function defaultPiExtensionPath(home: string = homedir()): string {
  if (!home) throw new Error("could not determine home directory");
  return join(home, ".pi", "agent", "extensions", "agentctl.ts");
}

function piInstallSummary(dest: string, status: FileStatus): string {
  const pathDisplay = tildeCollapse(dest);
  const lines = ["Installed agentctl extension for pi:"];
  if (status === "unchanged") {
    lines.push(`  = ${pathDisplay} (already up to date)`);
  } else {
    lines.push(`  ${status === "new" ? "+" : "~"} ${pathDisplay}`);
    lines.push(
      "    session_start        → agentctl agent event SESSION_STARTED --harness pi",
      "    before_agent_start   → agentctl agent event RUN_STARTED --harness pi",
      "    agent_settled        → agentctl agent event RUN_FINISHED --harness pi",
      "    tool_call            → agentctl agent event TOOL_CALL_START --harness pi",
      "    tool_result          → agentctl agent event TOOL_CALL_RESULT --harness pi",
      "    session_shutdown     → agentctl agent clear",
    );
  }
  return lines.join("\n");
}

/** Install the extension at `dest`; idempotent (no rewrite when unchanged). */
export function installPiTo(dest: string): string {
  mkdirSync(dirname(dest), { recursive: true });
  let status: FileStatus = "new";
  if (existsSync(dest)) {
    status =
      readFileSync(dest, "utf8") === PI_EXTENSION_CONTENT
        ? "unchanged"
        : "updated";
  }
  if (status !== "unchanged") writeFileSync(dest, PI_EXTENSION_CONTENT);
  return piInstallSummary(dest, status);
}

/** Delete the extension at `dest` if present; never removes its directory. */
export function uninstallPiTo(dest: string): string {
  const removed = existsSync(dest);
  if (removed) rmSync(dest);
  const pathDisplay = tildeCollapse(dest);
  return [
    "Uninstalled agentctl extension for pi:",
    removed ? `  - ${pathDisplay}` : `  = ${pathDisplay} (nothing to remove)`,
  ].join("\n");
}

export class PiHarness implements Harness {
  readonly name = "pi";

  install(): string {
    return installPiTo(defaultPiExtensionPath());
  }

  uninstall(): string {
    return uninstallPiTo(defaultPiExtensionPath());
  }
}
