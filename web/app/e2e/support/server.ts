import { execFile, spawn } from "node:child_process";
import { existsSync, mkdtempSync, readdirSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, join, resolve } from "node:path";
import { setTimeout as sleep } from "node:timers/promises";

import { repoRoot } from "../../../tooling/ports.ts";
import { askLive, waitUntilReady } from "./ready.ts";
import { closePool } from "./session-transport.ts";
import { copySampleInto, type Sample } from "./sample.ts";

/** A Bugaboo server started for one group of tests, with a data folder of its own. */
export interface Instance {
  /** The address people type: HTTPS on this computer, with the certificate the server made. */
  readonly url: string;
  /** The folder the server keeps its data in (what a test that needs a row the API cannot make writes into). */
  readonly dataDir: string;
  /** The setup code of the first start, from the file the server saves it in. */
  readonly setupCode: () => string;
  /**
   * What `bugaboo security-log verify` says about the data folder of this server, which it checks
   * from the outside, like an administrator would. Fails when the program says the log is broken.
   */
  readonly verifySecurityLog: () => Promise<string>;
  /**
   * Stops the server, restores the backup of that name from the backups folder of its data folder
   * with `bugaboo restore`, as an administrator does (OPS-07), and starts the server again on the
   * same data and port. Resolves to what the command said; fails when it failed.
   */
  readonly restore: (backup: string) => Promise<string>;
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
  BUGABOO_SEARCHES_PER_MINUTE: "100000",
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

/** How much of what a started program said is kept, from its end: enough to tell why it stopped. */
const KEPT_OUTPUT_CHARS = 1_000_000;

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
    // Only the end is kept: a server of a long run says more than a string can hold.
    said = (said + String(chunk)).slice(-KEPT_OUTPUT_CHARS);
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

/** What a command of the program prints for a data folder; it fails when the command failed. */
function command(executable: string, args: readonly string[], env: Settings = {}): Promise<string> {
  return new Promise((done, failed) => {
    const options = { env: { ...process.env, ...env } };
    execFile(executable, [...args], options, (error: unknown, stdout: string, stderr: string) => {
      if (error === null) {
        done(stdout);
      } else {
        failed(new Error(`${stdout}${stderr}`));
      }
    });
  });
}

/** Where the backup of that name lies in a data folder: the folder of the backups is the program's to name. */
export function backupIn(dataDir: string, backup: string): string {
  const found = readdirSync(dataDir, { recursive: true, encoding: "utf8" }).find(
    (path) => basename(path) === backup,
  );
  if (found === undefined) {
    throw new Error(`The data folder holds no backup ${backup}.`);
  }
  return join(dataDir, found);
}

/** What `bugaboo security-log verify` prints for a data folder; it fails when the log is broken. */
function verifySecurityLog(executable: string, dataDir: string): Promise<string> {
  return command(executable, ["security-log", "verify", "--data-dir", dataDir]);
}

/** The program serving one data folder on one port, which a restore stops and starts again. */
interface Serving {
  /** Waits until the program answers. */
  readonly ready: () => Promise<void>;
  /** Ends the program. */
  readonly end: () => Promise<void>;
  /** Runs a command of the program on the data folder while the server is stopped, and starts it again. */
  readonly whileStopped: (args: readonly string[]) => Promise<string>;
}

/** Starts the program on the data folder and keeps the one that runs. */
function serving(executable: string, port: number, folder: string, env: Settings): Serving {
  const serve = (): Launched =>
    launch(
      executable,
      ["serve", "--port", String(port), "--data-dir", folder, "--web", appFolder()],
      env,
    );
  let server = serve();
  const ready = (): Promise<void> => waitUntilReady(askLive(port), server);
  return {
    ready,
    end: () => server.end(),
    whileStopped: async (args) => {
      await server.end();
      const said = await command(executable, args, env).finally(() => {
        server = serve();
      });
      await ready();
      return said;
    },
  };
}

/**
 * Starts the real program on `port` with an empty data folder, or with a fresh copy of the sample
 * organization (`sample`: the full one or the one at a fixed moment, see `Sample`), and waits until it answers. The
 * program is the one `cargo build` made and the app is the one `npm run build` made: the tests
 * drive what people get (ADR-0023). `label` names the data folder, to tell leftovers apart. A
 * `dataDir` that is given is used as it is and left in place when the server stops: the load run
 * reads its database afterwards (`cargo xtask load`).
 */
export async function startServer(
  port: number,
  label: string,
  settings: Settings = {},
  sample: Sample | false = false,
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
  if (sample !== false) {
    copySampleInto(folder, sample);
  }
  const server = serving(executable, port, folder, { ...generousLimits, ...settings });
  const origin = `https://localhost:${String(port)}`;
  const stop = async (): Promise<void> => {
    closePool(origin);
    await server.end();
    if (!given) {
      await remove(folder);
    }
  };
  await server.ready().catch(async (error: unknown) => {
    await stop();
    throw error;
  });
  return {
    url: origin,
    dataDir: folder,
    setupCode: () => readFileSync(join(folder, SETUP_CODE_FILE), "utf8").trim(),
    verifySecurityLog: () => verifySecurityLog(executable, folder),
    restore: (backup) => {
      closePool(origin);
      return server.whileStopped(["restore", backupIn(folder, backup), "--data-dir", folder]);
    },
    stop,
  };
}
