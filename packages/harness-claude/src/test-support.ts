import { mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

/** Rust-written fixtures live in `apps/agentctl/tests/fixtures/`. */
const FIXTURES = fileURLToPath(
  new URL("../../../apps/agentctl/tests/fixtures/", import.meta.url),
);

export const fixturePath = (...parts: string[]): string =>
  join(FIXTURES, ...parts);

export const readFixture = (...parts: string[]): string =>
  readFileSync(fixturePath(...parts), "utf8");

export const makeTmp = (): string =>
  mkdtempSync(join(tmpdir(), "agentctl-adapters-"));
