import { FAKE_HOME, world } from "../support/world.ts";

const LOG_DIR = `${FAKE_HOME}/.local/state/agentctl/logs`;

/** Replacement module for `@agentctl/logging`. */
export const loggingMock = {
  initLogging: () => ({
    debug() {},
    info() {},
    warn() {},
    error() {},
  }),
  logDir: (): string | undefined => (world().noLogDir ? undefined : LOG_DIR),
  currentLogFile: (dir: string): string => `${dir}/agentctl.log.2026-01-01`,
};
