// The start of the Node programs that xtask runs without a bundler: the driver of the benchmarks
// (`cargo xtask bench`) and the one of the load test (`cargo xtask load`). Plain Node loads only
// what it can strip the types of, so a module of the screens (`.tsx`, a bundler's import) in their
// graph stops them before they measure anything; `driver.test.ts` loads each of them that way.

/** The argument that makes a driver stop as soon as its modules are loaded, before it starts anything. */
export const LOAD_ONLY = "--load-only";

/**
 * Runs the work of a driver: nothing when it was started with `LOAD_ONLY`; otherwise the work, and
 * on a failure its message on standard error and exit code 1.
 */
export async function runDriver(work: () => Promise<void>): Promise<void> {
  if (process.argv.includes(LOAD_ONLY)) {
    return;
  }
  await work().catch((error: unknown) => {
    process.stderr.write(
      `${error instanceof Error ? (error.stack ?? error.message) : String(error)}\n`,
    );
    process.exitCode = 1;
  });
}
