import { dataOf, type Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { percentText } from "../src/shared/number.ts";
import { addressInProject, paths } from "../src/shared/paths.ts";
import { giveRole, projectIdOf, roleName } from "./support/access.ts";
import { ada, bob, cleo, signInThroughApi } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn, signOut } from "./support/flows.ts";
import { idOf, prepareWorkTimes, setShareOf } from "./support/work-times.ts";

// *Project settings › Project members* (PLAN-15, ADR-0030, UX §4.13): a Project lead sets a share in
// place with Undo, and the mark of a sum over 100 % names only the projects the reader may see.

test.use({ preparation: { run: prepareWorkTimes } });

/** The share of a person in WEB, as the server has it. */
async function shareInWeb(server: Parameters<typeof idOf>[0], person: string): Promise<number> {
  const session = await signInThroughApi(server, ada);
  const page: Schema<"Page_MemberShareBody"> = dataOf(
    await session.client.GET("/api/v1/projects/{project}/shares", {
      params: { path: { project: await projectIdOf(session, "WEB") } },
    }),
  );
  await session.dispose();
  return (
    page.items.find((item) => item.person.shown === "name" && item.person.id === person)?.percent ??
    -1
  );
}

test("plan_15_a_project_lead_sets_a_share_on_project_members_with_undo", async ({
  page,
  server,
}) => {
  const bobId = await idOf(server, bob);
  await signIn(page, cleo);
  await page.goto(addressInProject(paths.projectMembers, "WEB"));
  await expect(
    page.getByRole("heading", { level: 1, name: t("nav.project-members") }),
  ).toBeVisible();
  const box = page.getByRole("textbox", {
    name: t("project-members.share-of", { name: bob.name }),
    exact: true,
  });
  await expect(box).toHaveValue("100");
  await box.fill("60");
  await box.press("Enter");
  await expect(
    page.getByText(t("project-members.share-set", { name: bob.name, percent: percentText(60) })),
  ).toBeVisible();
  await expect.poll(() => shareInWeb(server, bobId)).toBe(60);
  await page
    .getByRole("button", { name: t("common.undo"), exact: true })
    .last()
    .click();
  await expect.poll(() => shareInWeb(server, bobId)).toBe(100);
  await expect(box).toHaveValue("100");
});

test("plan_15_the_sum_over_100_percent_names_only_projects_the_reader_sees", async ({
  page,
  server,
}) => {
  const bobId = await idOf(server, bob);
  const admin = await signInThroughApi(server, ada);
  await giveRole(admin, bobId, roleName.teamMember, "OTHER");
  await admin.dispose();
  await setShareOf(server, ada, "OTHER", bobId, 70);
  await setShareOf(server, ada, "WEB", bobId, 60);
  // Cleo leads WEB and has no role in OTHER: she sees the mark, never the name of OTHER.
  await signIn(page, cleo);
  await page.goto(addressInProject(paths.projectMembers, "WEB"));
  const row = page.getByRole("listitem").filter({ hasText: bob.name });
  await expect(row.getByText(t("project-members.over"))).toBeVisible();
  await expect(
    row.getByText(t("project-members.also-in", { projects: t("project-members.others-hidden") })),
  ).toBeVisible();
  await expect(page.getByText("Other work")).toHaveCount(0);
  // The administrator sees OTHER, so it is named.
  await signOut(page);
  await signIn(page, ada);
  await page.goto(addressInProject(paths.projectMembers, "WEB"));
  await expect(
    page
      .getByRole("listitem")
      .filter({ hasText: bob.name })
      .getByText(t("project-members.also-in", { projects: "Other work" })),
  ).toBeVisible();
});
