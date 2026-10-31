import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { join, resolve } from "node:path";

import layout from "../../tools/dev/sample.json" with { type: "json" };
import { repoRoot } from "./ports.ts";

// `npm run readme-media` makes the pictures and the recordings of the front page again, from the
// sample organization (made-up people only), into `docs/media`. Run it after a change of the look
// of a screen that README.md shows. It needs the browsers of `cargo xtask setup`, and it makes the
// sample first if there is none. The script of the recordings is
// `web/app/e2e/readme-media.spec.ts`.

/** The folder the pictures and recordings are written to, relative to the repository. */
const MEDIA = "docs/media";

/** The issues of the sample the recordings are made on: a small sample is enough and quick. */
const SAMPLE_ISSUES = "500";

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

if (!existsSync(join(repoRoot, layout.out, layout.signInFile))) {
  run("cargo", ["xtask", "dataset", "sample", "--issues", SAMPLE_ISSUES]);
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
