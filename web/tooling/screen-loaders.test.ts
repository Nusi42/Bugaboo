import { readdirSync, readFileSync } from "node:fs";
import { resolve } from "node:path";

import { describe, expect, it } from "vitest";

import { repoRoot } from "./ports.ts";

const routes = resolve(repoRoot, "web/app/src/routes");

/** The text of all the route files together. */
function routeText(): string {
  return readdirSync(routes)
    .filter((name) => name.endsWith(".tsx"))
    .map((name) => readFileSync(resolve(routes, name), "utf8"))
    .join("\n");
}

/** The screens of the routes that the router does not load before it draws the page. */
function unloadedScreens(text: string): string[] {
  return text
    .split("\n")
    .flatMap((line) => /^(?:export )?const (\w+) = lazyRouteComponent\(/u.exec(line)?.[1] ?? [])
    .filter((screen) => !text.includes(`loader: loadsScreen(${screen})`))
    .filter((screen) => !text.includes(`component: ${screen},`));
}

describe("the lazy screens of the routes", () => {
  it("are found", () => {
    expect(routeText()).toContain("lazyRouteComponent(");
  });

  // A screen that is the route's own `component` is loaded by the router before the page. A
  // screen inside another component (`OnlyWith`, a wrapper that reads the address) is not, so the
  // route needs `loader: loadsScreen(Screen)`; else the page is drawn empty first (`preload.ts`).
  it("each have the router load them before the page is drawn", () => {
    expect(unloadedScreens(routeText())).toEqual([]);
  });

  it("find a screen that has no loader", () => {
    const text =
      "const A = lazyRouteComponent(a)\nconst B = lazyRouteComponent(b)\nloader: loadsScreen(A)";
    expect(unloadedScreens(text)).toEqual(["B"]);
  });
});
