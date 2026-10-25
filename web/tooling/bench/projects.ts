import layout from "../../../tools/dev/sample.json" with { type: "json" };
import { type ApiSession } from "../../app/e2e/support/api.ts";
import { listPage } from "../../app/e2e/support/perf-reads.ts";
import { inOrder } from "../../app/e2e/support/sequence.ts";

/** How many projects at the end of the list are counted to find the smallest. */
const TAIL_COUNTED = 10;
/** How many pages of a project are counted at most. */
const PAGES_COUNTED = 40;

/** A project and how many issues it has. */
interface Counted {
  readonly project: string;
  readonly count: number;
}

/** How many issues a project has, up to `PAGES_COUNTED` pages of them. */
async function countOf(
  session: ApiSession,
  project: string,
  cursor?: string,
  pages = 0,
  counted = 0,
): Promise<number> {
  const page = await listPage(session, project, cursor);
  const total = counted + page.items.length;
  const next = page.next_cursor ?? undefined;
  return next === undefined || pages + 1 >= PAGES_COUNTED
    ? total
    : countOf(session, project, next, pages + 1, total);
}

/**
 * The projects of the list scenarios. The big one is the first project of the sample (the dataset
 * ranks the sample's projects among the biggest, and the first of them first); it must have more
 * than one page of issues. The small one is the one with the fewest issues, counted, among the last
 * projects of the list; it must have some and fewer than the big one. Fails when either cannot be
 * found, so a scenario never measures a project that is not what its name says.
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
  const counts: Counted[] = [];
  await inOrder(
    projects.slice(-TAIL_COUNTED).map((project) => async () => {
      counts.push({ project, count: await countOf(session, project) });
    }),
  );
  const filled = counts.filter((each) => each.count > 0);
  const fewest = Math.min(...filled.map((each) => each.count));
  const smallest = filled.find((each) => each.count === fewest);
  if (smallest === undefined) {
    throw new Error("The last projects of the dataset have no issues.");
  }
  return { big, small: smallest.project };
}
