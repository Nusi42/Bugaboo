import { dataOf, type Schema } from "@bugaboo/api-client";

import { signInThroughApi } from "./accounts.ts";
import { createAgent } from "./agent-work.ts";
import { readFields } from "./fields.ts";
import { createProjectWithSession } from "./runs.ts";
import { sampleAdministrator } from "./sample.ts";
import { type Instance } from "./server.ts";
import { ISSUE, ISSUE_PROJECT, showcaseIds } from "./showcase-sample.ts";
import { filled, paths, routeParam, type Path, type RouteParam } from "../../src/shared/paths.ts";
import { places } from "../../src/shared/places.ts";

/** One screen of the route table, at an address that opens it on the sample. */
export interface ScreenAddress {
  /** The route, as the table of screens names it (`/settings/people/$person`). */
  readonly path: Path;
  readonly address: string;
}

/** The project the screens of the run of an AI app are made in: a project of its own, so the sample stays as it is. */
const RUN_PROJECT = { shortName: "TCH", name: "Touch" } as const;

/** The built-in theme the own themes of the screens extend. */
const BASE_THEME = "daylight";

/** What the things that only the sample's administrator makes are called: their names say what they are for. */
const madeName = "Every screen";

/** How many times the values were made on this run: a second time names its things apart (a theme's name is its own). */
let madeTimes = 0;

/** The value of every changing part of an address, and the personal theme that `/me/appearance/themes/…` opens. */
export type ScreenValues = Readonly<Record<RouteParam, string>> & {
  readonly personalTheme: string;
};

/**
 * The value of every changing part of an address on the sample, as its administrator sees it: what the sample has
 * (a project, an issue, a person, a group, a role, an own field), and what it has not, made through the API (a saved
 * search, an agent, two own themes, the run of an AI app).
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

/** The address of a route with its changing part filled in (no route has two); the page of a personal theme opens one. */
function addressOf(path: Path, values: ScreenValues): string {
  const param = Object.values(routeParam).find((each) => filled(path, each, "") !== path);
  if (param === undefined) {
    return path;
  }
  return filled(path, param, path === paths.myTheme ? values.personalTheme : values[param]);
}

/** Every screen of the route table (`places`), at an address on the sample. */
export function everyScreen(values: ScreenValues): readonly ScreenAddress[] {
  return places.map((place) => ({ path: place.path, address: addressOf(place.path, values) }));
}
