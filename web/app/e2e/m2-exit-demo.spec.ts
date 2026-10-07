import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { paths, projectAddress, projectPageAddress } from "../src/shared/paths.ts";
import { around, levelList, pickLevel, roleName } from "./support/access.ts";
import {
  check,
  closeTheSheet,
  findsNoneOf,
  good,
  open,
  seesOnlyTheTreeOf,
  sentence,
} from "./support/demo.ts";
import { expect, test } from "./support/fixtures.ts";
import { openFromMenu, signIn } from "./support/flows.ts";
import { inOrder } from "./support/sequence.ts";
import {
  sampleAdministrator,
  sampleGuestIn,
  sampleNamed,
  sampleNames,
  sampleProjectName,
} from "./support/sample.ts";

// The exit demo of M2 (docs/04-milestones.md) on the sample organization, as the maintainer can repeat
// it by hand with `cargo xtask dev --sample`: an administrator makes a role, hides Cost center from
// Contributors and names from Guests on the Website, checks Ana's access and reads the
// explanation; then a Guest and a Contributor sign in and see only what is theirs. Every step has
// no accessibility problem in the four themes, on a phone and on a computer, and with
// `BB_SCREENSHOT_DIR` set the same run takes the pictures for the review page of the milestone
// (`<folder>/M2/demo/<step>-<phone|computer>-<light|dark>.png`).

test.use({ sample: true });
test.describe.configure({ mode: "serial" });

const costCenter = t("issue-field.cost-center");
const ana = sampleNamed("Ana Moreno");
const ben = sampleNamed("Ben Hale");
/** The role the administrator makes: a copy of Viewer that may also see how the server is doing. */
const NEW_ROLE = "Reviewer";

/** The roles of the sample are listed: the six starting roles and the two the sample made. */
async function theRolesOfTheSample(page: Page): Promise<void> {
  const list = page.getByRole("list", { name: t("roles.list"), exact: true });
  const names = [
    roleName.administrator,
    roleName.projectLead,
    roleName.teamMember,
    roleName.contributor,
    t("starting-role.viewer"),
    roleName.guest,
    sampleNames.roles["support-agent"] ?? "",
    sampleNames.roles["security-reviewer"] ?? "",
  ];
  await inOrder(
    names.map((name) => async () => {
      await expect(list.getByRole("link", { name, exact: true })).toBeVisible();
    }),
  );
}

/** Step 1: the administrator makes a role from a copy of Viewer and lets it see the server's state. */
async function makeARole(page: Page): Promise<void> {
  await openFromMenu(page, t("nav.roles"));
  await expect(page.getByRole("heading", { level: 1, name: t("nav.roles") })).toBeVisible();
  await theRolesOfTheSample(page);
  await page.getByLabel(t("roles.field-name"), { exact: true }).fill(NEW_ROLE);
  await page.getByLabel(t("roles.start-from"), { exact: true }).selectOption({
    label: t("roles.start-copy", { name: t("starting-role.viewer") }),
  });
  await page.getByRole("button", { name: t("roles.make"), exact: true }).click();
  await expect(page.getByRole("heading", { level: 1, name: NEW_ROLE })).toBeVisible();
  const status = page.getByRole("checkbox", { name: t("permission.see-status"), exact: true });
  await expect(status).not.toBeChecked();
  await page.getByText(t("permission.see-status"), { exact: true }).click();
  await expect(status).toBeChecked();
  await expect(page.getByText(t("save.saved"), { exact: true })).toBeVisible();
  await good(page, "01-create-role");
}

/** Step 2: Cost center is hidden from Contributors on the Website; the Mobile app had it hidden already. */
async function hideCostCenter(page: Page): Promise<void> {
  const mobileApp = sampleProjectName("APP");
  await open(
    page,
    projectPageAddress(paths.projectFields, "APP"),
    t("field-permissions.project-title", { project: mobileApp }),
  );
  const inApp = await levelList(page, costCenter, roleName.contributor);
  await expect(inApp).toHaveValue("hidden");
  await expect(around(inApp)).toContainText(t("access.origin-here"));

  const website = sampleProjectName("WEB");
  await open(
    page,
    projectPageAddress(paths.projectFields, "WEB"),
    t("field-permissions.project-title", { project: website }),
  );
  const inWebsite = await levelList(page, costCenter, roleName.contributor);
  await expect(inWebsite).toHaveValue("can-see");
  await pickLevel(inWebsite, page, t("field-level.hidden"));
  await expect(around(inWebsite)).toContainText(t("access.origin-here"));
  await good(page, "02-hide-cost-center");

  // The sub-projects of the Website have it hidden as well, and say where that comes from.
  await open(
    page,
    projectPageAddress(paths.projectFields, "SHOP"),
    t("field-permissions.project-title", { project: sampleProjectName("SHOP") }),
  );
  const inShop = await levelList(page, costCenter, roleName.contributor);
  await expect(inShop).toHaveValue("hidden");
  await expect(around(inShop)).toContainText(t("access.origin-from", { project: website }));
}

