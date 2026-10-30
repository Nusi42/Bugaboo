import { t } from "@bugaboo/messages";

import { addressInProject, paths } from "../src/shared/paths.ts";
import { check } from "./support/access.ts";
import { ada, bob, cleo } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { openFromMenu, signIn, signOut } from "./support/flows.ts";
import { itemArea } from "./support/layout.ts";
import { previewMoveToLab } from "./support/offers-look.ts";
import { offerSwitch, prepareOffers, switchOff, typesOffered } from "./support/offers.ts";

// Issue types and groups of labels per project, and Check access per issue type (ISS-02, TAG-02,
// SEC-03, M4 WP-11): a Project lead limits what their project offers, an administrator limits a
// type to one project, a project that does not offer a type does not offer it in New issue, and
// Check access asks which issue type only when the levels differ by type.

test.use({ preparation: { run: prepareOffers } });
test.describe.configure({ mode: "serial" });

test("iss_02 a Project lead limits Website to Bug, and Lab still offers every type", async ({
  page,
}) => {
  await signIn(page, cleo);
  await page.goto(addressInProject(paths.projectIssueTypes, "WEB"));
  await expect(
    page.getByRole("heading", {
      level: 1,
      name: t("issue-types.project-title", { project: "Website" }),
    }),
  ).toBeVisible();
  const listName = t("issue-types.project-list");
  await switchOff(page, listName, "Task");
  await switchOff(page, listName, "Feature");
  await switchOff(page, listName, "Goal");
  await expect(offerSwitch(page, listName, "Bug")).toBeChecked();
  await expectGoodInEveryLook(page);
  // New issue in Website offers Bug only; Lab is not there for the lead.
  expect(await typesOffered(page, "WEB")).toEqual(["Bug"]);
  await page.goto(addressInProject(paths.projectIssueTypes, "LAB"));
  await expect(
    page.getByRole("heading", { level: 1, name: t("page.not-found.title") }),
  ).toBeVisible();
  await signOut(page);
  await signIn(page, ada);
  expect(await typesOffered(page, "LAB")).toEqual(["Bug", "Task", "Feature", "Goal"]);
});

test("iss_02 an administrator limits Bug to Website, and New issue in Lab does not offer it", async ({
  page,
}) => {
  await signIn(page, ada);
  await openFromMenu(page, t("nav.issue-types"));
  await page.getByRole("button", { name: "Bug", exact: true }).click();
  const panel = itemArea(page, "Bug");
  const offered = panel.getByRole("region", { name: t("projects.offered-in-title") });
  await expect(offered.getByText(t("projects.offered-in-everywhere"))).toBeVisible();
  // The whole organization does not offer Bug; Website does.
  await offered
    .getByRole("radio", { name: t("projects.offer-whole-organization"), exact: true })
    .click();
  await offered.getByLabel(t("projects.offered-in-state"), { exact: true }).selectOption({
    label: t("projects.offer-state-not"),
  });
  await offered.getByRole("button", { name: t("projects.offered-in-save"), exact: true }).click();
  await expect(
    offered.getByText(
      t("projects.offer-line", {
        name: t("projects.offer-whole-organization"),
        state: t("projects.offer-state-not"),
      }),
    ),
  ).toBeVisible();
  await offered.getByLabel(t("projects.offered-in-pick-search"), { exact: true }).fill("Website");
  await offered.getByRole("radio", { name: /Website/u }).click();
  await offered.getByRole("button", { name: t("projects.offered-in-save"), exact: true }).click();
  await expect(
    offered.getByText(
      t("projects.offer-line", { name: "Website", state: t("projects.offer-state-offered") }),
    ),
  ).toBeVisible();
  await expectGoodInEveryLook(page);
  expect(await typesOffered(page, "LAB")).not.toContain("Bug");
  expect(await typesOffered(page, "WEB")).toContain("Bug");
});

test("iss_02 moving an issue to a project that does not offer its type names the type", async ({
  page,
}) => {
  await signIn(page, ada);
  // The issue is not skipped: it keeps its type, and the preview says that Lab does not use it.
  const dialog = await previewMoveToLab(page);
  await expect(dialog.getByText(t("preview.summary", { changed: 1, skipped: 0 }))).toBeVisible();
  await expectGoodInEveryLook(page);
});

test("tag_02 a Project lead creates a group of labels for Website, and Lab does not offer it", async ({
  page,
}) => {
  await signIn(page, cleo);
  await page.goto(addressInProject(paths.projectLabels, "WEB"));
  await expect(
    page.getByRole("heading", {
      level: 1,
      name: t("labels.project-title", { project: "Website" }),
    }),
  ).toBeVisible();
  await page.getByLabel(t("labels.field-group-name"), { exact: true }).fill("Area");
  await page.getByRole("button", { name: t("labels.create-label-group"), exact: true }).click();
  await expect(page.getByText(t("labels.group-created", { name: "Area" }))).toBeVisible();
  const listName = t("labels.project-list");
  await expect(offerSwitch(page, listName, "Area")).toBeChecked();
  await expectGoodInEveryLook(page);
  await signOut(page);
  await signIn(page, ada);
  await page.goto(addressInProject(paths.projectLabels, "LAB"));
  // Lab is no place the group can reach: the screen of Lab does not list it at all.
  await expect(
    page.getByRole("heading", { level: 1, name: t("labels.project-title", { project: "Lab" }) }),
  ).toBeVisible();
  await expect(page.getByText("Area", { exact: true })).toHaveCount(0);
});

test("sec_03 Check access asks which issue type only when the levels differ by type", async ({
  page,
}) => {
  await signIn(page, ada);
  await openFromMenu(page, t("nav.check-access"));
  await check(page, "WEB", bob);
  const question = page.getByLabel(t("check-access.issue-type-question"), { exact: true });
  await expect(question).toBeVisible();
  await question.selectOption({ label: "Bug" });
  await expect(page.getByText(t("check-access.for-issue-type", { type: "Bug" }))).toBeVisible();
  await expectGoodInEveryLook(page);
  // A person whose levels are the same for every type is asked nothing more.
  await check(page, "WEB", cleo);
  const heading = t("check-access.result-heading", {
    person: cleo.name,
    place: "Website",
  });
  await expect(page.getByText(heading)).toBeVisible();
  await expect(question).toHaveCount(0);
});
