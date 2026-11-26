import { t } from "@bugaboo/messages";
import { expect } from "@playwright/test";

import { projectIssuesAddress } from "../src/shared/paths.ts";
import { ada, bob, signInThroughApi } from "./support/accounts.ts";
import {
  createBug,
  createTask,
  dashboardAddress,
  readyMadeBox,
} from "./support/dashboard-issues.ts";
import { boxOnScreen, createDashboard, searchOf, showAsTable } from "./support/dashboards.ts";
import { test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { aaron, createPeopleWithRoles, idInPeople, roleName } from "./support/dashboard-people.ts";

// The charts of dashboards (CFG-03, M10-Q13, QRY-08): every chart has its table, a chart by a person stands in the
// order of the names and never of size, and *Open bugs by priority* says what the grouped list says.

test.use({ preparation: { run: prepareWeb } });

test("cfg_03_every_chart_has_a_table_and_a_person_chart_is_in_the_order_of_names", async ({
  page,
  server,
}) => {
  const admin = await signInThroughApi(server, ada);
  // Aaron is created last but sorts first by name; Bob has the most issues, so neither the order of creation nor the
  // order of size is the order of names.
  await createPeopleWithRoles(server, admin, [
    { person: aaron, role: roleName.teamMember, project: "WEB" },
  ]);
  const [aaronId, adaId, bobId] = [
    await idInPeople(admin, aaron),
    await idInPeople(admin, ada),
    await idInPeople(admin, bob),
  ];
  await createTask(admin, "Chart one", { assigned_to: [bobId] });
  await createTask(admin, "Chart two", { assigned_to: [bobId] });
  await createTask(admin, "Chart three", { assigned_to: [adaId] });
  await createTask(admin, "Chart four", { assigned_to: [aaronId] });
  const title = "Who works on charts";
  const made = await createDashboard(admin, "Charts", [
    {
      kind: "numbers-and-chart",
      title,
      search_text: "title contains Chart",
      by: "assigned to",
      shape: "bars",
      column: 0,
      row: 0,
      width: 12,
      height: 4,
    },
  ]);
  await admin.dispose();
  await signIn(page, ada);
  await page.goto(dashboardAddress(made.id));
  const box = boxOnScreen(page, title);
  // The table is what screen readers read, while the bars are shown.
  await expect(box.getByRole("table", { name: title })).toHaveCount(1);
  await showAsTable(box);
  await expect(box.getByRole("table", { name: title })).toBeVisible();
  await expect(box.getByRole("rowheader")).toHaveText([aaron.name, ada.name, bob.name]);
  await expect(box.getByRole("cell")).toHaveText([/ 1$/u, / 1$/u, / 2$/u]);
});

test("qry_08_open_bugs_by_priority_matches_the_grouped_list", async ({ page, server }) => {
  const admin = await signInThroughApi(server, ada);
  await createBug(admin, "Broken header", "high");
  await createBug(admin, "Broken footer", "high");
  await createBug(admin, "Broken link", "low");
  const asked = await readyMadeBox(admin, "open-bugs-by-priority");
  const made = await createDashboard(admin, "Bugs", [asked]);
  await admin.dispose();
  await signIn(page, ada);
  await page.goto(dashboardAddress(made.id));
  const box = boxOnScreen(page, asked.title);
  await showAsTable(box);
  await expect(box.getByRole("rowheader")).toHaveText([t("priority.high"), t("priority.low")]);
  await expect(box.getByRole("cell")).toHaveText([/ 2$/u, / 1$/u]);
  // The grouped list of the same search says the same counts for the same groups.
  await page.goto(`${projectIssuesAddress("WEB")}?s=${searchOf(asked)}&group=priority`);
  await expect(
    page.getByRole("row", {
      name: t("issues.group-open", { group: t("priority.high"), count: 2 }),
    }),
  ).toBeVisible();
  await expect(
    page.getByRole("row", {
      name: t("issues.group-open", { group: t("priority.low"), count: 1 }),
    }),
  ).toBeVisible();
});
