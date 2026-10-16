import { describe, expect, it } from "vitest";
import { tildeCollapse } from "./paths.ts";

describe("tildeCollapse", () => {
  it("collapses the home prefix", () => {
    expect(tildeCollapse("/home/u/a/b", "/home/u")).toBe("~/a/b");
  });
  it("collapses home itself", () => {
    expect(tildeCollapse("/home/u", "/home/u")).toBe("~/");
  });
  it("leaves other paths and sibling prefixes alone", () => {
    expect(tildeCollapse("/etc/x", "/home/u")).toBe("/etc/x");
    expect(tildeCollapse("/home/user/x", "/home/u")).toBe("/home/user/x");
  });
  it("ignores an empty home", () => {
    expect(tildeCollapse("/a", "")).toBe("/a");
  });
});
