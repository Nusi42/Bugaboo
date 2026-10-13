import { readPorts } from "../../../../tooling/ports.ts";
import { startServer } from "../../support/server.ts";
import { DATABASE_URL_VARIABLE, type LoadOptions, parseOptions } from "./options.ts";
import { run } from "./run.ts";

// `cargo xtask load --people 10` (architecture §11, TEST-CONCURRENT): starts the server of the
// load test on the database of the run, lets the people work, checks what must hold afterwards and
// says what did not. The exit code is 1 when anything did not hold. xtask passes the address of
// the PostgreSQL database in the variable named in `options.ts`.

/** The settings that point the server at the database of the run (SQLite is the default). */
function databaseSettings(options: LoadOptions): Readonly<Record<string, string>> {
  if (options.database === "sqlite") {
    return {};
  }
  const url = process.env[DATABASE_URL_VARIABLE];
  if (url === undefined || url === "") {
    throw new TypeError(
      `A run on PostgreSQL needs the address of the database in ${DATABASE_URL_VARIABLE}.`,
    );
  }
  return { BUGABOO_DATABASE_URL: url };
}

async function main(): Promise<void> {
  const options = parseOptions(process.argv.slice(2));
  const server = await startServer(
    readPorts().loadTestServer,
    `load-${options.database}`,
    databaseSettings(options),
  );
  try {
    const { said, failures } = await run(server, options);
    process.stdout.write(`${said}\n`);
    if (failures.length > 0) {
      process.stderr.write(
        `${String(failures.length)} things did not hold:\n${failures.join("\n")}\n`,
      );
      process.exitCode = 1;
    }
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
