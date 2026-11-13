import { dataOf, type Schema } from "@bugaboo/api-client";

import { signInThroughApi } from "./accounts.ts";
import { createAgent } from "./agent-work.ts";
import { readFields } from "./fields.ts";
import { createProjectWithSession } from "./runs.ts";
import { sampleAdministrator } from "./sample.ts";
import { type Instance } from "./server.ts";
import { changeIssue, dayFromToday } from "./timeline.ts";
import { ISSUE, ISSUE_PROJECT, showcaseIds } from "./showcase-sample.ts";
import { routeParam, type RouteParam } from "../../src/shared/paths.ts";

/** The project the screens of the run of an AI app are made in: a project of its own, so the sample stays as it is. */
const RUN_PROJECT = { shortName: "TCH", name: "Touch" } as const;

/** How many days the issue of the screens takes on the timeline. */
const DATED_DAYS = 5;

/** The built-in theme the own themes of the screens extend. */
const BASE_THEME = "daylight";

/** What the things that only the sample's administrator makes are called: their names say what they are for. */
const madeName = "Every screen";

/**
 * How many times the values were made, from a start of this run's own: a second time, or a second run on a server that
 * kept its data, names its things apart (the name of a theme and the short name of a project are their own).
 */
let madeTimes = Date.now() % 100_000;

/** The value of every changing part of an address, and the personal theme that `/me/appearance/themes/…` opens. */
export type ScreenValues = Readonly<Record<RouteParam, string>> & {
  readonly personalTheme: string;
};

/**
 * The value of every changing part of an address on the sample, as its administrator sees it: what the sample has
 * (a project, an issue, a person, a group, a role, an own field), and what it has not, made through the API (a saved
 * search, an agent, two own themes, the run of an AI app, the dates of the issue).
 */
export async function screenValues(server: Instance): Promise<ScreenValues> {
  madeTimes += 1;
  const name = `${madeName} ${String(madeTimes)}`;
  const ids = await showcaseIds(server);
  const admin = await signInThroughApi(server, sampleAdministrator());
  try {
    const theme = async (kind: Schema<"ThemeKind">): Promise<string> =>
      dataOf(
        await admin.client.POST("/api/v1/themes", {
          body: { name: `${name} ${kind}`, extends: BASE_THEME, kind },
        }),
      ).id;
    const search = dataOf(
      await admin.client.POST("/api/v1/saved-searches", {
        body: { name, search_text: "" },
      }),
    ).id;
    // The issue of the screens has dates, so the timeline draws a bar a finger can press, not only the tray.
    await changeIssue(admin, ISSUE, {
      start_date: dayFromToday(0),
      due_date: dayFromToday(DATED_DAYS),
    });
    const made = await createProjectWithSession(
      server,
      admin,
      `${RUN_PROJECT.shortName}${String(madeTimes)}`,
      `${RUN_PROJECT.name} ${String(madeTimes)}`,
    );
    return {
      [routeParam.project]: ISSUE_PROJECT,
      [routeParam.person]: ids.person,
      [routeParam.group]: ids.group,
      [routeParam.role]: ids.role,
      [routeParam.field]: (await readFields(admin))[0]?.id ?? "",
      [routeParam.issue]: ISSUE,
      [routeParam.run]: made.run,
      [routeParam.search]: search,
      [routeParam.agent]: await createAgent(admin, name),
      [routeParam.theme]: await theme("for-everyone"),
      personalTheme: await theme("personal"),
    };
  } finally {
    await admin.dispose();
  }
}
