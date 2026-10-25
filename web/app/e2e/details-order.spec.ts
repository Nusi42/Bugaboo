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

/** The built-in fields of the Details in the order the test wants, with the name of their row. */
const rowsWanted = [
  ["tags", t("labels.tags-heading")],
  ["labels", t("labels.labels-heading")],
  ["assigned-to", t("issue-field.assigned-to")],
  ["priority", t("issue-field.priority")],
  ["reported-by", t("issue-field.reported-by")],
] as const;
/** The fields of the issue page in the order the test wants, after *Description* and *Status*. */
const wanted = rowsWanted.map(([word]) => word);
/** The rows that the layout of the test orders, in its order. */
const order: readonly string[] = rowsWanted.map(([, name]) => name);

/** Makes the layout of the issue page of the project WEB with the fields in the wanted order. */
async function prepareDetailsOrder(server: Instance): Promise<void> {
  await prepareWeb(server);
  const admin = await signedInAdmin(server);
  const query = { kind: "issue-page", project: "WEB" } as const;
  const created = dataOf(await admin.client.POST("/api/v1/layouts", { body: query }));
  const slot = created.layout;
  if (slot === undefined || slot === null) {
    throw new TypeError("The layout was not created.");
  }
  const roles = created.roles.map((role) => role.id);
  const what = {
    record: { kind: "layout", id: slot.id },
    part: layoutEditPart,
  } as const;
  const taken = dataOf(
    await admin.client.POST("/api/v1/edit-locks", {
      body: { target: what, editor_id: crypto.randomUUID() },
    }),
  );
  if (taken.kind !== "taken") {
    throw new TypeError("Somebody else is editing the layout.");
  }
  const field = (word: Schema<"EditorFieldBody">["field"]) => ({
    field: word,
    roles,
  });
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
  // The rows of Tags, Labels and Followers come from parts that load when the page is opened:
  // the last of them is waited for, so the order is read when every row is there.
  await expect(rows.filter({ hasText: t("followers.name") })).toBeVisible();
  await expect(rows.filter({ hasText: t("labels.labels-heading") })).toBeVisible();
  const names = await rows.allInnerTexts();
  expect(names.filter((name) => order.includes(name))).toEqual(order);
});

test("cfg_01 the Details draw no row before the layout is read, so no row moves when it arrives (M3-F102)", async ({
  page,
}) => {
  await signIn(page, ada);
  // The layout is held until the test lets it go: no time decides what is seen meanwhile.
  const held = Promise.withResolvers<void>();
  await page.route("**/api/v1/layouts/form*", async (route) => {
    await held.promise;
    await route.continue();
  });
  await openIssueWithDetails(page, "WEB-1");
  const details = page.getByRole("complementary", {
    name: t("issues.details"),
  });
  await expect(details).toHaveAttribute("aria-busy", "true");
  const rows = partIn(page, "name-value.name");
  await expect(rows).toHaveCount(0);
  held.resolve();
  await expect(details).toHaveAttribute("aria-busy", "false");
  await expect(rows.filter({ hasText: t("labels.labels-heading") })).toBeVisible();
  const names = await rows.allInnerTexts();
  expect(names.filter((name) => order.includes(name))).toEqual(order);
});
