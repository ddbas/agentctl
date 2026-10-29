import type {
  AgentInfo,
  AgentRepository,
  AgentState,
  EventRecord,
} from "@agentctl/core";
import { FIXED_NOW, world } from "../support/world.ts";

class FakeRepository implements AgentRepository {
  private readonly infos = new Map<string, AgentInfo>();

  constructor() {
    for (const info of world().states) {
      this.infos.set(info.pane_id, { ...info });
    }
  }

  readAgentInfo(paneId: string): AgentInfo | undefined {
    return this.infos.get(paneId);
  }

  writeStateWithTools(
    paneId: string,
    harness: string,
    status: AgentState,
    activeTools: number,
  ): void {
    this.infos.set(paneId, {
      pane_id: paneId,
      harness,
      status,
      updated_at: FIXED_NOW,
      active_tools: activeTools,
    });
  }

  deleteState(paneId: string): void {
    this.infos.delete(paneId);
  }

  listAgentInfos(): AgentInfo[] {
    return [...this.infos.values()];
  }

  appendEvent(paneId: string, record: EventRecord): void {}
}

/** Replacement module for `@agentctl/state-fs`. */
export const stateFsMock = {
  FileAgentRepository: { xdg: () => new FakeRepository() },
};
