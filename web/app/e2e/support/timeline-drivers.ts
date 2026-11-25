import { dataOf } from "@bugaboo/api-client";

import { ada, signInThroughApi, type Instance } from "./accounts.ts";
import { dayFromToday } from "./person-day.ts";
import { changeIssue, planned, prepareTimeline } from "./timeline.ts";

// The scene of deadline drivers and releases (PLAN-05, ISS-08, ADR-0043 amendment of 2026-11-24):
// the scene of the timeline, where WEB-1 must finish before WEB-2, both under the goal; and the
// release 2.5, which WEB-2 is planned for and ends after.

/** The release of the scene, and its *Release date* in days from today. */
export const release = { name: "2.5", day: 6 } as const;

/** Gets the server ready: the timeline's scene, with the release and the issue planned for it. */
export async function prepareDrivers(server: Instance): Promise<void> {
  await prepareTimeline(server);
  const admin = await signInThroughApi(server, ada);
  const made = dataOf(
    await admin.client.POST("/api/v1/projects/{project}/releases", {
      params: { path: { project: "WEB" } },
      body: { name: release.name, release_date: dayFromToday(release.day) },
    }),
  );
  await changeIssue(admin, planned.waits, { planned_for_release: made.id });
  await admin.dispose();
}
