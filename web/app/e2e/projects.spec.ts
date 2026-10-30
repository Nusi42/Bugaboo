import { t } from "@bugaboo/messages";

import { ada, createAdmin } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { addressEndingAt, openFromMenu, signIn } from "./support/flows.ts";
import { itemArea } from "./support/layout.ts";
import { fillCreate, openOverview, tree, treeLink } from "./support/projects.ts";
import { addressInProject, paths, projectAddress } from "../src/shared/paths.ts";

// Projects (PRJ-01, PRJ-02): making projects and sub-projects, the tree of the sidebar (the *More*
// sheet on a phone), the settings page of the organization, the overview and the settings of a
// project, and the short names that stay with a project.

test.use({
  preparation: {
    run: async (server) => {
      await (await createAdmin(server)).dispose();
    },
  },
});
test.describe.configure({ mode: "serial" });

test("prj_01 an administrator makes a project in the organization settings and finds it in the tree", async ({
  page,
}) => {
  await signIn(page, ada);
  await openFromMenu(page, t("nav.projects"));
  await expect(page.getByRole("heading", { level: 1, name: t("projects.title") })).toBeVisible();
  await expect(page.getByText(t("projects.none-yet"))).toBeVisible();
  // The short name is typed in small letters; the form turns them into capitals.
  await fillCreate(page, "Website", "web");
  await expect(page.getByLabel(t("projects.field-short-name"), { exact: true })).toHaveValue("");
  // The list holds the project, which opens in the panel next to it.
  await page.getByRole("button", { name: "Website", exact: true }).click();
  await expect(itemArea(page, "Website")).toBeVisible();
  await expect(page.getByRole("link", { name: t("projects.open") })).toHaveAttribute(
    "href",
    projectAddress("WEB"),
  );
  await expect(await treeLink(page, "Website")).toBeVisible();
});

test("prj_01 a project is opened from the tree and gets a sub-project", async ({ page }) => {
  await signIn(page, ada);
  await (await treeLink(page, "Website")).click();
  await expect(page).toHaveURL(addressEndingAt(projectAddress("WEB")));
  await expect(page.getByRole("heading", { level: 1, name: "Website" })).toBeVisible();
  await expect(page.getByText(t("projects.sub-projects-empty"))).toBeVisible();
  // The one next step of an empty project: make a sub-project.
  await page.getByRole("button", { name: t("projects.create-sub-project") }).click();
  await fillCreate(page, "Documents", "docs");
  const subProjects = page.getByRole("list", { name: t("projects.sub-projects") });
  await expect(subProjects.getByRole("link", { name: "Documents" })).toBeVisible();
});

test("prj_01 the tree opens to the sub-projects and remembers what was open", async ({ page }) => {
  await signIn(page, ada);
  const region = await tree(page);
  await region
    .getByRole("button", { name: t("projects.toggle-name", { name: "Website" }) })
    .click();
  await expect(region.getByRole("link", { name: /^Documents/u })).toBeVisible();
  await page.reload();
  const again = await tree(page);
  await expect(again.getByRole("link", { name: /^Documents/u })).toBeVisible();
  await expect(
    again.getByRole("button", { name: t("projects.toggle-name", { name: "Website" }) }),
  ).toHaveAttribute("aria-expanded", "true");
});

test("prj_01 the overview and the settings of a project are good in every theme", async ({
  page,
}) => {
  await signIn(page, ada);
  await openOverview(page, projectAddress("WEB"), "Website");
  await expectGoodInEveryLook(page);
  await openOverview(
    page,
    addressInProject(paths.projectSettings, "WEB"),
    t("projects.settings-title", { name: "Website" }),
  );
  await expectGoodInEveryLook(page);
});

test("prj_01 an old short name opens the project and the address becomes the new one", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(addressInProject(paths.projectSettings, "WEB"));
  // The page has one form for the project and one for its settings (the biggest file, M4 WP-05),
  // each with its own Save: this one is the form of the short name.
  const shortName = page.getByLabel(t("projects.field-short-name"), { exact: true });
  await shortName.fill("site");
  await page
    .locator("form")
    .filter({ has: shortName })
    .getByRole("button", { name: t("common.save"), exact: true })
    .click();
  await expect(page.getByText(t("save.saved"), { exact: true })).toBeVisible();
  await expect(page).toHaveURL(addressEndingAt(addressInProject(paths.projectSettings, "SITE")));
  // The old address still works and is replaced by the new one.
  await page.goto(projectAddress("WEB"));
  await expect(page).toHaveURL(addressEndingAt(projectAddress("SITE")));
  await expect(page.getByRole("heading", { level: 1, name: "Website" })).toBeVisible();
});

test("prj_01 a short name that is taken is refused with a sentence", async ({ page }) => {
  await signIn(page, ada);
  await openFromMenu(page, t("nav.projects"));
  await page.getByLabel(t("projects.field-name"), { exact: true }).fill("Other");
  // WEB was the short name of the Website: nobody else can take it.
  await page.getByLabel(t("projects.field-short-name"), { exact: true }).fill("web");
  await page.getByRole("button", { name: t("projects.create"), exact: true }).click();
  await expect(page.getByRole("alert")).toHaveText(t("error.name.taken"));
});

test("prj_01 quick search finds a project by its name", async ({ page }) => {
  await signIn(page, ada);
  await page.keyboard.press("Control+k");
  await page.getByRole("searchbox", { name: t("search.label"), exact: true }).fill("docu");
  await page.getByRole("option", { name: /Documents/u }).click();
  await expect(page).toHaveURL(addressEndingAt(projectAddress("DOCS")));
});
