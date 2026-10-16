/** Replace the `home` directory prefix with `~` for human-readable output. */
export function tildeCollapse(path: string, home: string): string {
  if (home && path.startsWith(`${home}/`)) {
    return `~/${path.slice(home.length + 1)}`;
  }
  if (home && path === home) return "~/";
  return path;
}
