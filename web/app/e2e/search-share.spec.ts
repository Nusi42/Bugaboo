import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { ada, createAdmin, ownId, signInThroughApi } from "./support/accounts.ts";
import { addToGroup, ana, createGroup, createPeopleWithRoles, roleName } from "./support/access.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { nothing, onThisLayout } from "./support/layout.ts";
import { createProjectWithIssue } from "./support/issues.ts";
import { leftEdgeOf, openBar, wordsStartOf } from "./support/search-bar.ts";
import { savedSearchAddress } from "../src/shared/paths.ts";

// Share (QRY-06, ADR-0034): the window lists only the groups and the projects the person may share
// with, and nothing of the others is there to be found.

let anasSearch = "";

test.use({
  preparation: {
    run: async (server) => {
      const admin = await createAdmin(server);
      await createProjectWithIssue(admin, "WEB", "Website");
      await createProjectWithIssue(admin, "SEC", "Secret plans");
      const mine = await createGroup(admin, "Crew of Ana");
      await createGroup(admin, "Somebody else's crew");
      await createPeopleWithRoles(server, admin, [
        { person: ana, role: roleName.teamMember, project: "WEB" },
      ]);
      const session = await signInThroughApi(server, ana);
      await addToGroup(admin, mine, await ownId(session));
      anasSearch = dataOf(
        await session.client.POST("/api/v1/saved-searches", {
          body: { name: "Ana's list", search_text: "assigned to is me" },
        }),
      ).id;
      await session.dispose();
      await admin.dispose();
    },
  },
});

test("qry_06 a person who may share only for one project is offered that project and no group", async ({
  page,
}) => {
  await signIn(page, ana);
  await page.goto(savedSearchAddress(anasSearch));
  await openBar(page);
  await page.getByRole("button", { name: t("search.share") }).click();
  const window = page.getByRole("dialog", {
    name: t("search.share-title", { name: "Ana's list" }),
  });
  await expect(window.getByRole("checkbox", { name: "Website" })).toBeVisible();
  // A group needs the right to share for the whole organization; Ana has it for one project only,
  // so no group is offered, not even the one she is in.
  await expect(window.getByText(t("search.share-none-groups"))).toBeVisible();
  await expect(window.getByRole("checkbox", { name: "Crew of Ana" })).toHaveCount(0);
  // What the person may not share with is not in the window, and nothing of it is in the page.
  await expect(window.getByRole("checkbox", { name: "Secret plans" })).toHaveCount(0);
  await expect(window.getByRole("checkbox", { name: "Somebody else's crew" })).toHaveCount(0);
  await expect(page.getByText("Secret plans")).toHaveCount(0);
  await expect(page.getByText("Somebody else's crew")).toHaveCount(0);
  // The button that makes a project's list is for people who change the project, not for a team member.
  await window.getByText("Website", { exact: true }).click();
  await expect(window.getByRole("checkbox", { name: "Website", checked: true })).toBeVisible();
  await expect(
    window.getByRole("button", { name: t("search.project-list-use", { name: "Website" }) }),
  ).toHaveCount(0);
});

test("qry_06 a person who did not make the search is not offered Share", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(savedSearchAddress(anasSearch));
  // Ada does not own Ana's search: it is not hers to share, and she is not offered the window.
  await expect(page.getByRole("button", { name: t("search.share") })).toHaveCount(0);
});

test("qry_06 the title, the headings and the tick boxes of Share stand on one edge", async ({
  page,
}) => {
  await signIn(page, ana);
  await page.goto(savedSearchAddress(anasSearch));
  await openBar(page);
  await page.getByRole("button", { name: t("search.share") }).click();
  const window = page.getByRole("dialog", {
    name: t("search.share-title", { name: "Ana's list" }),
  });
  const title = await wordsStartOf(
    window.getByText(t("search.share-title", { name: "Ana's list" })),
  );
  const heading = await wordsStartOf(window.getByText(t("search.share-projects")));
  const box = await leftEdgeOf(window.locator("label").filter({ hasText: "Website" }));
  expect(Math.abs(box - heading)).toBeLessThanOrEqual(1);
  // On a phone the title is in a bar with *Back* before it, so only a computer compares it.
  await onThisLayout(page, {
    computer: () => {
      expect(Math.abs(heading - title)).toBeLessThanOrEqual(1);
      return Promise.resolve();
    },
    tablet: () => {
      expect(Math.abs(heading - title)).toBeLessThanOrEqual(1);
      return Promise.resolve();
    },
    phone: nothing,
  });
});
