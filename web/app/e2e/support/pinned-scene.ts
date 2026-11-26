import { dataOf } from "@bugaboo/api-client";

import { createAdmin } from "./accounts.ts";
import { createProject } from "./projects.ts";
import { type Instance } from "./server.ts";

// The scene of *Pinned* (CFG-05) for the accessibility suite and its pictures: the project WEB and a saved search,
// both pinned by Ada.

export const pinnedProject = { shortName: "WEB", name: "Website" } as const;
export const pinnedSearch = "Open bugs in Website";

/** The ID of the pinned saved search, after `preparePinned`. */
let searchId = "";

/** Makes the project and the saved search and pins both, as Ada. */
export async function preparePinned(server: Instance): Promise<void> {
  const admin = await createAdmin(server);
  const project = await createProject(admin, pinnedProject.shortName, pinnedProject.name);
  searchId = dataOf(
    await admin.client.POST("/api/v1/saved-searches", {
      body: { name: pinnedSearch, search_text: "" },
    }),
  ).id;
  await admin.client.POST("/api/v1/me/pins", { body: { kind: "project", target: project } });
  await admin.client.POST("/api/v1/me/pins", {
    body: { kind: "saved-search", target: searchId },
  });
  await admin.dispose();
}

/** The ID of the pinned saved search. */
export function pinnedSearchId(): string {
  return searchId;
}
