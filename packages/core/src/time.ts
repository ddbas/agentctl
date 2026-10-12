/** RFC 3339 helpers matching chrono's output (UTC, millisecond precision). */

/**
 * chrono `Debug`/serde form for a UTC instant given in epoch microseconds:
 * `2026-01-01T00:00:00Z`, `...00.123Z` or `...00.123456Z` (fractional digits
 * only as needed, in groups of three).
 */
export function formatMicros(epochMicros: number): string {
  const ms = Math.floor(epochMicros / 1000);
  const frac = (ms % 1000) * 1000 + (epochMicros - ms * 1000);
  const base = new Date(ms).toISOString().slice(0, 19);
  if (frac === 0) return `${base}Z`;
  const digits = String(frac).padStart(6, "0");
  return `${base}.${frac % 1000 === 0 ? digits.slice(0, 3) : digits}Z`;
}

/** chrono form for a `Date` (millisecond precision). */
export function formatTimestamp(date: Date): string {
  return formatMicros(date.getTime() * 1000);
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
