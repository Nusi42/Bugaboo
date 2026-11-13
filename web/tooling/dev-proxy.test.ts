import { describe, expect, it } from "vitest";

import { readPorts } from "./ports.ts";
import { serverPathPattern, serverPaths } from "./server-paths.ts";
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
  });
});
