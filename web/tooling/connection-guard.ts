import { execFile } from "node:child_process";
import { mkdirSync, writeFileSync } from "node:fs";
import { relative, resolve } from "node:path";

import type {
  FullConfig,
  FullResult,
  Reporter,
  TestCase,
  TestResult,
} from "@playwright/test/reporter";

import {
  ConnectionGuard,
  countConnections,
  PAUSE_BETWEEN_LOOKS_MS,
  type Sample,
} from "./connections.ts";
import { readPorts } from "./ports.ts";

/**
 * The commands that list the connections, as lines of `local:port remote:port state`; the first
 * that works on this computer is used. Windows is asked through PowerShell (`pwsh` when there is no
 * `powershell`): `netstat` there prints the states in the language of the system. Only the states
 * the guard reads are listed.
 */
function listers(): readonly (readonly [string, readonly string[]])[] {
  const script =
    "Get-NetTCPConnection -State Established, TimeWait, CloseWait | " +
    "ForEach-Object { '{0}:{1} {2}:{3} {4}' -f $_.LocalAddress, $_.LocalPort, $_.RemoteAddress, $_.RemotePort, $_.State }";
  const flags = ["-NoProfile", "-NonInteractive", "-Command", script];
  return process.platform === "win32"
    ? [
        ["powershell", flags],
        ["pwsh", flags],
      ]
    : [
        ["ss", ["-tan"]],
        ["netstat", ["-ant"]],
      ];
}

/** What the first command that works prints (nothing is a list of no connection); `null` when none works. */
function listing(
  commands: readonly (readonly [string, readonly string[]])[],
): Promise<string | null> {
  const [first, ...rest] = commands;
  if (first === undefined) {
    return Promise.resolve(null);
  }
  return new Promise((done) => {
    execFile(
      first[0],
      [...first[1]],
      { maxBuffer: 64 * 1024 * 1024, env: { ...process.env, LC_ALL: "C" } },
      (error: unknown, stdout: string) => {
        if (error === null) {
          done(stdout);
        } else {
          void listing(rest).then(done);
        }
      },
    );
  });
}

/**
 * Counts the connections of the servers of the tests all through the run, and fails the run at the
 * end when more were left over at once than `MOST_LEFTOVER_CONNECTIONS` (TEST-CONNECTIONS). A look
 * takes seconds on Windows, so the next one is planned only when the one before has ended.
 */
export default class ConnectionGuardReporter implements Reporter {
  private readonly ports = readPorts();
  private readonly guard = new ConnectionGuard();
  /** The spec each worker ran last, by the port of its server. */
  private readonly specs: Record<number, string> = {};
  private readonly series = ["seconds,established,time_wait,close_wait"];
  private started = Date.now();
  private specFolder = "";
  private resultFolder = "";
  private stopped = false;
  private timer: NodeJS.Timeout | undefined;
  private inFlight: Promise<void> = Promise.resolve();

  public onBegin(config: FullConfig): void {
    // Every project has the same folders: the config says them once.
    const [project] = config.projects;
    if (project === undefined) {
      throw new Error("The config has no project.");
    }
    this.specFolder = project.testDir;
    this.resultFolder = project.outputDir;
    this.started = Date.now();
    this.plan();
  }

  public onTestBegin(test: TestCase, result: TestResult): void {
    const file = relative(this.specFolder, test.location.file).replaceAll("\\", "/");
    const project = test.parent.project()?.name ?? "";
    this.specs[this.ports.e2eWorkersFrom + result.parallelIndex] = `${file} [${project}]`;
  }

  public async onEnd(): Promise<{ status: FullResult["status"] } | undefined> {
    this.stopped = true;
    clearTimeout(this.timer);
    // Only the look that is under way is waited for; a run too short for any look makes one.
    await this.inFlight;
    if (this.series.length === 1) {
      await this.look();
    }
    const summary = this.guard.summary();
    const verdict = this.guard.verdict();
    mkdirSync(this.resultFolder, { recursive: true });
    writeFileSync(resolve(this.resultFolder, "connections.csv"), `${this.series.join("\n")}\n`);
    writeFileSync(resolve(this.resultFolder, "connections.txt"), `${summary}\n${verdict ?? ""}\n`);
    process.stdout.write(`${summary}\n`);
    if (verdict === undefined) {
      return undefined;
    }
    console.error(verdict);
    return { status: "failed" };
  }

  public printsToStdio(): boolean {
    return false;
  }

  /** Plans the next look after a pause; the timer does not keep the process alive. */
  private plan(): void {
    if (this.stopped) {
      return;
    }
    this.timer = setTimeout(() => {
      this.inFlight = this.look().then(() => {
        this.plan();
      });
    }, PAUSE_BETWEEN_LOOKS_MS);
    this.timer.unref();
  }

  private async look(): Promise<void> {
    const text = await listing(listers());
    if (text === null) {
      this.guard.recordFailure();
      return;
    }
    const sample: Sample = countConnections(
      text,
      this.ports.e2eWorkersFrom,
      this.ports.e2eWorkersTo,
    );
    this.guard.record(sample, this.specs);
    const seconds = Math.round((Date.now() - this.started) / 1000);
    const { established, timeWait, closeWait } = sample.total;
    this.series.push(
      `${String(seconds)},${String(established)},${String(timeWait)},${String(closeWait)}`,
    );
  }
}
