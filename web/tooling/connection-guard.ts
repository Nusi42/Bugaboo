import { execFile } from "node:child_process";
import { mkdirSync, writeFileSync } from "node:fs";
import { relative, resolve } from "node:path";

import type { FullResult, Reporter, TestCase, TestResult } from "@playwright/test/reporter";

import { ConnectionGuard, countConnections, SAMPLE_EVERY_MS, type Sample } from "./connections.ts";
import { readPorts, repoRoot } from "./ports.ts";

/**
 * The commands that list the connections of the ports `from` to `to`, as lines of
 * `local:port remote:port state`; the first that works on this computer is used. Windows is asked
 * through PowerShell: `netstat` there prints the states in the language of the system.
 */
function listers(from: number, to: number): readonly (readonly [string, readonly string[]])[] {
  const inRange = (port: string): string =>
    `($_.${port} -ge ${String(from)} -and $_.${port} -le ${String(to)})`;
  const script =
    `Get-NetTCPConnection | Where-Object { ${inRange("LocalPort")} -or ${inRange("RemotePort")} } | ` +
    "ForEach-Object { '{0}:{1} {2}:{3} {4}' -f $_.LocalAddress, $_.LocalPort, $_.RemoteAddress, $_.RemotePort, $_.State }";
  return process.platform === "win32"
    ? [["powershell", ["-NoProfile", "-NonInteractive", "-Command", script]]]
    : [
        ["ss", ["-tan"]],
        ["netstat", ["-ant"]],
      ];
}

/** Where the run writes what it saw: the peak and, look by look, the counts (for the report). */
const RESULT_FOLDER = resolve(repoRoot, "test-results");
const SPEC_FOLDER = resolve(repoRoot, "web/app/e2e");

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
 * end when more were left over at once than `MOST_LEFTOVER_CONNECTIONS` (TEST-CONNECTIONS).
 */
export default class ConnectionGuardReporter implements Reporter {
  private readonly ports = readPorts();
  private readonly guard = new ConnectionGuard();
  /** The spec each worker ran last, by the port of its server. */
  private readonly specs: Record<number, string> = {};
  private readonly series = ["seconds,established,time_wait,close_wait"];
  private started = Date.now();
  private timer: NodeJS.Timeout | undefined;
  private looking: Promise<void> = Promise.resolve();

  public onBegin(): void {
    this.started = Date.now();
    this.timer = setInterval(() => {
      this.looking = this.looking.then(() => this.look());
    }, SAMPLE_EVERY_MS);
  }

  public onTestBegin(test: TestCase, result: TestResult): void {
    const file = relative(SPEC_FOLDER, test.location.file).replaceAll("\\", "/");
    const project = test.parent.project()?.name ?? "";
    this.specs[this.ports.e2eWorkersFrom + result.parallelIndex] = `${file} [${project}]`;
  }

  public async onEnd(): Promise<{ status: FullResult["status"] } | undefined> {
    if (this.timer !== undefined) {
      clearInterval(this.timer);
    }
    await this.looking;
    await this.look();
    const summary = this.guard.summary();
    const verdict = this.guard.verdict();
    mkdirSync(RESULT_FOLDER, { recursive: true });
    writeFileSync(resolve(RESULT_FOLDER, "connections.csv"), `${this.series.join("\n")}\n`);
    writeFileSync(resolve(RESULT_FOLDER, "connections.txt"), `${summary}\n${verdict ?? ""}\n`);
    process.stdout.write(`${summary}
`);
    if (verdict === undefined) {
      return undefined;
    }
    console.error(verdict);
    return { status: "failed" };
  }

  public printsToStdio(): boolean {
    return false;
  }

  private async look(): Promise<void> {
    const text = await listing(listers(this.ports.e2eWorkersFrom, this.ports.e2eWorkersTo));
    if (text === null) {
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
