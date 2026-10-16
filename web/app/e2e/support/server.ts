import { execFile, spawn } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { setTimeout as sleep } from "node:timers/promises";

import { repoRoot } from "../../../tooling/ports.ts";
import { askLive, waitUntilReady } from "./ready.ts";
import { copySampleInto } from "./sample.ts";

/** A Bugaboo server started for one group of tests, with a data folder of its own. */
export interface Instance {
  /** The address people type: HTTPS on this computer, with the certificate the server made. */
  readonly url: string;
  /** The setup code of the first start, from the file the server saves it in. */
  readonly setupCode: () => string;
  /**
   * What `bugaboo security-log verify` says about the data folder of this server, which it checks
   * from the outside, like an administrator would. Fails when the program says the log is broken.
   */
  readonly verifySecurityLog: () => Promise<string>;
  /** Stops the server and deletes its data (unless the data folder was given to `startServer`). */
  readonly stop: () => Promise<void>;
}

/** Start-up settings for one server, by their names in the environment (`BUGABOO_<NAME>`). */
export type Settings = Readonly<Record<string, string>>;

/** The pause between two tries to delete a data folder that Windows still keeps busy. */
const REMOVE_PAUSE_MS = 100;
const SETUP_CODE_FILE = "setup-code.txt";

/** Limits that never get in the way of a test, unless a test sets its own. */
const generousLimits: Settings = {
  BUGABOO_RATE_LIMIT_PER_MINUTE: "100000",
  BUGABOO_SIGN_IN_ATTEMPTS_PER_MINUTE: "100000",
  BUGABOO_SIGN_IN_FAILURES_PER_MINUTE: "100000",
};

/** The program that `cargo build` made (`BB_SERVER_BIN` names another one). */
function program(): string {
  const named = process.env["BB_SERVER_BIN"];
  if (named !== undefined) {
    return named;
  }
  const folder = process.env["CARGO_TARGET_DIR"] ?? resolve(repoRoot, "target");
  return resolve(folder, "debug", process.platform === "win32" ? "bugaboo.exe" : "bugaboo");
}

/** The browser app that `npm run build` made. */
function appFolder(): string {
  return resolve(repoRoot, "web/app/dist");
}

/** A started program: whether it has ended, what it said, and a way to end it. */
interface Launched {
  readonly exited: () => boolean;
  readonly output: () => string;
  readonly end: () => Promise<void>;
}

/** Starts the program with its output collected. */
function launch(executable: string, args: readonly string[], env: Settings): Launched {
  const child = spawn(executable, [...args], {
    env: { ...process.env, ...env },
    stdio: ["ignore", "pipe", "pipe"],
  });
  let said = "";
  const collect = (chunk: unknown): void => {
    said += String(chunk);
  };
  child.stdout.on("data", collect);
  child.stderr.on("data", collect);
  const ended = new Promise<void>((done) => {
    child.once("exit", () => {
      done();
    });
  });
  return {
    exited: () => child.exitCode !== null,
    output: () => said,
    end: async () => {
      child.kill();
      await Promise.race([ended, sleep(5000).then(() => child.kill("SIGKILL"))]);
      await ended;
    },
  };
}

/** Deletes a folder; Windows may keep a file busy for a moment after a program ended. */
async function remove(folder: string, triesLeft = 20): Promise<void> {
  try {
    rmSync(folder, { recursive: true, force: true });
  } catch (error) {
    if (triesLeft === 0) {
      throw error;
    }
    await sleep(REMOVE_PAUSE_MS);
    await remove(folder, triesLeft - 1);
  }
}

/** What `bugaboo security-log verify` prints for a data folder; it fails when the log is broken. */
function verifySecurityLog(executable: string, dataDir: string): Promise<string> {
  return new Promise((done, failed) => {
    execFile(
      executable,
      ["security-log", "verify", "--data-dir", dataDir],
      (error: unknown, stdout: string, stderr: string) => {
        if (error === null) {
          done(stdout);
        } else {
          failed(new Error(`${stdout}${stderr}`));
        }
      },
    );
  });
}

/**
 * Starts the real program on `port` with an empty data folder, or with a fresh copy of the sample
 * organization (`sample`, made by `cargo xtask dataset sample`), and waits until it answers. The
 * program is the one `cargo build` made and the app is the one `npm run build` made: the tests
 * drive what people get (ADR-0023). `label` names the data folder, to tell leftovers apart. A
 * `dataDir` that is given is used as it is and left in place when the server stops: the load run
 * reads its database afterwards (`cargo xtask load`).
 */
export async function startServer(
  port: number,
  label: string,
  settings: Settings = {},
  sample = false,
  dataDir?: string,
): Promise<Instance> {
  const executable = program();
  if (!existsSync(executable) || !existsSync(join(appFolder(), "index.html"))) {
    throw new Error(
      `Build first: the program ${executable} and the app ${appFolder()} must exist ` +
        "(cargo build -p bb-server, npm run build, or cargo xtask build).",
    );
  }
  const given = dataDir !== undefined;
  const folder = dataDir ?? mkdtempSync(join(tmpdir(), `bugaboo-e2e-${label}-`));
  if (sample) {
    copySampleInto(folder);
  }
  const server = launch(
    executable,
    ["serve", "--port", String(port), "--data-dir", folder, "--web", appFolder()],
    { ...generousLimits, ...settings },
  );
  const origin = `https://localhost:${String(port)}`;
  const stop = async (): Promise<void> => {
    await server.end();
    if (!given) {
      await remove(folder);
    }
  };
  await waitUntilReady(askLive(port), server).catch(async (error: unknown) => {
    await stop();
    throw error;
  });
  return {
    url: origin,
    setupCode: () => readFileSync(join(folder, SETUP_CODE_FILE), "utf8").trim(),
    verifySecurityLog: () => verifySecurityLog(executable, folder),
    stop,
  };
}
