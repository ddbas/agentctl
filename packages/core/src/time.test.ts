import { describe, expect, it } from "vitest";
import {
  formatMicros,
  formatTimestamp,
  timestampMillis,
  toRfc3339,
} from "./time.ts";

describe("time", () => {
  it("formats like chrono with only the needed fractional digits", () => {
    const base = Date.parse("2026-01-01T00:00:00Z") * 1000;
    expect(formatMicros(base)).toBe("2026-01-01T00:00:00Z");
    expect(formatMicros(base + 123_000)).toBe("2026-01-01T00:00:00.123Z");
    expect(formatMicros(base + 123_456)).toBe("2026-01-01T00:00:00.123456Z");
    expect(formatMicros(base + 5)).toBe("2026-01-01T00:00:00.000005Z");
    expect(formatTimestamp(new Date("2026-01-01T00:00:00.500Z"))).toBe(
      "2026-01-01T00:00:00.500Z",
    );
  });

  it("converts to rfc3339 +00:00 and parses millis", () => {
    expect(toRfc3339("2026-01-01T00:00:00.123456Z")).toBe(
      "2026-01-01T00:00:00.123456+00:00",
    );
    expect(timestampMillis("2026-01-01T00:00:00Z")).toBe(
      Date.parse("2026-01-01T00:00:00Z"),
    );
    expect(timestampMillis("garbage")).toBe(Number.NEGATIVE_INFINITY);
  });
});
