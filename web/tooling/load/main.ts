import { readPorts } from "../ports.ts";
import { startServer } from "../../app/e2e/support/server.ts";
import { DATABASE_URL_VARIABLE, type LoadOptions, parseOptions } from "./options.ts";
import { run } from "./run.ts";

// `cargo xtask load --people 10` (architecture §11, TEST-CONCURRENT): starts the server of the
// load test on the database of the run and lets the people work. It writes the book of the run to
// a file and leaves the data folder in place; xtask then runs `bb-load-check`, which judges the
// database against the book. xtask passes the address of the PostgreSQL database in the variable
// named in `options.ts`.

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
    false,
    options.dataDir,
  );
  try {
    process.stdout.write(`${await run(server, options)}
`);
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
