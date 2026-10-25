import layout from "../../../tools/dev/sample.json" with { type: "json" };
import { type ApiSession } from "../../app/e2e/support/api.ts";
import { listPage } from "../../app/e2e/support/perf-reads.ts";

/** The first of the projects (the last one first) that has issues and less than a page of them. */
async function firstSmall(session: ApiSession, projects: readonly string[]): Promise<string> {
  const [project, ...rest] = projects;
  if (project === undefined) {
    throw new Error("The dataset has no project with issues of less than one page.");
  }
  const page = await listPage(session, project);
  return page.items.length > 0 && page.next_cursor === undefined
    ? project
    : firstSmall(session, rest);
}

/**
 * The projects of the list scenarios. The big one is the first project of the sample (the dataset
 * ranks the sample's projects among the biggest, and the first of them first); it must have more
 * than one page of issues. The small one is the last project of the list that has issues and less
 * than a page of them, counted by reading its first page. Fails when either cannot be found, so a
 * scenario never measures a project that is not what its name says.
 */
export async function bigAndSmall(
  session: ApiSession,
  projects: readonly string[],
): Promise<{ readonly big: string; readonly small: string }> {
  const big = layout.organization.projects[0]?.shortName ?? "";
  if (!projects.includes(big)) {
    throw new Error(`The dataset has no project ${big} of the sample.`);
  }
  if ((await listPage(session, big)).next_cursor === undefined) {
    throw new Error(`The project ${big} has no more than one page of issues; it is not big.`);
  }
  return { big, small: await firstSmall(session, projects.toReversed()) };
}
