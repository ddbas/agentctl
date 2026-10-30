import type { Harness } from "@agentctl/core";

function fakeHarness(name: string) {
  return class implements Harness {
    readonly name = name;
    install(): string {
      return `Installed agentctl for ${name}`;
    }
    uninstall(): string {
      return `Uninstalled agentctl for ${name}`;
    }
  };
}

/** Replacement module for `@agentctl/harness-pi`. */
export const harnessPiMock = { PiHarness: fakeHarness("pi") };
/** Replacement module for `@agentctl/harness-claude`. */
export const harnessClaudeMock = { ClaudeHarness: fakeHarness("claude") };
