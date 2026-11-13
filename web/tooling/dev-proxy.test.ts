import { describe, expect, it } from "vitest";

import { readPorts } from "./ports.ts";
import { parseServerPaths, serverPathPattern, serverPaths } from "./server-paths.ts";
import config from "./vite.config.ts";

const proxy: Readonly<Record<string, unknown>> = config.server?.proxy ?? {};
const passedOn = new RegExp(serverPathPattern(), "u");

describe("dev_proxy the dev server passes the server's addresses on", () => {
  it("has one rule, for the addresses the server answers itself", () => {
    expect(Object.keys(proxy)).toEqual([serverPathPattern()]);
    expect(serverPaths()).toContain("api");
  });

  it("reaches the server of this folder and keeps the browser's host", () => {
    const target = `http://127.0.0.1:${String(readPorts().server)}`;
    expect(Object.values(proxy)).toEqual([{ target, changeOrigin: false }]);
  });

  it("passes on whole first parts only, and keeps the files and pages of the app", () => {
    const server = [
      "/api/v1/auth/me",
      "/api?x=1",
      "/health",
      "/health/live",
      "/mcp",
      "/oauth/token",
      "/.well-known/oauth-authorization-server",
    ];
    const app = ["/healthy", "/apiary", "/assets/app.js", "/p/BLOG/issues", "/x.well-known"];
    expect(server.filter((address) => !passedOn.test(address))).toEqual([]);
    expect(app.filter((address) => passedOn.test(address))).toEqual([]);
    expect(passedOn.test("/")).toBe(false);
  });

  it("reads the list as rustfmt writes it once it grows: one name a line, a trailing comma, comments", () => {
    const router = [
      "/// The addresses.",
      "const NO_PAGE_FALLBACK: [&str; 3] = [",
      '    "assets",',
      "    // The probes of the load balancer.",
      '    "health", /* for now */',
      '    ".well-known",',
      "];",
    ].join("\n");
    expect(parseServerPaths(router)).toEqual(["assets", "health", ".well-known"]);
  });

  it("refuses a list it cannot read, rather than passing the start page on", () => {
    expect(() => parseServerPaths('const OTHER: [&str; 1] = ["api"];')).toThrow(
      /NO_PAGE_FALLBACK/u,
    );
    expect(() => parseServerPaths('const NO_PAGE_FALLBACK: [&str; 1] = ["a b"];')).toThrow(/a b/u);
    expect(() => parseServerPaths("const NO_PAGE_FALLBACK: [&str; 1] = [api];")).toThrow(/api/u);
  });
});
