import {
  appendFileSync,
  existsSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  renameSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";
import {
  AGENT_STATES,
  type AgentInfo,
  type AgentRepository,
  type AgentState,
  type EventRecord,
  formatTimestamp,
} from "@agentctl/core";
import { z } from "zod";
import { nowTimestamp } from "../clock.ts";

const RFC3339 =
  /^\d{4}-\d{2}-\d{2}[Tt]\d{2}:\d{2}:\d{2}(\.\d+)?([Zz]|[+-]\d{2}:\d{2})$/;

/** RFC 3339 timestamp, normalised to UTC (`Z`) like chrono's `DateTime<Utc>`. */
const timestampSchema = z
  .string()
  .refine((s) => RFC3339.test(s) && !Number.isNaN(Date.parse(s)))
  .transform((s) =>
    /[Zz]$/.test(s) ? s.toUpperCase() : formatTimestamp(new Date(s)),
  );

/** Compatibility contract for `state.json` files (including Rust-written ones). */
export const AgentInfoSchema = z.object({
  pane_id: z.string(),
  harness: z.string(),
  status: z.enum(AGENT_STATES),
  updated_at: timestampSchema,
  active_tools: z.number().int().min(0).max(0xffff_ffff).default(0),
});

function parseAgentInfo(bytes: string): AgentInfo | undefined {
  try {
    const parsed = AgentInfoSchema.safeParse(JSON.parse(bytes));
    return parsed.success ? parsed.data : undefined;
  } catch {
    return undefined;
  }
}

/** `$XDG_DATA_HOME` if set and non-empty, otherwise `~/.local/share`. */
export function xdgDataHome(
  env: NodeJS.ProcessEnv = process.env,
  home: string = homedir(),
): string {
  const val = env.XDG_DATA_HOME;
  if (val) return val;
  return join(env.HOME ?? home ?? ".", ".local", "share");
}

/**
 * The sole module that reads or writes agent state on disk:
 * `<base>/agentctl/agents/<pane-id>/{state.json,events.jsonl}`.
 */
export class FileAgentRepository implements AgentRepository {
  constructor(
    private readonly base: string,
    private readonly clock: () => string = nowTimestamp,
  ) {}

  static xdg(): FileAgentRepository {
    return new FileAgentRepository(xdgDataHome());
  }

  private agentDir(paneId: string): string {
    return join(this.base, "agentctl", "agents", paneId);
  }

  readAgentInfo(paneId: string): AgentInfo | undefined {
    try {
      return parseAgentInfo(
        readFileSync(join(this.agentDir(paneId), "state.json"), "utf8"),
      );
    } catch {
      return undefined;
    }
  }

  writeStateWithTools(
    paneId: string,
    harness: string,
    status: AgentState,
    activeTools: number,
  ): void {
    const dir = this.agentDir(paneId);
    mkdirSync(dir, { recursive: true });
    const info: AgentInfo = {
      pane_id: paneId,
      harness,
      status,
      updated_at: this.clock(),
      active_tools: activeTools,
    };
    const finalPath = join(dir, "state.json");
    const tmpPath = join(dir, "state.json.tmp");
    writeFileSync(tmpPath, JSON.stringify(info, null, 2));
    renameSync(tmpPath, finalPath);
  }

  deleteState(paneId: string): void {
    const dir = this.agentDir(paneId);
    if (existsSync(dir)) rmSync(dir, { recursive: true, force: true });
  }

  listAgentInfos(): AgentInfo[] {
    const agentsDir = join(this.base, "agentctl", "agents");
    let entries: string[];
    try {
      entries = readdirSync(agentsDir);
    } catch {
      return [];
    }
    const infos: AgentInfo[] = [];
    for (const entry of entries) {
      try {
        const info = parseAgentInfo(
          readFileSync(join(agentsDir, entry, "state.json"), "utf8"),
        );
        if (info) infos.push(info);
      } catch {
        // skip unreadable entries
      }
    }
    return infos;
  }

  appendEvent(paneId: string, record: EventRecord): void {
    const dir = this.agentDir(paneId);
    mkdirSync(dir, { recursive: true });
    appendFileSync(join(dir, "events.jsonl"), `${JSON.stringify(record)}\n`);
  }
}
