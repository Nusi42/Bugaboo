import { t } from "@bugaboo/messages";

import {
  groupAddress,
  personAddress,
  projectRolesAddress,
  roleAddress,
} from "../src/shared/paths.ts";
import { ada, bob, cleo, makeAdmin, makePerson } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { makeProject } from "./support/projects.ts";
import { screenshotFolder, shoot } from "./support/screenshots.ts";

// The pages of People, groups and roles for the maintainer's review (M2 WP-11), at phone width, in a
// light and a dark look. They are only taken when `BB_SCREENSHOT_DIR` names the folder:
//   BB_SCREENSHOT_DIR=... npx playwright test screenshots-people.spec --project=phone
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
/** Tall, so that the page, which scrolls inside the frame of the app, is whole in the picture. */
test.use({ viewport: { width: 400, height: 1700 }, deviceScaleFactor: 2 });

const MILESTONE = "M2";
let bobId = "";
let groupId = "";
let roleId = "";

test.use({
  preparation: {
    run: async (server) => {
      const admin = await makeAdmin(server);
      const bobSession = await makePerson(server, admin, bob);
      bobId = String((await bobSession.get("/api/v1/auth/me"))["id"]);
      await bobSession.dispose();
      await (await makePerson(server, admin, cleo)).dispose();
      const acme = await makeProject(admin, "ACME", "Acme");
      await makeProject(admin, "WEB", "Website", acme);
      groupId = String((await admin.post("/api/v1/groups", { name: "Backend" }))["id"]);
      await admin.post("/api/v1/groups", { name: "Design" });
      await admin.put(`/api/v1/groups/${groupId}/members/${bobId}`, {});
      const listed = (await admin.get("/api/v1/roles"))["items"];
      const byName = new Map<string, string>();
      for (const item of Array.isArray(listed) ? listed : []) {
        byName.set(String(Reflect.get(item, "name")), String(Reflect.get(item, "id")));
      }
      roleId = byName.get("Team member") ?? "";
      // The group has Team member on Acme and its sub-projects; on Website it has No access.
      await admin.post(`/api/v1/projects/${acme}/assignments`, {
        who: { group: groupId },
        role: roleId,
        applies_to: "project-and-sub-projects",
      });
      await admin.post(`/api/v1/projects/${acme}/assignments`, {
        who: { person: bobId },
        role: byName.get("Viewer"),
        applies_to: "project",
      });
      await admin.dispose();
    },
  },
});

test("the pages of people, groups and roles", async ({ page }) => {
  test.setTimeout(300_000);
  await signIn(page, ada);

  await page.goto("/settings/people");
  await expect(page.getByRole("heading", { level: 1, name: t("nav.people") })).toBeVisible();
  await expect(page.getByRole("option", { name: /^Bob Builder/u })).toBeVisible();
  await shoot(page, "50-people", MILESTONE);

  await page.goto(personAddress(bobId));
  await expect(page.getByRole("heading", { level: 1, name: bob.name })).toBeVisible();
  await shoot(page, "51-person", MILESTONE);

  await page.goto(groupAddress(groupId));
  await expect(page.getByRole("heading", { level: 2, name: "Backend" })).toBeVisible();
  await shoot(page, "52-groups", MILESTONE);

  await page.goto(roleAddress(roleId));
  await expect(page.getByRole("heading", { level: 1, name: "Team member" })).toBeVisible();
  await shoot(page, "53-role", MILESTONE);

  // Website: the rows of Acme come down to it, and the group gets No access here.
  await page.goto(projectRolesAddress("WEB"));
  const rows = page.getByRole("list", { name: t("assignments.list"), exact: true });
  await expect(rows.getByRole("listitem").first()).toBeVisible();
  await page.getByRole("radio", { name: t("assignments.kind-group"), exact: true }).check();
  await page.getByRole("radio", { name: "Backend", exact: true }).check();
  await page.getByLabel(t("assignments.role"), { exact: true }).selectOption({
    label: t("roles.no-access"),
  });
  await page.getByRole("button", { name: t("assignments.give-no-access"), exact: true }).click();
  await expect(rows.getByRole("listitem").first()).toContainText(t("roles.no-access"));
  await expect(rows.getByText(t("roles.from-project", { name: "Acme" })).first()).toBeVisible();
  await shoot(page, "54-assignments", MILESTONE);
});
