import { dataOf } from "@bugaboo/api-client";

import { signedInAdmin } from "./access.ts";
import { type ApiSession } from "./api.ts";
import { prepareOffers } from "./offers.ts";
import { inOrder } from "./sequence.ts";
import { type Instance } from "./server.ts";

// The state of the server of the pictures of the screens of M4 WP-11 (see
// `screenshots-offers.spec.ts`).

/**
 * The server of the pictures: the projects Website and Lab with the lead, Bob's role and a bug, and
 * Website limited to Bug, a group of labels created for Website and one for the organization.
 */
export async function prepareOffersLook(server: Instance): Promise<void> {
  await prepareOffers(server);
  const admin = await signedInAdmin(server);
  await limitTypes(admin);
  await admin.dispose();
}

/** Website offers Bug only; Bug is offered nowhere else (lines for the organization and Website). */
async function limitTypes(admin: ApiSession): Promise<void> {
  const types = dataOf(await admin.client.GET("/api/v1/issue-types")).items;
  await inOrder(
    types.map((type) => async () => {
      const changes =
        type.name === "Bug"
          ? [{ offered: false }, { project: "WEB", offered: true }]
          : [{ project: "WEB", offered: false }];
      await admin.client.PUT("/api/v1/issue-types/{issue_type}/offers", {
        params: { path: { issue_type: type.id ?? "" } },
        body: { version: type.version ?? 1, changes },
      });
    }),
  );
  const group = dataOf(
    await admin.client.POST("/api/v1/label-groups", {
      body: { name: "Area", pick_one: false, project: "WEB" },
    }),
  );
  await admin.client.POST("/api/v1/labels", { body: { name: "login", group: group.id } });
  await admin.client.POST("/api/v1/label-groups", {
    body: { name: "Platform", pick_one: false },
  });
}
