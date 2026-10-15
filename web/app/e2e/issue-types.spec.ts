import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { ada, bob, makeAdmin, makePerson } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { openFromMenu, signIn } from "./support/flows.ts";
import { makeProject } from "./support/projects.ts";
import { addressInProject, paths } from "../src/shared/paths.ts";

// Issue types and the mode of issue IDs (ISS-02, ISS-01a, WP-03): the four types of the first
// start in a searchable list with a side panel, making one, changing it and retiring it, and the
// choice between counting and random IDs on the settings of a project.

test.use({
  preparation: {
    run: async (server) => {
      const admin = await makeAdmin(server);
      await (await makePerson(server, admin, bob)).dispose();
      await makeProject(admin, "WEB", "Website");
      await admin.dispose();
    },
  },
});
test.describe.configure({ mode: "serial" });

/** The list of issue types on the page. */
function typeList(page: Page) {
  return page.getByRole("list", { name: t("issue-types.search"), exact: true });
}

test("iss_02 an administrator sees the four types of the first start and opens one", async ({
  page,
}) => {
  await signIn(page, ada);
  await openFromMenu(page, t("nav.issue-types"));
  await expect(page.getByRole("heading", { level: 1, name: t("nav.issue-types") })).toBeVisible();
  const list = typeList(page);
  await expect(list.getByRole("listitem")).toHaveCount(4);
  await expect(list.getByRole("button")).toHaveText(["Bug", "Task", "Feature", "Goal"]);
  await page.getByRole("searchbox", { name: t("issue-types.search"), exact: true }).fill("fea");
  await expect(list.getByRole("listitem")).toHaveCount(1);
  await list.getByRole("button", { name: "Feature", exact: true }).click();
  const panel = page.getByRole("complementary", { name: "Feature" });
  await expect(panel).toBeVisible();
  await expect(panel.getByLabel(t("issue-types.field-name"), { exact: true })).toHaveValue(
    "Feature",
  );
});

test("iss_02 a type is renamed, given another icon and kept after the page is read again", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(paths.issueTypes);
  await page.getByRole("button", { name: "Task", exact: true }).click();
  const panel = page.getByRole("complementary", { name: "Task" });
  await panel.getByLabel(t("issue-types.field-name"), { exact: true }).fill("Chore");
  await panel.getByLabel(t("issue-types.field-icon"), { exact: true }).selectOption({
    label: t("issue-icon.wrench"),
  });
  await panel.getByRole("button", { name: t("common.save"), exact: true }).click();
  // The panel is named after the type, so it has the new name now.
  await expect(
    page.getByRole("complementary", { name: "Chore" }).getByText(t("save.saved")),
  ).toBeVisible();
  // The list is read again from the server, with the panel closed (a phone shows one thing at a
  // time).
  await page.goto(paths.issueTypes);
  await expect(typeList(page).getByRole("button", { name: "Chore", exact: true })).toBeVisible();
  await expect(typeList(page).getByRole("button", { name: "Task", exact: true })).toHaveCount(0);
});

test("iss_02 a type is made, retired and offered again", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(paths.issueTypes);
  await page.getByLabel(t("issue-types.field-name"), { exact: true }).fill("Question");
  await page.getByLabel(t("issue-types.field-icon"), { exact: true }).selectOption({
    label: t("issue-icon.circle-help"),
  });
  await page.getByRole("button", { name: t("issue-types.make"), exact: true }).click();
  await expect(page.getByText(t("issue-types.made", { name: "Question" }))).toBeVisible();
  const list = typeList(page);
  await expect(list.getByRole("listitem")).toHaveCount(5);
  // The same name is taken, whatever the capitals.
  await page.getByLabel(t("issue-types.field-name"), { exact: true }).fill("question");
  await page.getByRole("button", { name: t("issue-types.make"), exact: true }).click();
  await expect(page.getByText(t("error.name.taken"))).toBeVisible();
  await list.getByRole("button", { name: "Question", exact: true }).click();
  const panel = page.getByRole("complementary", { name: "Question" });
  // The switch is pressed by its words, as a person does; the words change with the state.
  await panel.getByText(t("issue-types.retire"), { exact: true }).click();
  await expect(
    panel.getByRole("switch", { name: t("issue-types.offer-again"), exact: true }),
  ).toBeChecked();
  // With the panel closed, the list says that the type is retired.
  await page.goto(paths.issueTypes);
  await expect(list.getByText(t("issue-types.retired"))).toBeVisible();
  await list.getByRole("button", { name: "Question", exact: true }).click();
  await page.getByText(t("issue-types.offer-again"), { exact: true }).click();
  await expect(
    page.getByRole("switch", { name: t("issue-types.retire"), exact: true }),
  ).not.toBeChecked();
  await page.goto(paths.issueTypes);
  await expect(list.getByText(t("issue-types.retired"))).toHaveCount(0);
});

test("iss_02 Issue types is good in every theme, also with a type open", async ({ page }) => {
  await signIn(page, ada);
  await openFromMenu(page, t("nav.issue-types"));
  await expect(typeList(page).getByRole("listitem")).not.toHaveCount(0);
  await expectGoodInEveryLook(page);
  await typeList(page).getByRole("button", { name: "Bug", exact: true }).click();
  await expect(page.getByRole("complementary", { name: "Bug" })).toBeVisible();
  await expectGoodInEveryLook(page);
});

test("iss_02 a person who may not manage issue types does not find the page", async ({ page }) => {
  await signIn(page, bob);
  await page.goto(paths.issueTypes);
  await expect(
    page.getByRole("heading", { level: 1, name: t("page.not-found.title") }),
  ).toBeVisible();
});

test("iss_01a the mode of issue IDs is a setting of the project, and it is kept", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(addressInProject(paths.projectSettings, "WEB"));
  const choice = page.getByRole("radio", { name: t("issue-id-mode.counting"), exact: true });
  // The organization starts as Random.
  await expect(
    page.getByRole("radio", { name: t("issue-id-mode.random"), exact: true }),
  ).toBeChecked();
  // The form shows the choice once the change is on its way, so the click is not a `check`.
  await choice.click();
  await expect(choice).toBeChecked();
  // The reload must come after the server kept the choice, not only after the form shows it.
  await expect(page.getByRole("status").filter({ hasText: t("save.saved") })).toBeVisible();
  await page.reload();
  await expect(choice).toBeChecked();
  await expectGoodInEveryLook(page);
});
