import { dataOf, manyChangesIssuesMax, pageSizeMax } from "@bugaboo/api-client";

import { signInThroughApi } from "../../app/e2e/support/accounts.ts";
import { buildOf } from "../../app/e2e/support/perf.ts";
import { peopleIn } from "../../app/e2e/support/sample.ts";
import { inOrder } from "../../app/e2e/support/sequence.ts";
import { type Instance } from "../../app/e2e/support/server.ts";
import { measure } from "./measure.ts";
import { bigAndSmall } from "./projects.ts";
import { firstIssues, someIssues } from "./reads.ts";
import { type Context, type Scenario, scenarios } from "./scenarios.ts";

/**
 * The scenarios to measure: all of them, or those named in `BB_BENCH_ONLY` (names separated by
 * commas) when one is worked on; the numbers then hold only those.
 */
function chosen(all: readonly Scenario[]): readonly Scenario[] {
  const only = (process.env["BB_BENCH_ONLY"] ?? "")
    .split(",")
    .map((name) => name.trim())
    .filter((name) => name !== "");
  return only.length === 0 ? all : all.filter((scenario) => only.includes(scenario.name));
}

/** Signs in as the first Administrator of the dataset and measures every scenario. */
export async function run(folder: string, database: string, server: Instance): Promise<unknown> {
  const administrator = peopleIn(folder).find((person) => person.role === "administrator");
  if (administrator === undefined) {
    throw new Error("The dataset has no Administrator to sign in as.");
  }
  const session = await signInThroughApi(server, administrator);
  const listed = dataOf(
    await session.client.GET("/api/v1/projects", { params: { query: { limit: pageSizeMax } } }),
  );
  const projects = listed.items.map((item) => String(item.short_name));
  const places = await bigAndSmall(session, projects);
  const context: Context = {
    session,
    issues: await someIssues(session, projects),
    many: await firstIssues(session, places.big, manyChangesIssuesMax),
    ...places,
  };
  const measured: Record<string, unknown> = {};
  await inOrder(
    chosen(scenarios).map((scenario) => async () => {
      measured[scenario.name] = await measure(scenario, context);
    }),
  );
  await session.dispose();
  return {
    build: buildOf(process.env["BB_SERVER_BIN"]),
    database,
    scenarios: measured,
  };
}
