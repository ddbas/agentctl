/** RFC 3339 helpers matching chrono's output (UTC, millisecond precision). */

/** chrono `Debug`/serde form: `2026-01-01T00:00:00Z` or `...00.123Z`. */
export function formatTimestamp(date: Date): string {
  return date.toISOString().replace(".000Z", "Z");
}

/** chrono `to_rfc3339()` form: `2026-01-01T00:00:00+00:00`. */
export function toRfc3339(timestamp: string): string {
  return timestamp.endsWith("Z")
    ? `${timestamp.slice(0, -1)}+00:00`
    : timestamp;
}

/** Epoch millis for sorting; `-Infinity` when unparsable. */
export function timestampMillis(timestamp: string): number {
  const ms = Date.parse(timestamp);
  return Number.isNaN(ms) ? Number.NEGATIVE_INFINITY : ms;
}
