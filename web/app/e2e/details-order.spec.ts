import { dataOf, layoutEditPart, type Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { issueAddress } from "../src/shared/paths.ts";
import { signedInAdmin } from "./support/access.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { openIssueWithDetails, signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { type Instance } from "./support/server.ts";
import { partIn } from "./support/parts.ts";

// The Details follow the layout of the issue page (CFG-01, M3-F102): the rows stand in the order
// in which the layout lists their fields, the one order that the form uses too.

test.use({ preparation: { run: prepareDetailsOrder } });

/** The fields of the issue page in the order the test wants, after *Description* and *Status*. */
const wanted = ["tags", "labels", "assigned-to", "priority", "reported-by"] as const;

/** Makes the layout of the issue page of the project WEB with the fields in the wanted order. */
async function prepareDetailsOrder(server: Instance): Promise<void> {
  await prepareWeb(server);
  const admin = await signedInAdmin(server);
  const query = { kind: "issue-page", project: "WEB" } as const;
  const made = dataOf(await admin.client.POST("/api/v1/layouts", { body: query }));
  const slot = made.layout;
  if (slot === undefined || slot === null) {
    throw new TypeError("The layout was not made.");
  }
  const roles = made.roles.map((role) => role.id);
  const what = { record: { kind: "layout", id: slot.id }, part: layoutEditPart };
  const taken = dataOf(
    await admin.client.POST("/api/v1/edit-locks", {
      body: { what, editor_id: crypto.randomUUID() },
    }),
  );
  if (taken.status !== "taken") {
    throw new TypeError("Somebody else is editing the layout.");
  }
  const field = (word: Schema<"EditorFieldBody">["field"]) => ({ field: word, roles });
  dataOf(
    await admin.client.PUT("/api/v1/layouts/{layout}", {
      params: { path: { layout: slot.id } },
      body: {
        version: slot.version,
        lock_token: taken.lock_token,
        sections: [
          {
            tab: "",
            title: "",
            fields: ["description", "status", ...wanted].map((word) => field(word)),
          },
        ],
      },
    }),
  );
  await admin.dispose();
}

test("cfg_01 the rows of the Details stand in the order of the layout (M3-F102)", async ({
  page,
}) => {
  await signIn(page, ada);
  await openIssueWithDetails(page, "WEB-1");
  await expect(page).toHaveURL(new RegExp(issueAddress("WEB-1"), "u"));
  const rows = partIn(page, "name-value.name");
  await expect(rows.first()).toBeVisible();
  const names = await rows.allInnerTexts();
  const order = [
    t("labels.tags-heading"),
    t("labels.labels-heading"),
    t("issue-field.assigned-to"),
    t("issue-field.priority"),
    t("issue-field.reported-by"),
  ];
  expect(names.filter((name) => order.includes(name))).toEqual(order);
});
