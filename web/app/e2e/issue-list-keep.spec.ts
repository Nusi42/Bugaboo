import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { paths, projectIssuesAddress, savedSearchAddress } from "../src/shared/paths.ts";
import { projectIdOf, signedInAdmin } from "./support/access.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { webIssues } from "./support/issues.ts";
import { grid, inViewChoices, listFrame, prepareLook, rowOf } from "./support/list-look.ts";
import { type Instance } from "./support/server.ts";

// What a list keeps (UX §4.2, QRY-06): a saved search keeps the view it was changed to, and
// *Remove all filters* on a project's list that has a text of its own shows everything. How close
// together the rows are is no part of it: every list follows the person's Spacing (ADR-0034,
// amendment).

let listId = "";

async function prepare(server: Instance): Promise<void> {
  await prepareLook(server);
  const admin = await signedInAdmin(server);
  listId = dataOf(
    await admin.client.POST("/api/v1/saved-searches", {
      body: { name: "Only critical", search_text: "priority is critical" },
    }),
  ).id;
  await admin.client.POST("/api/v1/saved-searches/{search}/shares", {
    params: { path: { search: listId } },
    body: { kind: "project", id: await projectIdOf(admin, "WEB") },
  });
  await admin.client.PUT("/api/v1/projects/{project}/list", {
    params: { path: { project: "WEB" } },
    body: { search: listId },
  });
  await admin.dispose();
}

test.use({ preparation: { run: prepare } });
test.describe.configure({ mode: "serial" });

test("ux_4_2 Remove all filters on a project list with a text of its own shows every issue", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  await expect(rowOf(page, webIssues.login)).toBeVisible();
  await expect(rowOf(page, webIssues.typo)).toHaveCount(0);
  // The text of the list is in the box; emptying it asks for everything, and the address says so.
  await page.goto(`${projectIssuesAddress("WEB")}?s=priority%20is%20low`);
  await page.getByRole("button", { name: t("issues.remove-filters"), exact: true }).click();
  await expect(rowOf(page, webIssues.typo)).toBeVisible();
  await expect(rowOf(page, webIssues.login)).toBeVisible();
  expect(new URL(page.url()).searchParams.get("s")).toBe("");
  // The address keeps it on a reload: it does not fall back to the text of the list.
  await page.reload();
  await expect(rowOf(page, webIssues.typo)).toBeVisible();
});

test("qry_06 a saved search keeps a change of its view with Save", async ({ page, server }) => {
  await signIn(page, ada);
  await page.goto(savedSearchAddress(listId));
  await expect(page.getByRole("heading", { level: 1, name: "Only critical" })).toBeVisible();
  await inViewChoices(page, async (scope) => {
    await scope
      .getByLabel(t("issues.group-by"), { exact: true })
      .selectOption({ label: t("issue-field.priority") });
  });
  await page.getByRole("button", { name: t("search.save-changes"), exact: true }).click();
  await expect(page.getByRole("button", { name: t("search.save-changes") })).toHaveCount(0);
  const admin = await signedInAdmin(server);
  const kept = dataOf(
    await admin.client.GET("/api/v1/saved-searches/{search}", {
      params: { path: { search: listId } },
    }),
  );
  expect(kept.presentation.group_by).toBe("priority");
  expect(kept.presentation).not.toHaveProperty("spacing");
  await admin.dispose();
});

test("thm_06 every list follows Spacing in My settings › Appearance, and an old address's spacing is ignored", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(paths.appearance);
  await page.getByRole("radio", { name: t("appearance.spacing.compact"), exact: true }).check();
  await expect(page.getByRole("status").filter({ hasText: t("save.saved") })).toBeVisible();
  await page.goto(projectIssuesAddress("WEB"));
  await expect(grid(page)).toBeVisible();
  await expect(listFrame(page)).toHaveAttribute("data-variant", /compact/u);
  // Neither the list nor its Filters sheet has a Spacing of its own.
  await inViewChoices(page, async (scope) => {
    await expect(scope.getByText(t("issues.group-by"), { exact: true })).toBeVisible();
    await expect(scope.getByText(t("appearance.spacing.legend"), { exact: true })).toHaveCount(0);
  });
  // A saved search follows it too.
  await page.goto(savedSearchAddress(listId));
  await expect(page.getByRole("heading", { level: 1, name: "Only critical" })).toBeVisible();
  await expect(listFrame(page)).toHaveAttribute("data-variant", /compact/u);
  // Back to Comfortable: an address written before, which still says Compact, opens Comfortable.
  await page.goto(paths.appearance);
  await page.getByRole("radio", { name: t("appearance.spacing.comfortable"), exact: true }).check();
  await expect(page.getByRole("status").filter({ hasText: t("save.saved") })).toBeVisible();
  await page.goto(`${projectIssuesAddress("WEB")}?spacing=compact`);
  await expect(grid(page)).toBeVisible();
  await expect(listFrame(page)).not.toHaveAttribute("data-variant", /compact/u);
});
