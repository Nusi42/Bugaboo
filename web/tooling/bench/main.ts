import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";

import { startServer } from "../../app/e2e/support/server.ts";
import { readPorts } from "../ports.ts";
import { run } from "./run.ts";

// `cargo xtask bench` (PERF-BENCH): starts a release server on the large dataset of one database,
// warms up, runs every scenario of `scenarios.ts` a fixed number of times and writes p50, p95 and
// p99 of each to a file. xtask judges the file against the budgets of `perf-bench.toml`.
// Arguments: --folder (the dataset: its data folder and sign-in.txt), --database, --result.
// For PostgreSQL the address of the database is in BB_BENCH_DATABASE_URL.

function flag(name: string): string {
  const at = process.argv.indexOf(name);
  const value = at < 0 ? undefined : process.argv[at + 1];
  if (value === undefined || value === "") {
    throw new TypeError(`${name} needs a value.`);
  }
  return value;
}

async function main(): Promise<void> {
  const folder = flag("--folder");
  const database = flag("--database");
  const settings: Record<string, string> =
    database === "postgres"
      ? { BUGABOO_DATABASE_URL: process.env["BB_BENCH_DATABASE_URL"] ?? "" }
      : {};
  const server = await startServer(
    readPorts().loadTestServer,
    `bench-${database}`,
    settings,
    false,
    join(folder, "data"),
  );
  try {
    const result = flag("--result");
    const found = await run(folder, database, server);
    mkdirSync(dirname(result), { recursive: true });
    writeFileSync(result, `${JSON.stringify(found, null, 2)}\n`);
  } finally {
    await server.stop();
  }
}

await main().catch((error: unknown) => {
  process.stderr.write(
    `${error instanceof Error ? (error.stack ?? error.message) : String(error)}\n`,
  );
  process.exitCode = 1;
});
