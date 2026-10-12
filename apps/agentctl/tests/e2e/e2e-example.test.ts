import { expect, it } from "vitest";
import { startGenericContainer } from "./support/tmux-env.ts";

/**
 * Canary for the e2e infrastructure: start -> assert running -> stop.
 * If this passes, Docker is reachable and testcontainers works.
 */
it("container starts and stops", async () => {
  const container = await startGenericContainer();
  try {
    expect(container.getId()).not.toBe("");
  } finally {
    await container.stop({ remove: true });
  }
});
