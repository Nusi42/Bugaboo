import { mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { describe, expect, it, vi } from "vitest";

import ConnectionGuardReporter from "./connection-guard.ts";

// The computer is not asked: a list of no connection is what every look reads.
vi.mock("node:child_process", () => ({
  execFile: (
    _command: string,
    _args: readonly string[],
    _options: Readonly<object>,
    done: (error: null, out: string) => void,
  ) => {
    done(null, "");
  },
}));

const folder = mkdtempSync(join(tmpdir(), "guard-"));
const config = { projects: [{ testDir: folder, outputDir: folder }] };

/** A test of a spec of the folder that ran in the project `desktop`. */
const test = {
  location: { file: join(folder, "fields-issue.spec.ts") },
  parent: { project: () => ({ name: "desktop", dependencies: [] }) },
};

/** Runs the reporter over one test that failed with this message; what it printed to stderr. */
async function printedWhenFailedWith(message: string): Promise<string> {
  const printed = vi.spyOn(console, "error").mockImplementation(() => {
    // Silent: the test reads what was printed.
  });
  vi.spyOn(process.stdout, "write").mockReturnValue(true);
  const reporter = new ConnectionGuardReporter();
  reporter.onBegin(config);
  reporter.onTestEnd(test, { errors: [{ message }] });
  await reporter.onEnd();
  const lines = printed.mock.calls.flat().map(String);
  vi.restoreAllMocks();
  return lines.join("\n");
}

describe("the reporter of the connections", () => {
  it("prints the sentence about a run out of ports and writes it to connections.txt", async () => {
    const printed = await printedWhenFailedWith("page.goto: net::ERR_NO_BUFFER_SPACE");
    expect(printed).toContain("ERR_NO_BUFFER_SPACE (the first in fields-issue.spec.ts [desktop])");
    expect(readFileSync(join(folder, "connections.txt"), "utf8")).toContain(
      "fields-issue.spec.ts [desktop]",
    );
  });

  it("says nothing about ports when no test ran out of them", async () => {
    const printed = await printedWhenFailedWith("expected 1 to be 2");
    expect(printed).not.toContain("ERR_NO_BUFFER_SPACE");
    expect(readFileSync(join(folder, "connections.txt"), "utf8")).not.toContain(
      "ERR_NO_BUFFER_SPACE",
    );
  });
});
