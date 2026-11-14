import { t } from "@bugaboo/messages";

import { projectIdOf, roleId, roleName, setFieldLevel, signedInAdmin } from "./support/access.ts";
import { bob } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { openIssueWithDetails, signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { issuePageLandmarks } from "./support/landmarks.ts";
import { expectStill, movementWhile } from "./support/movement.ts";
import { partIn } from "./support/parts.ts";

// The line of a status the person may not change (UX §4.4, §6.4, M8 WP-13): it holds the button,
// the reason and *Saved* after a change of the title, and *Saved* coming in moves nothing under it,
// on a phone too, where the line is short.

test.use({
  preparation: {
    run: async (server) => {
      await prepareWeb(server);
      // A Team Member of the Website may change the title but only see the status.
      const admin = await signedInAdmin(server);
      await setFieldLevel(
        admin,
        {
          role: await roleId(admin, roleName.teamMember),
          field: "status",
          project: await projectIdOf(admin, "WEB"),
        },
        "can-see",
      );
      await admin.dispose();
    },
  },
});

test("ux_01 a title changed beside a locked status says Saved in its line and moves nothing", async ({
  page,
}) => {
  await signIn(page, bob);
  await openIssueWithDetails(page, "WEB-1");
  await page.waitForLoadState("networkidle");
  const line = partIn(page, "issue-status-line");
  await expect(
    line.getByText(t("issues.read-only", { field: t("issue-field.status") })),
  ).toBeVisible();
  const title = partIn(page, "issue-title");
  await title.getByRole("button").click();
  await title.getByRole("textbox").fill("A title beside a locked status");
  const around = await issuePageLandmarks(page, title);
  const saving = await movementWhile(page, around.all, async () => {
    await title.getByRole("textbox").press("Enter");
    await expect(line.getByText(new RegExp(`^${t("save.saved")}`, "u"))).toBeVisible();
  });
  expectStill(saving, around);
});
