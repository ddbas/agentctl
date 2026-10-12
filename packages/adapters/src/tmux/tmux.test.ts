import { afterEach, describe, expect, it, vi } from "vitest";
import { makeTmp } from "../test-support.ts";
import { ExecTmux, parsePanes, parseProcessTable } from "./index.ts";

describe("parsePanes", () => {
  it("parses tab-delimited rows and tolerates bad numbers", () => {
    const out =
      "claude\tmy sess\t1\t2\t123\t%5\nnode\ts\tx\ty\tz\t%6\nshort\tline\n";
    expect(parsePanes(out)).toEqual([
      {
        agent: "claude",
        session: "my sess",
        windowIndex: 1,
        paneIndex: 2,
        panePid: 123,
        paneId: "%5",
      },
      {
        agent: "node",
        session: "s",
        windowIndex: 0,
        paneIndex: 0,
        panePid: 0,
        paneId: "%6",
      },
    ]);
  });

  it("keeps extra tabs in the last field and handles CRLF", () => {
    expect(parsePanes("pi\ts\t0\t0\t1\t%1\textra\r\n")[0]?.paneId).toBe(
      "%1\textra",
    );
  });
});

describe("parseProcessTable", () => {
  it("groups child argv[0] basenames by parent pid", () => {
    const table = parseProcessTable(
      "  100     1 /usr/bin/zsh -l\n  200   100 /opt/bin/pi --flag\n  201   100 node app.js\nbad line\n",
    );
    expect(table.get(1)).toEqual(["zsh"]);
    expect(table.get(100)).toEqual(["pi", "node"]);
  });
});

describe("ExecTmux", () => {
  afterEach(() => vi.unstubAllEnvs());

  it("reports 'tmux not found' when the binary is absent", () => {
    vi.stubEnv("PATH", makeTmp());
    expect(() => new ExecTmux().listPanes()).toThrow("tmux not found");
  });

  it("returns an empty list when no server is running at the socket", () => {
    expect(new ExecTmux().listPanes(`${makeTmp()}/missing.sock`)).toEqual([]);
  });
});
