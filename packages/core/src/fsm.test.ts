import { describe, expect, it } from "vitest";
import { type AgentState, compareAgentState, type FsmEvent } from "./domain.ts";
import { applyEvent } from "./fsm.ts";

const ALL: AgentState[] = ["idle", "working", "blocked", "done", "unknown"];
const elicit = (kind: "TOOL_CALL_START" | "TOOL_CALL_RESULT"): FsmEvent => ({
  kind,
  type: "elicitation",
});

describe("status FSM", () => {
  it("SESSION_STARTED resets every state to idle", () => {
    for (const s of ALL)
      expect(applyEvent(s, { kind: "SESSION_STARTED" })).toBe("idle");
  });

  it("RUN_FINISHED reaches done from every state", () => {
    for (const s of ALL)
      expect(applyEvent(s, { kind: "RUN_FINISHED" })).toBe("done");
  });

  it("RUN_STARTED moves idle and done to working", () => {
    expect(applyEvent("idle", { kind: "RUN_STARTED" })).toBe("working");
    expect(applyEvent("done", { kind: "RUN_STARTED" })).toBe("working");
  });

  it("TOOL_CALL_START moves done to working (or blocked for elicitation)", () => {
    expect(applyEvent("done", { kind: "TOOL_CALL_START" })).toBe("working");
    expect(
      applyEvent("done", { kind: "TOOL_CALL_START", type: "custom" }),
    ).toBe("working");
    expect(applyEvent("done", elicit("TOOL_CALL_START"))).toBe("blocked");
  });

  it("TOOL_CALL_RESULT does not move done", () => {
    for (const type of [undefined, "custom", "elicitation"] as const) {
      expect(applyEvent("done", { kind: "TOOL_CALL_RESULT", type })).toBe(
        "done",
      );
    }
  });

  it("elicitation blocks working and result unblocks", () => {
    expect(applyEvent("working", elicit("TOOL_CALL_START"))).toBe("blocked");
    expect(applyEvent("blocked", elicit("TOOL_CALL_RESULT"))).toBe("working");
  });

  it("untyped and custom tool call events are no-ops", () => {
    for (const kind of ["TOOL_CALL_START", "TOOL_CALL_RESULT"] as const) {
      expect(applyEvent("working", { kind })).toBe("working");
      expect(applyEvent("working", { kind, type: "custom" })).toBe("working");
    }
  });

  it("unmatched events are no-ops", () => {
    expect(applyEvent("idle", { kind: "TOOL_CALL_START" })).toBe("idle");
    expect(applyEvent("idle", elicit("TOOL_CALL_RESULT"))).toBe("idle");
    expect(applyEvent("blocked", { kind: "RUN_STARTED" })).toBe("blocked");
    expect(applyEvent("unknown", { kind: "RUN_STARTED" })).toBe("unknown");
  });
});

describe("AgentState ordering", () => {
  it("sorts blocked < done < working < idle < unknown", () => {
    const states: AgentState[] = [
      "unknown",
      "idle",
      "blocked",
      "working",
      "done",
    ];
    states.sort(compareAgentState);
    expect(states).toEqual(["blocked", "done", "working", "idle", "unknown"]);
  });
});
