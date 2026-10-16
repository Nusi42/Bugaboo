import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { ada, bob, createAdmin, createPerson } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { openFromMenu, signIn } from "./support/flows.ts";
import { paths } from "../src/shared/paths.ts";

// Kinds of relation (ISS-04, WP-07): the four kinds of the first start, each named for both ends,
// making one, changing its words and retiring it, in every theme; and a person who may not manage
// them does not find the page.

test.use({
  preparation: {
    run: async (server) => {
      const admin = await createAdmin(server);
      await (await createPerson(server, admin, bob)).dispose();
      await admin.dispose();
    },
  },
});
test.describe.configure({ mode: "serial" });

/** The list of kinds of relation on the page. */
function kindList(page: Page) {
  return page.getByRole("list", { name: t("relation-kinds.search"), exact: true });
}

/** The name of a kind in the list: the words for both ends. */
function reads(forward: string, backward: string): string {
  return t("relation-kinds.reads", { forward, backward });
}

test("iss_04 an administrator sees the four kinds of the first start and opens one", async ({
  page,
}) => {
  await signIn(page, ada);
  await openFromMenu(page, t("nav.relation-kinds"));
  await expect(
    page.getByRole("heading", { level: 1, name: t("nav.relation-kinds") }),
  ).toBeVisible();
  const list = kindList(page);
  await expect(list.getByRole("listitem")).toHaveCount(4);
  await expect(list.getByRole("button")).toHaveText([
    reads("blocks", "is blocked by"),
    reads("duplicate of", "has duplicate"),
    reads("causes", "is caused by"),
    reads("related to", "related to"),
  ]);
  await list.getByRole("button", { name: reads("causes", "is caused by"), exact: true }).click();
  const panel = page.getByRole("complementary", { name: reads("causes", "is caused by") });
  await expect(panel.getByLabel(t("relation-kinds.field-forward"), { exact: true })).toHaveValue(
    "causes",
  );
});

test("iss_04 a kind is made, its words are changed, and it is retired and offered again", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(paths.relationKinds);
  await page.getByLabel(t("relation-kinds.field-forward"), { exact: true }).fill("precedes");
  await page.getByLabel(t("relation-kinds.field-backward"), { exact: true }).fill("follows");
  await page.getByRole("button", { name: t("relation-kinds.create"), exact: true }).click();
  await expect(page.getByText(t("relation-kinds.created", { name: "precedes" }))).toBeVisible();
  // A name that another kind has, in either direction, is taken.
  await page.getByLabel(t("relation-kinds.field-forward"), { exact: true }).fill("Follows");
  await page.getByLabel(t("relation-kinds.field-backward"), { exact: true }).fill("comes before");
  await page.getByRole("button", { name: t("relation-kinds.create"), exact: true }).click();
  await expect(page.getByText(t("error.name.taken"))).toBeVisible();
  const list = kindList(page);
  await expect(list.getByRole("listitem")).toHaveCount(5);
  await list.getByRole("button", { name: reads("precedes", "follows"), exact: true }).click();
  const panel = page.getByRole("complementary", { name: reads("precedes", "follows") });
  await panel.getByText(t("relation-kinds.retire"), { exact: true }).click();
  await expect(
    panel.getByRole("switch", { name: t("relation-kinds.offer-again"), exact: true }),
  ).toBeChecked();
  await page.goto(paths.relationKinds);
  await expect(list.getByText(t("relation-kinds.retired"))).toBeVisible();
});

test("iss_04 Kinds of relation is good in every theme, also with a kind open", async ({ page }) => {
  await signIn(page, ada);
  await openFromMenu(page, t("nav.relation-kinds"));
  await expect(kindList(page).getByRole("listitem")).not.toHaveCount(0);
  await expectGoodInEveryLook(page);
  await kindList(page)
    .getByRole("button", { name: reads("blocks", "is blocked by"), exact: true })
    .click();
  await expect(
    page.getByRole("complementary", { name: reads("blocks", "is blocked by") }),
  ).toBeVisible();
  await expectGoodInEveryLook(page);
});

test("iss_04 a person who may not manage kinds of relation does not find the page", async ({
  page,
}) => {
  await signIn(page, bob);
  await page.goto(paths.relationKinds);
  await expect(
    page.getByRole("heading", { level: 1, name: t("page.not-found.title") }),
  ).toBeVisible();
});