/** Step 3: Guests no longer see who is assigned on the Website: they read "Someone". */
async function hideNames(page: Page): Promise<void> {
  await open(
    page,
    projectPageAddress(paths.projectPeople, "WEB"),
    t("people-visibility.project-title", { project: sampleProjectName("WEB") }),
  );
  const assigned = await levelList(page, t("people-kind.assigned-to"), roleName.guest);
  // A Guest sees who is assigned by role only, as the starting rules say.
  await expect(assigned).toHaveValue("role-only");
  await pickLevel(assigned, page, t("people-level.hidden"));
  const who = t("projects.person-someone");
  await expect(sentence(page, t("people-visibility.example-assigned", { who }))).toBeVisible();
  await good(page, "03-hide-names");
}

/** Step 4: Ana's access on the Website, with the reasons, and what is hidden from her now. */
async function checkAnasAccess(page: Page): Promise<void> {
  const website = sampleProjectName("WEB");
  await open(
    page,
    projectPageAddress(paths.projectCheckAccess, "WEB"),
    t("check-access.project-title", { project: website }),
  );
  await check(page, null, ana);
  await expect(
    page.getByRole("heading", {
      name: t("check-access.result-heading", { person: ana.name, place: website }),
    }),
  ).toBeVisible();
  // She is a Contributor because the group External has the role on the Website.
  const group = sampleNames.groups["external"] ?? "";
  const reason = t("check-access.reason.role-group-project", {
    person: ana.name,
    group,
    role: roleName.contributor,
    project: website,
  });
  await expect(sentence(page, reason)).toBeVisible();
  // Cost center is hidden from her now, and the answer says which role sets that and where.
  await page.getByRole("button", { name: t("check-access.show-every-field") }).click();
  const level = t("check-access.level.role-project", {
    role: roleName.contributor,
    level: t("field-level.hidden"),
    project: website,
  });
  await expect(sentence(page, level)).toBeVisible();
  await good(page, "04-check-access");
}

/** Step 5: a person with No access in a sub-project reads that it wins over their role. */
async function readNoAccess(page: Page): Promise<void> {
  await open(
    page,
    projectPageAddress(paths.projectCheckAccess, "AND"),
    t("check-access.project-title", { project: sampleProjectName("AND") }),
  );
  await check(page, null, ben);
  const blocked = t("check-access.reason.blocked-person", {
    person: ben.name,
    project: sampleProjectName("AND"),
  });
  await expect(sentence(page, blocked)).toBeVisible();
  await good(page, "05-no-access");
}

test("the exit demo: an administrator makes a role, hides what Contributors and Guests may not see, and checks Ana's access", async ({
  page,
}) => {
  test.setTimeout(240_000);
  await signIn(page, sampleAdministrator());
  await test.step("1. The administrator makes a role from a copy of Viewer", () => makeARole(page));
  await test.step("2. The administrator hides Cost center from Contributors on the Website", () =>
    hideCostCenter(page));
  await test.step("3. The administrator hides names from Guests on the Website", () =>
    hideNames(page));
  await test.step("4. The administrator checks Ana's access on the Website and reads why", () =>
    checkAnasAccess(page));
  await test.step("5. No access wins over the role", () => readNoAccess(page));
});

/** Signs in as the person and looks: the Website is theirs, nothing else is there for them. */
async function seesOnlyTheWebsite(
  page: Page,
  person: ReturnType<typeof sampleNamed>,
  step: string,
): Promise<void> {
  await signIn(page, person);
  await open(page, projectAddress("WEB"), sampleProjectName("WEB"));
  await seesOnlyTheTreeOf(page, "WEB");
  await good(page, step);
  await closeTheSheet(page);
  await findsNoneOf(page, "WEB", "APP");
}

test("the exit demo: a Guest and a Contributor sign in and see only what is theirs", async ({
  page,
}) => {
  test.setTimeout(120_000);
  await test.step("A Guest on the Website sees the Website", () =>
    seesOnlyTheWebsite(page, sampleGuestIn("WEB"), "06-guest-sees"));
  await test.step("Ana, a Contributor through the group External, sees the Website", async () => {
    await page.context().clearCookies();
    await seesOnlyTheWebsite(page, ana, "07-contributor-sees");
  });
});
