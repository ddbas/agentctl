import { formatMicros } from "@agentctl/core";

/** Current instant as a chrono-style RFC 3339 UTC string (microsecond precision). */
export function nowTimestamp(): string {
  const micros = Math.floor(
    (performance.timeOrigin + performance.now()) * 1000,
  );
  return formatMicros(micros);
}
