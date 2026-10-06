import { execFile, spawn } from "node:child_process";
import { randomBytes } from "node:crypto";
import { existsSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { DatabaseSync } from "node:sqlite";
import { connect } from "node:net";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { setTimeout as sleep } from "node:timers/promises";

import { repoRoot } from "../../../tooling/ports.ts";
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
  /**
   * Switches a yes-or-no setting of the organization the way the settings screen will: there is no
   * screen for it yet, so the value is written into the database of the running server. It counts
   * from the next request, as a change made on a screen would.
   */
  readonly setOrganizationSwitch: (key: string, on: boolean) => void;
  /**
   * Gives a person the starting role (`contributor`, `guest`, …) the way *Who has which role* will:
   * for the whole organization, or on a project (given by its short name) and its sub-projects.
   * That screen belongs to another package, so the row is written into the database of the running
   * server (M2-F92). It counts from the next request.
   */
  readonly giveRole: (email: string, role: string, project: string | null) => void;
  /** Stops the server and deletes its data. */
  readonly stop: () => Promise<void>;
}

/** Start-up settings for one server, by their names in the environment (`BUGABOO_<NAME>`). */
export type Settings = Readonly<Record<string, string>>;

/** How long a server gets to start before the test gives up. */
const START_LIMIT_MS = 30_000;
const POLL_MS = 100;
const SETUP_CODE_FILE = "setup-code.txt";
/** The built-in database in the data folder (`bb_store::DATABASE_FILE`). */
const DATABASE_FILE = "bugaboo.db";
const BUSY_MS = 5000;
const MICROSECONDS_PER_MS = 1000;

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

/**
 * Whether the server accepts connections. It opens its port only after its database and keys are
 * ready (it starts to listen last), so an open port means a server that can answer.
 */
function answers(port: number): Promise<boolean> {
  return new Promise((done) => {
    const probe = connect({ host: "localhost", port });
    probe.once("connect", () => {
      probe.destroy();
      done(true);
    });
    probe.once("error", () => {
      done(false);
    });
  });
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

/** Waits until the server answers; gives up when it ends, or when it takes too long. */
async function waitUntilReady(port: number, server: Launched, started = Date.now()): Promise<void> {
  if (server.exited()) {
    throw new Error(`The server stopped while starting:\n${server.output()}`);
  }
  if (Date.now() - started > START_LIMIT_MS) {
    throw new Error(`The server did not answer on port ${String(port)}:\n${server.output()}`);
  }
  if (!(await answers(port))) {
    await sleep(POLL_MS);
    await waitUntilReady(port, server, started);
  }
}

/** Deletes a folder; Windows may keep a file busy for a moment after a program ended. */
async function remove(folder: string, triesLeft = 20): Promise<void> {
  try {
    rmSync(folder, { recursive: true, force: true });
  } catch (error) {
    if (triesLeft === 0) {
      throw error;
    }
    await sleep(POLL_MS);
    await remove(folder, triesLeft - 1);
  }
}

/** Writes the value of a setting of the whole organization into the database of a server. */
function putOrganizationSwitch(dataDir: string, key: string, on: boolean): void {
  const database = new DatabaseSync(join(dataDir, DATABASE_FILE));
  try {
    database.exec(`PRAGMA busy_timeout = ${String(BUSY_MS)}`);
    database
      .prepare(
        "INSERT INTO setting (key, level, scope, value, locked, updated_at) " +
          "VALUES (?, 'organization', ?, ?, 0, ?) " +
          "ON CONFLICT (key, level, scope) DO UPDATE SET value = excluded.value, " +
          "updated_at = excluded.updated_at",
      )
      .run(key, new Uint8Array(), JSON.stringify(on), Date.now() * MICROSECONDS_PER_MS);
  } finally {
    database.close();
  }
}

/** The ID in a column of a row that was read, as the bytes the database holds. */
function idOf(row: Readonly<Record<string, unknown>> | undefined, what: string): Uint8Array {
  const id = row?.["id"];
  if (!(id instanceof Uint8Array)) {
    throw new Error(`${what} is not in the database of the server.`);
  }
  return id;
}

/** A new ID of the time-ordered kind (UUID version 7), which the rows of *Who has which role* have. */
function orderedId(): Uint8Array {
  const time = BigInt(Date.now());
  const bytes = Uint8Array.from(randomBytes(16));
  bytes.set(
    Array.from({ length: 6 }, (_, at) => Number((time >> BigInt(8 * (5 - at))) & 0xffn)),
    0,
  );
  bytes.set([((bytes.at(6) ?? 0) & 0x0f) | 0x70], 6);
  bytes.set([((bytes.at(8) ?? 0) & 0x3f) | 0x80], 8);
  return bytes;
}

/** Writes a row of *Who has which role* into the database of a server. */
function putRole(dataDir: string, email: string, role: string, project: string | null): void {
  const database = new DatabaseSync(join(dataDir, DATABASE_FILE));
  try {
    database.exec(`PRAGMA busy_timeout = ${String(BUSY_MS)}`);
    const person = idOf(
      database.prepare("SELECT id FROM person WHERE email = ?").get(email),
      email,
    );
    const found = idOf(database.prepare("SELECT id FROM role WHERE starting = ?").get(role), role);
    const place =
      project === null
        ? null
        : idOf(
            database.prepare("SELECT id FROM project WHERE short_name = ?").get(project),
            project,
          );
    const row = orderedId();
    database
      .prepare(
        'INSERT INTO assignment (id, person, "group", role, project, applies_to, given_at) ' +
          "VALUES (?, ?, NULL, ?, ?, ?, ?)",
      )
      .run(
        row,
        person,
        found,
        place,
        project === null ? "organization" : "project-and-sub-projects",
        BigInt(Date.now()) * BigInt(MICROSECONDS_PER_MS),
      );
  } finally {
    database.close();
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
 * drive what people get (ADR-0023). `label` names the data folder, to tell leftovers apart.
 */
export async function startServer(
  port: number,
  label: string,
  settings: Settings = {},
  sample = false,
): Promise<Instance> {
  const executable = program();
  if (!existsSync(executable) || !existsSync(join(appFolder(), "index.html"))) {
    throw new Error(
      `Build first: the program ${executable} and the app ${appFolder()} must exist ` +
        "(cargo build -p bb-server, npm run build, or cargo xtask build).",
    );
  }
  const dataDir = mkdtempSync(join(tmpdir(), `bugaboo-e2e-${label}-`));
  if (sample) {
    copySampleInto(dataDir);
  }
  const server = launch(
    executable,
    ["serve", "--port", String(port), "--data-dir", dataDir, "--web", appFolder()],
    { ...generousLimits, ...settings },
  );
  const stop = async (): Promise<void> => {
    await server.end();
    await remove(dataDir);
  };
  await waitUntilReady(port, server).catch(async (error: unknown) => {
    await stop();
    throw error;
  });
  return {
    url: `https://localhost:${String(port)}`,
    setupCode: () => readFileSync(join(dataDir, SETUP_CODE_FILE), "utf8").trim(),
    verifySecurityLog: () => verifySecurityLog(executable, dataDir),
    setOrganizationSwitch: (key, on) => {
      putOrganizationSwitch(dataDir, key, on);
    },
    giveRole: (email, role, project) => {
      putRole(dataDir, email, role, project);
    },
    stop,
  };
}
