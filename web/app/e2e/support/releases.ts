import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { type Locator, type Page } from "@playwright/test";

import { ada, signInThroughApi, type ApiSession, type Instance } from "./accounts.ts";
import { prepareWeb } from "./issues.ts";
import { dayFromToday } from "./person-day.ts";
import { changeIssue } from "./timeline.ts";

// The scene of the tests of the releases (ISS-08, ADR-0049, UX §4.16): the project WEB of the tests of
// issues with the releases *2.4* (an issue is planned for it, so it cannot be deleted), *2.5*, and *2.3*,
// which is retired.

/** The releases of the scene, by what they are for. */
export const releases = {
  /** WEB-1 is planned for it: a delete is refused. */
  named: "2.4",
  open: "2.5",
  retired: "2.3",
} as const;

/** The state a release is marked with. */
type State = "planned" | "released" | "retired";

/** Makes a release in WEB, in *Planned*, and puts it into another state when it is asked; answers its ID. */
async function createReleaseFor(
  admin: ApiSession,
  name: string,
  days: number,
  state: State = "planned",
): Promise<string> {
  const made = dataOf(
    await admin.client.POST("/api/v1/projects/{project}/releases", {
      params: { path: { project: "WEB" } },
      body: { name, release_date: dayFromToday(days) },
    }),
  );
  if (state !== "planned") {
    dataOf(
      await admin.client.PATCH("/api/v1/projects/{project}/releases/{release}", {
        params: { path: { project: "WEB", release: made.id ?? "" } },
        body: { version: made.version ?? 1, state },
      }),
    );
  }
  return made.id ?? "";
}

/** The IDs of the releases of the scene, once the server is ready. */
export const releaseIds = { named: "", open: "", retired: "" };

/** Gets the server ready: WEB, three releases, and WEB-1 planned for *2.4*. */
export async function prepareReleases(server: Instance): Promise<void> {
  await prepareWeb(server);
  const admin = await signInThroughApi(server, ada);
  releaseIds.named = await createReleaseFor(admin, releases.named, 5);
  releaseIds.open = await createReleaseFor(admin, releases.open, 20);
  releaseIds.retired = await createReleaseFor(admin, releases.retired, -30, "retired");
  await changeIssue(admin, "WEB-1", { planned_for_release: releaseIds.named });
  await admin.dispose();
}

/** Opens the page *Releases* of WEB. */
export async function openReleases(page: Page): Promise<void> {
  await page.goto("/p/WEB/releases");
}

/** The table of the releases of one state (*Planned*, *Released*), named by the state. */
export function tableOf(page: Page, state: string): Locator {
  return page.getByRole("table", { name: state, exact: true });
}

/** The row of a release in the table of its state. */
export function rowOf(page: Page, state: string, name: string): Locator {
  return tableOf(page, state).getByRole("row").filter({ hasText: name });
}

/** The button that opens the window of a release. */
export function changeButton(page: Page, name: string): Locator {
  return page.getByRole("button", { name: t("releases.change-name", { name }) });
}
