import type { Harness } from "@agentctl/core";
import { shared } from "../support/world.ts";

function fakeHarness(name: string) {
  return class implements Harness {
    readonly name = name;
    install(): string {
      shared().recorded.effects.push(`harness install ${name}`);
      return `Installed agentctl for ${name}`;
    }
    uninstall(): string {
      shared().recorded.effects.push(`harness uninstall ${name}`);
      return `Uninstalled agentctl for ${name}`;
    }
  };
}

/** Replacement module for `@agentctl/harness-pi`. */
export const harnessPiMock = { PiHarness: fakeHarness("pi") };
/** Replacement module for `@agentctl/harness-claude`. */
export const harnessClaudeMock = { ClaudeHarness: fakeHarness("claude") };
