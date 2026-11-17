import { createHash } from "node:crypto";

import type { Plugin } from "vite";

import { firstLookPlaces, replayFirstLook } from "../packages/ui/src/first-look.ts";

/** How many characters of the hash of its text the name of the built script carries. */
const HASH_LENGTH = 8;

/**
 * The text of the script: `replayFirstLook` itself, called with the places it needs. The function
 * uses nothing from outside itself, so its text is the whole script.
 */
export function firstLookScript(): string {
  return `(${replayFirstLook.toString()})(${JSON.stringify(firstLookPlaces)});\n`;
}

/** Where the built page loads the script from: a name that changes with its text, like every built file. */
export function builtScriptName(script: string): string {
  const hash = createHash("sha256").update(script).digest("hex").slice(0, HASH_LENGTH);
  return `assets/first-look-${hash}.js`;
}

/**
 * Puts the person's look on the page frame before its first paint (UX §6.4): a script of its own at
 * the end of the head, after the style sheet, which runs before anything is drawn. The built page
 * loads it as a file of its own origin, not as a script in the page, so the content security policy
 * stays `script-src 'self'` (SEC-11). The server of the development, which sends no such policy,
 * puts its text in the page.
 */
export function firstLook(): Plugin[] {
  const script = firstLookScript();
  const built = builtScriptName(script);
  return [
    {
      name: "bugaboo-first-look-build",
      apply: "build",
      generateBundle() {
        this.emitFile({ type: "asset", fileName: built, source: script });
      },
      // After the style sheet of the app, which the build puts in the head before this runs: the sheet of an own theme
      // that the script links comes after it, so its colors win over those of the theme it extends.
      transformIndexHtml: {
        order: "post",
        handler: () => [{ tag: "script", attrs: { src: `/${built}` }, injectTo: "head" }],
      },
    },
    {
      name: "bugaboo-first-look-serve",
      apply: "serve",
      transformIndexHtml: {
        order: "post",
        handler: () => [{ tag: "script", children: script, injectTo: "head" }],
      },
    },
  ];
}
