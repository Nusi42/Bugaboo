import { spawnSync } from "node:child_process";
import { resolve } from "node:path";

import { repoRoot } from "./ports.ts";

// `npm run readme-media` makes the pictures and the recordings of the front page again, into
// `docs/media`, on Quillmoor Cycle Works, a made-up bicycle workshop that the script sets up on an
// empty server (`web/app/e2e/support/readme-organization-data.ts`). Run it after a change of the
// look of a screen that README.md shows. It needs the browsers of `cargo xtask setup`, the program
// (`cargo build -p bb-server`) and the browser app (`npm run build`). The script of the pictures
// and recordings is `web/app/e2e/readme-media.spec.ts`; `BB_README_MEDIA_DIR` names another folder.

/** The folder the pictures and recordings are written to, relative to the repository. */
const MEDIA = process.env["BB_README_MEDIA_DIR"] ?? "docs/media";

/** Runs a program in the repository and stops this script when it fails. */
function run(program: string, args: readonly string[], env: Readonly<Record<string, string>> = {}) {
  const result = spawnSync(program, args, {
    cwd: repoRoot,
    stdio: "inherit",
    shell: process.platform === "win32",
    env: { ...process.env, ...env },
  });
  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}

run(
  "npx",
  [
    "playwright",
    "test",
    "--config",
    "web/tooling/playwright.config.ts",
    "readme-media.spec",
    "--project=desktop",
    "--workers=1",
  ],
  { BB_README_MEDIA_DIR: resolve(repoRoot, MEDIA) },
);
