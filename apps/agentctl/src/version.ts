declare const __AGENTCTL_VERSION__: string | undefined;

/** Package version, injected at build time by Rspack's DefinePlugin. */
export const VERSION: string =
  typeof __AGENTCTL_VERSION__ === "string" ? __AGENTCTL_VERSION__ : "0.0.0-dev";
