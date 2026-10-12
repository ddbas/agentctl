import { describe, expect, it } from "vitest";
import { useTmuxEnv } from "./support/tmux-env.ts";

describe("agentctl help", () => {
  const env = useTmuxEnv();

  it("help subcommand exits zero", () => {
    expect(env().agentctl(["help"]).status).toBe(0);
  });

  it("help subcommand prints program name", () => {
    expect(env().agentctl(["help"]).stdout).toContain("agentctl");
  });

  it("help subcommand prints about text", () => {
    expect(env().agentctl(["help"]).stdout).toContain("agent multiplexer");
  });

  it("--help exits zero", () => {
    expect(env().agentctl(["--help"]).status).toBe(0);
  });

  it("--help prints usage", () => {
    expect(env().agentctl(["--help"]).stdout.toLowerCase()).toContain("usage");
  });

  it("no args exits zero", () => {
    expect(env().agentctl([]).status).toBe(0);
  });
});
